// SPDX-License-Identifier: AGPL-3.0-or-later
//! In-memory session registry. Synchronous on purpose: every method takes the
//! lock, mutates state and pushes messages with `try_send`, so it is trivial to
//! unit-test without a runtime. Grace-period timers live in the WebSocket layer.

use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;
use std::time::Duration;

use rand::Rng;
use serde_json::Value;
use sharu_proto::{ErrorCode, HostStatus, IceServer, PeerId, ServerMsg, ViewerInfo, HOST_ID};
use tokio::sync::mpsc;

use crate::{codes, nick};

pub type Tx = mpsc::Sender<ServerMsg>;

#[derive(Clone, Debug)]
pub struct HubConfig {
    pub max_viewers: usize,
    pub max_sessions: usize,
    pub code_words: usize,
    pub host_grace: Duration,
    pub leave_grace: Duration,
    pub ice_servers: Vec<IceServer>,
}

/// How the host connection went away.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disconnect {
    /// Goodbye message or WebSocket close frame (tab closed / page refreshed).
    Left,
    /// Socket died without a goodbye (network loss, crash).
    Lost,
}

struct Viewer {
    nickname: String,
    tx: Tx,
}

struct Session {
    code: String,
    host_token: String,
    host_conn: u64,
    host_tx: Option<Tx>,
    status: HostStatus,
    epoch: u64,
    passphrase: Option<String>,
    has_audio: bool,
    next_peer: PeerId,
    viewers: BTreeMap<PeerId, Viewer>,
}

fn push(tx: &Tx, msg: ServerMsg) {
    // A full queue means a stalled client; dropping is better than blocking the hub.
    let _ = tx.try_send(msg);
}

fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

impl Session {
    fn to_host(&self, msg: ServerMsg) {
        if let Some(tx) = &self.host_tx {
            push(tx, msg);
        }
    }

    fn to_viewers(&self, msg: &ServerMsg) {
        for v in self.viewers.values() {
            push(&v.tx, msg.clone());
        }
    }

    fn announce_count(&self) {
        let msg = ServerMsg::ViewerCount {
            count: self.viewers.len() as u32,
        };
        self.to_host(msg.clone());
        self.to_viewers(&msg);
    }

    fn nick_taken(&self, nick: &str) -> bool {
        let lower = nick.to_lowercase();
        self.viewers
            .values()
            .any(|v| v.nickname.to_lowercase() == lower)
    }

    fn viewer_infos(&self) -> Vec<ViewerInfo> {
        self.viewers
            .iter()
            .map(|(peer, v)| ViewerInfo {
                peer: *peer,
                nickname: v.nickname.clone(),
            })
            .collect()
    }
}

pub struct Hub {
    cfg: HubConfig,
    sessions: Mutex<HashMap<String, Session>>,
}

fn clean_passphrase(p: Option<String>) -> Result<Option<String>, ErrorCode> {
    match p.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) {
        Some(s) if s.chars().count() > 128 => Err(ErrorCode::BadMessage),
        other => Ok(other),
    }
}

impl Hub {
    pub fn new(cfg: HubConfig) -> Self {
        Self {
            cfg,
            sessions: Mutex::new(HashMap::new()),
        }
    }

    #[cfg(test)]
    pub fn session_count(&self) -> usize {
        self.sessions.lock().expect("hub poisoned").len()
    }

    /// Creates a session; returns its lookup key. Sends `Created` to `tx`.
    pub fn create(
        &self,
        conn: u64,
        tx: Tx,
        passphrase: Option<String>,
        has_audio: bool,
    ) -> Result<String, ErrorCode> {
        let passphrase = clean_passphrase(passphrase)?;
        let mut rng = rand::rng();
        let mut map = self.sessions.lock().expect("hub poisoned");
        if map.len() >= self.cfg.max_sessions {
            return Err(ErrorCode::RateLimited);
        }
        let mut found = None;
        for _ in 0..16 {
            let code = codes::generate_code(&mut rng, self.cfg.code_words);
            let key = code.to_ascii_lowercase();
            if !map.contains_key(&key) {
                found = Some((code, key));
                break;
            }
        }
        let (code, key) = found.ok_or(ErrorCode::Internal)?;
        let token = format!("{:032x}", rng.random::<u128>());
        push(
            &tx,
            ServerMsg::Created {
                code: code.clone(),
                host_token: token.clone(),
                ice_servers: self.cfg.ice_servers.clone(),
                max_viewers: self.cfg.max_viewers as u32,
            },
        );
        map.insert(
            key.clone(),
            Session {
                code,
                host_token: token,
                host_conn: conn,
                host_tx: Some(tx),
                status: HostStatus::Live,
                epoch: 0,
                passphrase,
                has_audio,
                next_peer: HOST_ID + 1,
                viewers: BTreeMap::new(),
            },
        );
        Ok(key)
    }

