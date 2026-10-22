// SPDX-License-Identifier: AGPL-3.0-or-later
//! WebSocket endpoint: parses client messages, drives the hub, runs heartbeats
//! and the host grace timers.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, State};
use axum::http::header::{HOST, ORIGIN};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use sharu_proto::{
    ClientMsg, ErrorCode, HostStatus, PeerId, ServerMsg, HOST_ID, MAX_MESSAGE_BYTES,
};
use tokio::sync::mpsc;

use crate::config::Config;
use crate::hub::{Disconnect, Hub, Tx};
use crate::ratelimit::RateLimiter;

pub struct App {
    pub cfg: Config,
    pub hub: Hub,
    create_limiter: RateLimiter,
    join_limiter: RateLimiter,
    global_join_limiter: RateLimiter,
    msg_limiter: RateLimiter,
    conn_counter: AtomicU64,
}

impl App {
    pub fn new(cfg: Config) -> Self {
        let min = Duration::from_secs(60);
        Self {
            hub: Hub::new(cfg.hub.clone()),
            cfg,
            create_limiter: RateLimiter::new(10, min),
            join_limiter: RateLimiter::new(60, min),
            // Caps total guessing speed against the code space (see README: Security).
            global_join_limiter: RateLimiter::new(1200, min),
            msg_limiter: RateLimiter::new(400, Duration::from_secs(10)),
            conn_counter: AtomicU64::new(1),
        }
    }
}

enum Role {
    None,
    Host { key: String },
    Viewer { key: String, peer: PeerId },
}

enum Flow {
    Continue,
    MarkLeft,
    Stop,
}

pub fn origin_allowed(cfg: &Config, headers: &HeaderMap) -> bool {
    let Some(origin) = headers.get(ORIGIN).and_then(|v| v.to_str().ok()) else {
        return true; // non-browser client: no ambient credentials to abuse
    };
    if !cfg.allowed_origins.is_empty() {
        return cfg.allowed_origins.iter().any(|o| o == origin);
    }
    let host = headers.get(HOST).and_then(|v| v.to_str().ok());
    origin.split("://").nth(1).is_some() && origin.split("://").nth(1) == host
}

pub fn client_ip(cfg: &Config, headers: &HeaderMap, addr: SocketAddr) -> String {
    if cfg.trust_proxy {
        if let Some(first) = headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(',').next())
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            return first.to_string();
        }
    }
    addr.ip().to_string()
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(app): State<Arc<App>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Response {
    if !origin_allowed(&app.cfg, &headers) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let ip = client_ip(&app.cfg, &headers, addr);
    ws.max_message_size(MAX_MESSAGE_BYTES)
        .on_upgrade(move |socket| handle(socket, app, ip))
}

fn err(tx: &Tx, code: ErrorCode, message: &str) {
    let _ = tx.try_send(ServerMsg::Error {
        code,
        message: message.to_string(),
    });
}

fn dispatch(app: &App, ip: &str, conn: u64, tx: &Tx, role: &mut Role, msg: ClientMsg) -> Flow {
    match msg {
        ClientMsg::Ping => {
            let _ = tx.try_send(ServerMsg::Pong);
        }
        ClientMsg::Create {
            passphrase,
            has_audio,
        } => {
            if !matches!(*role, Role::None) {
                err(tx, ErrorCode::AlreadyJoined, "already in a session");
            } else if !app.create_limiter.check(ip) {
                err(
                    tx,
                    ErrorCode::RateLimited,
                    "too many sessions, try again later",
                );
            } else {
                match app.hub.create(conn, tx.clone(), passphrase, has_audio) {
                    Ok(key) => *role = Role::Host { key },
                    Err(c) => err(tx, c, "could not create session"),
                }
            }
        }
        ClientMsg::Resume { code, host_token } => {
            if !matches!(*role, Role::None) {
                err(tx, ErrorCode::AlreadyJoined, "already in a session");
            } else {
                match app.hub.resume(conn, tx.clone(), &code, &host_token) {
                    Ok(key) => *role = Role::Host { key },
                    Err(c) => err(tx, c, "could not resume session"),
                }
            }
        }
        ClientMsg::Join {
            code,
            nickname,
            passphrase,
        } => {
            if !matches!(*role, Role::None) {
                err(tx, ErrorCode::AlreadyJoined, "already in a session");
            } else if !app.join_limiter.check(ip) || !app.global_join_limiter.check("all") {
                err(
                    tx,
                    ErrorCode::RateLimited,
                    "too many attempts, try again later",
                );
            } else {
                match app.hub.join(tx.clone(), &code, nickname, passphrase) {
                    Ok((key, peer)) => *role = Role::Viewer { key, peer },
                    Err(c) => err(tx, c, "could not join"),
                }
            }
        }
        ClientMsg::Signal { to, data } => match &*role {
            Role::Host { key } => app.hub.signal(key, HOST_ID, to, data),
            Role::Viewer { key, peer } => app.hub.signal(key, *peer, to, data),
            Role::None => err(tx, ErrorCode::Forbidden, "not in a session"),
        },
        ClientMsg::SetAudio { has_audio } => match &*role {
            Role::Host { key } => app.hub.set_audio(key, has_audio),
            _ => err(tx, ErrorCode::Forbidden, "host only"),
        },
        ClientMsg::Kick { peer } => match &*role {
            Role::Host { key } => app.hub.kick(key, peer),
            _ => err(tx, ErrorCode::Forbidden, "host only"),
        },
        ClientMsg::Leaving => {
            if matches!(*role, Role::Host { .. }) {
                return Flow::MarkLeft;
            }
        }
        ClientMsg::End => {
            if let Role::Host { key } = &*role {
                app.hub.end(key);
                *role = Role::None;
                return Flow::Stop;
            }
            err(tx, ErrorCode::Forbidden, "host only");
        }
    }
    Flow::Continue
}