    /// Re-attaches a host (refresh, or reconnect after a network drop).
    pub fn resume(&self, conn: u64, tx: Tx, code: &str, token: &str) -> Result<String, ErrorCode> {
        let key = codes::normalize_code(code).ok_or(ErrorCode::NotFound)?;
        let mut map = self.sessions.lock().expect("hub poisoned");
        let s = map.get_mut(&key).ok_or(ErrorCode::NotFound)?;
        if !ct_eq(s.host_token.as_bytes(), token.as_bytes()) {
            return Err(ErrorCode::Forbidden);
        }
        s.host_conn = conn;
        s.host_tx = Some(tx.clone());
        s.status = HostStatus::Live;
        s.epoch += 1;
        push(
            &tx,
            ServerMsg::Resumed {
                code: s.code.clone(),
                has_audio: s.has_audio,
                viewers: s.viewer_infos(),
                ice_servers: self.cfg.ice_servers.clone(),
            },
        );
        s.to_viewers(&ServerMsg::HostState {
            status: HostStatus::Live,
            grace_secs: None,
        });
        Ok(key)
    }

    pub fn join(
        &self,
        tx: Tx,
        code: &str,
        nickname: Option<String>,
        passphrase: Option<String>,
    ) -> Result<(String, PeerId), ErrorCode> {
        let key = codes::normalize_code(code).ok_or(ErrorCode::NotFound)?;
        let mut map = self.sessions.lock().expect("hub poisoned");
        let s = map.get_mut(&key).ok_or(ErrorCode::NotFound)?;
        if let Some(expected) = &s.passphrase {
            let given = passphrase.unwrap_or_default();
            if !ct_eq(expected.as_bytes(), given.trim().as_bytes()) {
                return Err(ErrorCode::WrongPassphrase);
            }
        }
        if s.viewers.len() >= self.cfg.max_viewers {
            return Err(ErrorCode::SessionFull);
        }
        let requested = nickname.as_deref().map(str::trim).filter(|n| !n.is_empty());
        let nickname = match requested {
            Some(n) => {
                let n = nick::sanitize(n).ok_or(ErrorCode::NicknameInvalid)?;
                if s.nick_taken(&n) {
                    return Err(ErrorCode::NicknameTaken);
                }
                n
            }
            None => {
                let mut rng = rand::rng();
                (0..32)
                    .map(|_| nick::default_nickname(&mut rng))
                    .find(|n| !s.nick_taken(n))
                    .ok_or(ErrorCode::Internal)?
            }
        };
        let peer = s.next_peer;
        s.next_peer += 1;
        s.viewers.insert(
            peer,
            Viewer {
                nickname: nickname.clone(),
                tx: tx.clone(),
            },
        );
        push(
            &tx,
            ServerMsg::Joined {
                peer,
                nickname: nickname.clone(),
                host: s.status,
                has_audio: s.has_audio,
                viewer_count: s.viewers.len() as u32,
                ice_servers: self.cfg.ice_servers.clone(),
            },
        );
        s.to_host(ServerMsg::ViewerJoined { peer, nickname });
        s.announce_count();
        Ok((key, peer))
    }

    /// Relays opaque signalling data. `from` is assigned by the server, never
    /// taken from the client. Mesh topology: viewers may only address the host.
    pub fn signal(&self, key: &str, from: PeerId, to: PeerId, data: Value) {
        let map = self.sessions.lock().expect("hub poisoned");
        let Some(s) = map.get(key) else { return };
        let msg = ServerMsg::Signal { from, data };
        if from == HOST_ID {
            if let Some(v) = s.viewers.get(&to) {
                push(&v.tx, msg);
            }
        } else if to == HOST_ID {
            s.to_host(msg);
        }
    }

    pub fn set_audio(&self, key: &str, has_audio: bool) {
        let mut map = self.sessions.lock().expect("hub poisoned");
        if let Some(s) = map.get_mut(key) {
            s.has_audio = has_audio;
            s.to_viewers(&ServerMsg::AudioChanged { has_audio });
        }
    }

    pub fn kick(&self, key: &str, peer: PeerId) {
        let mut map = self.sessions.lock().expect("hub poisoned");
        if let Some(s) = map.get_mut(key) {
            if let Some(v) = s.viewers.remove(&peer) {
                push(&v.tx, ServerMsg::Kicked);
                s.to_host(ServerMsg::ViewerLeft { peer });
                s.announce_count();
            }
        }
    }

    pub fn leave_viewer(&self, key: &str, peer: PeerId) {
        let mut map = self.sessions.lock().expect("hub poisoned");
        if let Some(s) = map.get_mut(key) {
            if s.viewers.remove(&peer).is_some() {
                s.to_host(ServerMsg::ViewerLeft { peer });
                s.announce_count();
            }
        }
    }

    /// Explicit end by the host (or expired grace period).
    pub fn end(&self, key: &str) {
        let mut map = self.sessions.lock().expect("hub poisoned");
        if let Some(s) = map.remove(key) {
            s.to_viewers(&ServerMsg::HostState {
                status: HostStatus::Ended,
                grace_secs: None,
            });
        }
    }

    /// Marks the host as gone. Returns `(epoch, grace)` so the caller can start a
    /// timer and call [`Hub::expire_host`]. Stale connections (already replaced
    /// by a resume) are ignored.
    pub fn host_disconnected(
        &self,
        key: &str,
        conn: u64,
        kind: Disconnect,
    ) -> Option<(u64, Duration)> {
        let mut map = self.sessions.lock().expect("hub poisoned");
        let s = map.get_mut(key)?;
        if s.host_conn != conn {
            return None;
        }
        s.host_tx = None;
        let (status, grace) = match kind {
            Disconnect::Left => (HostStatus::Left, self.cfg.leave_grace),
            Disconnect::Lost => (HostStatus::Lost, self.cfg.host_grace),
        };
        s.status = status;
        s.epoch += 1;
        s.to_viewers(&ServerMsg::HostState {
            status,
            grace_secs: Some(grace.as_secs()),
        });
        Some((s.epoch, grace))
    }