async fn send_json(socket: &mut WebSocket, msg: &ServerMsg) -> Result<(), axum::Error> {
    let text = serde_json::to_string(msg).map_err(axum::Error::new)?;
    socket.send(Message::Text(text.into())).await
}

async fn handle(mut socket: WebSocket, app: Arc<App>, ip: String) {
    let conn = app.conn_counter.fetch_add(1, Ordering::Relaxed);
    let conn_key = conn.to_string();
    let (tx, mut rx) = mpsc::channel::<ServerMsg>(64);
    let mut role = Role::None;
    let (mut left, mut got_close) = (false, false);
    let started = Instant::now();
    let mut last_seen = started;
    let mut tick = tokio::time::interval(Duration::from_secs(20));

    loop {
        tokio::select! {
            incoming = socket.recv() => match incoming {
                Some(Ok(Message::Text(text))) => {
                    last_seen = Instant::now();
                    if !app.msg_limiter.check(&conn_key) {
                        err(&tx, ErrorCode::RateLimited, "slow down");
                        continue;
                    }
                    match serde_json::from_str::<ClientMsg>(text.as_str()) {
                        Ok(m) => match dispatch(&app, &ip, conn, &tx, &mut role, m) {
                            Flow::Continue => {}
                            Flow::MarkLeft => left = true,
                            Flow::Stop => break,
                        },
                        Err(_) => err(&tx, ErrorCode::BadMessage, "invalid message"),
                    }
                }
                Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) => last_seen = Instant::now(),
                Some(Ok(Message::Close(_))) => { got_close = true; break; }
                Some(Ok(Message::Binary(_))) | Some(Err(_)) | None => break,
            },
            outgoing = rx.recv() => match outgoing {
                Some(msg) => {
                    if send_json(&mut socket, &msg).await.is_err() { break; }
                    // Terminal messages: the hub has already dropped this viewer.
                    if matches!(msg, ServerMsg::Kicked)
                        || matches!(msg, ServerMsg::HostState { status: HostStatus::Ended, .. })
                    { break; }
                }
                None => break,
            },
            _ = tick.tick() => {
                if last_seen.elapsed() > Duration::from_secs(45) { break; }
                if matches!(role, Role::None) && started.elapsed() > Duration::from_secs(30) { break; }
                if socket.send(Message::Ping(Vec::new().into())).await.is_err() { break; }
            }
        }
    }

    let kind = if left || got_close {
        Disconnect::Left
    } else {
        Disconnect::Lost
    };
    match role {
        Role::Host { key } => {
            if let Some((epoch, grace)) = app.hub.host_disconnected(&key, conn, kind) {
                let app = app.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(grace).await;
                    app.hub.expire_host(&key, epoch);
                });
            }
        }
        Role::Viewer { key, peer } => app.hub.leave_viewer(&key, peer),
        Role::None => {}
    }
    app.msg_limiter.forget(&conn_key);
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn headers(pairs: &[(&'static str, &'static str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.insert(*k, HeaderValue::from_static(v));
        }
        h
    }

    #[test]
    fn origin_rules() {
        let mut cfg = Config::default_local();
        assert!(origin_allowed(&cfg, &headers(&[])));
        assert!(origin_allowed(
            &cfg,
            &headers(&[("origin", "https://a.example"), ("host", "a.example")])
        ));
        assert!(!origin_allowed(
            &cfg,
            &headers(&[("origin", "https://evil.example"), ("host", "a.example")])
        ));
        cfg.allowed_origins = vec!["https://agostaxyz.github.io".into()];
        assert!(origin_allowed(
            &cfg,
            &headers(&[("origin", "https://agostaxyz.github.io")])
        ));
        assert!(!origin_allowed(
            &cfg,
            &headers(&[("origin", "https://a.example"), ("host", "a.example")])
        ));
    }

    #[test]
    fn forwarded_for_only_when_trusted() {
        let addr: SocketAddr = "10.0.0.1:1234".parse().unwrap();
        let h = headers(&[("x-forwarded-for", "203.0.113.7, 10.0.0.1")]);
        let mut cfg = Config::default_local();
        assert_eq!(client_ip(&cfg, &h, addr), "10.0.0.1");
        cfg.trust_proxy = true;
        assert_eq!(client_ip(&cfg, &h, addr), "203.0.113.7");
    }
}