    /// Ends the session if the host did not come back since `epoch`.
    pub fn expire_host(&self, key: &str, epoch: u64) {
        let mut map = self.sessions.lock().expect("hub poisoned");
        let expired =
            matches!(map.get(key), Some(s) if s.epoch == epoch && s.status != HostStatus::Live);
        if expired {
            if let Some(s) = map.remove(key) {
                s.to_viewers(&ServerMsg::HostState {
                    status: HostStatus::Ended,
                    grace_secs: None,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> HubConfig {
        HubConfig {
            max_viewers: 2,
            max_sessions: 10,
            code_words: 3,
            host_grace: Duration::from_secs(60),
            leave_grace: Duration::from_secs(10),
            ice_servers: vec![],
        }
    }

    fn chan() -> (Tx, mpsc::Receiver<ServerMsg>) {
        mpsc::channel(64)
    }

    fn drain(rx: &mut mpsc::Receiver<ServerMsg>) -> Vec<ServerMsg> {
        let mut out = Vec::new();
        while let Ok(m) = rx.try_recv() {
            out.push(m);
        }
        out
    }

    /// Creates a session; returns (hub, key, code, token, host_rx).
    fn setup(passphrase: Option<&str>) -> (Hub, String, String, String, mpsc::Receiver<ServerMsg>) {
        let hub = Hub::new(cfg());
        let (tx, mut rx) = chan();
        let key = hub
            .create(1, tx, passphrase.map(String::from), false)
            .unwrap();
        let (code, token) = match drain(&mut rx).remove(0) {
            ServerMsg::Created {
                code, host_token, ..
            } => (code, host_token),
            other => panic!("unexpected {other:?}"),
        };
        (hub, key, code, token, rx)
    }

    #[test]
    fn join_notifies_host_and_counts_everyone() {
        let (hub, _key, code, _t, mut host) = setup(None);
        let (tx, mut v) = chan();
        let (_, peer) = hub
            .join(tx, &code, Some("LemonOtter".into()), None)
            .unwrap();
        assert_eq!(peer, 1);
        let host_msgs = drain(&mut host);
        assert!(host_msgs.contains(&ServerMsg::ViewerJoined {
            peer: 1,
            nickname: "LemonOtter".into()
        }));
        assert!(host_msgs.contains(&ServerMsg::ViewerCount { count: 1 }));
        let viewer_msgs = drain(&mut v);
        assert!(matches!(viewer_msgs[0], ServerMsg::Joined { peer: 1, .. }));
        assert!(!viewer_msgs
            .iter()
            .any(|m| matches!(m, ServerMsg::ViewerJoined { .. })));
    }

    #[test]
    fn default_nickname_is_assigned() {
        let (hub, _k, code, _t, _h) = setup(None);
        let (tx, mut v) = chan();
        hub.join(tx, &code, None, None).unwrap();
        match &drain(&mut v)[0] {
            ServerMsg::Joined { nickname, .. } => assert!(nick::sanitize(nickname).is_some()),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn passphrase_capacity_and_nickname_rules() {
        let (hub, _k, code, _t, _h) = setup(Some("blue door"));
        let (t1, _r1) = chan();
        assert_eq!(
            hub.join(t1, &code, None, None).unwrap_err(),
            ErrorCode::WrongPassphrase
        );
        let (t2, _r2) = chan();
        hub.join(t2, &code, Some("AAaa".into()), Some("blue door".into()))
            .unwrap();
        let (t3, _r3) = chan();
        assert_eq!(
            hub.join(t3, &code, Some("aaAA".into()), Some("blue door".into()))
                .unwrap_err(),
            ErrorCode::NicknameTaken
        );
        let (t4, _r4) = chan();
        assert_eq!(
            hub.join(t4, &code, Some("bad name".into()), Some("blue door".into()))
                .unwrap_err(),
            ErrorCode::NicknameInvalid
        );
        let (t5, _r5) = chan();
        hub.join(t5, &code, None, Some("blue door".into())).unwrap();
        let (t6, _r6) = chan();
        assert_eq!(
            hub.join(t6, &code, None, Some("blue door".into()))
                .unwrap_err(),
            ErrorCode::SessionFull
        );
    }

    #[test]
    fn unknown_or_malformed_codes_are_not_found() {
        let (hub, ..) = setup(None);
        let (tx, _rx) = chan();
        assert_eq!(
            hub.join(tx.clone(), "nope", None, None).unwrap_err(),
            ErrorCode::NotFound
        );
        assert_eq!(
            hub.join(tx, "Amber-11-Harbor-Willow-Orchid", None, None)
                .unwrap_err(),
            ErrorCode::NotFound
        );
    }

    #[test]
    fn signalling_is_host_viewer_only() {
        let (hub, key, code, _t, mut host) = setup(None);
        let (t1, mut v1) = chan();
        let (t2, mut v2) = chan();
        let (_, p1) = hub.join(t1, &code, None, None).unwrap();
        let (_, p2) = hub.join(t2, &code, None, None).unwrap();
        drain(&mut host);
        drain(&mut v1);
        drain(&mut v2);

        hub.signal(&key, p1, p2, Value::Null); // viewer -> viewer: dropped
        assert!(drain(&mut v2).is_empty());
        hub.signal(&key, p1, HOST_ID, Value::Bool(true)); // viewer -> host
        assert_eq!(
            drain(&mut host),
            vec![ServerMsg::Signal {
                from: p1,
                data: Value::Bool(true)
            }]
        );
        hub.signal(&key, HOST_ID, p2, Value::Bool(false)); // host -> viewer
        assert_eq!(
            drain(&mut v2),
            vec![ServerMsg::Signal {
                from: HOST_ID,
                data: Value::Bool(false)
            }]
        );
        assert!(drain(&mut v1).is_empty());
    }

    #[test]
    fn kick_removes_viewer_and_tells_everyone() {
        let (hub, key, code, _t, mut host) = setup(None);
        let (t1, mut v1) = chan();
        let (_, p1) = hub.join(t1, &code, None, None).unwrap();
        drain(&mut host);
        drain(&mut v1);
        hub.kick(&key, p1);
        assert_eq!(drain(&mut v1), vec![ServerMsg::Kicked]);
        let msgs = drain(&mut host);
        assert!(msgs.contains(&ServerMsg::ViewerLeft { peer: p1 }));
        assert!(msgs.contains(&ServerMsg::ViewerCount { count: 0 }));
    }

    #[test]
    fn lost_host_gets_long_grace_then_resumes_with_token() {
        let (hub, key, code, token, host) = setup(None);
        let (t1, mut v1) = chan();
        hub.join(t1, &code, None, None).unwrap();
        drain(&mut v1);
        drop(host);

        let (epoch, grace) = hub.host_disconnected(&key, 1, Disconnect::Lost).unwrap();
        assert_eq!(grace, Duration::from_secs(60));
        assert_eq!(
            drain(&mut v1),
            vec![ServerMsg::HostState {
                status: HostStatus::Lost,
                grace_secs: Some(60)
            }]
        );

        let (tx, mut new_host) = chan();
        assert_eq!(
            hub.resume(2, tx.clone(), &code, "wrong").unwrap_err(),
            ErrorCode::Forbidden
        );
        hub.resume(2, tx, &code, &token).unwrap();
        assert!(matches!(drain(&mut new_host)[0], ServerMsg::Resumed { .. }));
        assert_eq!(
            drain(&mut v1),
            vec![ServerMsg::HostState {
                status: HostStatus::Live,
                grace_secs: None
            }]
        );

        hub.expire_host(&key, epoch); // stale timer: must not end the session
        assert_eq!(hub.session_count(), 1);
    }

    #[test]
    fn left_host_has_short_grace_and_expiry_ends_session() {
        let (hub, key, code, _t, _host) = setup(None);
        let (t1, mut v1) = chan();
        hub.join(t1, &code, None, None).unwrap();
        drain(&mut v1);
        let (epoch, grace) = hub.host_disconnected(&key, 1, Disconnect::Left).unwrap();
        assert_eq!(grace, Duration::from_secs(10));
        hub.expire_host(&key, epoch);
        assert_eq!(hub.session_count(), 0);
        let msgs = drain(&mut v1);
        assert_eq!(
            msgs.last(),
            Some(&ServerMsg::HostState {
                status: HostStatus::Ended,
                grace_secs: None
            })
        );
    }

    #[test]
    fn superseded_host_connection_is_ignored() {
        let (hub, key, code, token, _host) = setup(None);
        let (tx, _rx) = chan();
        hub.resume(2, tx, &code, &token).unwrap();
        assert!(hub.host_disconnected(&key, 1, Disconnect::Lost).is_none());
    }

    #[test]
    fn explicit_end_removes_session() {
        let (hub, key, code, _t, _h) = setup(None);
        let (t1, mut v1) = chan();
        hub.join(t1, &code, None, None).unwrap();
        drain(&mut v1);
        hub.end(&key);
        assert_eq!(hub.session_count(), 0);
        assert_eq!(
            drain(&mut v1),
            vec![ServerMsg::HostState {
                status: HostStatus::Ended,
                grace_secs: None
            }]
        );
    }

    #[test]
    fn audio_flag_reaches_viewers() {
        let (hub, key, code, _t, _h) = setup(None);
        let (t1, mut v1) = chan();
        hub.join(t1, &code, None, None).unwrap();
        drain(&mut v1);
        hub.set_audio(&key, true);
        assert_eq!(
            drain(&mut v1),
            vec![ServerMsg::AudioChanged { has_audio: true }]
        );
    }
}
