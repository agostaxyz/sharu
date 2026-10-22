// SPDX-License-Identifier: AGPL-3.0-or-later
//! Wire protocol: JSON text frames over WebSocket, tagged by `"type"`.
//! The TypeScript mirror lives in `web/src/lib/proto.ts`; both sides are
//! checked against `fixtures/*.json`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub type PeerId = u32;
/// The host is always peer 0; viewers get 1, 2, 3, ... per session.
pub const HOST_ID: PeerId = 0;
pub const MAX_MESSAGE_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HostStatus {
    /// Host connected.
    Live,
    /// Host closed the tab / navigated away (short grace for refreshes).
    Left,
    /// Host connection dropped without a goodbye (longer grace).
    Lost,
    /// Session is over.
    Ended,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    BadMessage,
    NotFound,
    WrongPassphrase,
    SessionFull,
    NicknameTaken,
    NicknameInvalid,
    Forbidden,
    RateLimited,
    AlreadyJoined,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IceServer {
    pub urls: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewerInfo {
    pub peer: PeerId,
    pub nickname: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMsg {
    Create {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        passphrase: Option<String>,
        #[serde(default)]
        has_audio: bool,
    },
    Resume {
        code: String,
        host_token: String,
    },
    Join {
        code: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        nickname: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        passphrase: Option<String>,
    },
    /// Opaque SDP/ICE payload relayed to `to`. Viewers may only address the host (0).
    Signal {
        to: PeerId,
        data: Value,
    },
    SetAudio {
        has_audio: bool,
    },
    Kick {
        peer: PeerId,
    },
    /// Best-effort "tab is closing" notice (pagehide).
    Leaving,
    End,
    Ping,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMsg {
    Created {
        code: String,
        host_token: String,
        ice_servers: Vec<IceServer>,
        max_viewers: u32,
    },
    Resumed {
        code: String,
        has_audio: bool,
        viewers: Vec<ViewerInfo>,
        ice_servers: Vec<IceServer>,
    },
    Joined {
        peer: PeerId,
        nickname: String,
        host: HostStatus,
        has_audio: bool,
        viewer_count: u32,
        ice_servers: Vec<IceServer>,
    },
    /// Host only.
    ViewerJoined {
        peer: PeerId,
        nickname: String,
    },
    /// Host only.
    ViewerLeft {
        peer: PeerId,
    },
    /// Everyone; viewers never see each other's nicknames.
    ViewerCount {
        count: u32,
    },
    Signal {
        from: PeerId,
        data: Value,
    },
    HostState {
        status: HostStatus,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        grace_secs: Option<u64>,
    },
    AudioChanged {
        has_audio: bool,
    },
    Kicked,
    Pong,
    Error {
        code: ErrorCode,
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLIENT: &str = include_str!("../fixtures/client.json");
    const SERVER: &str = include_str!("../fixtures/server.json");

    fn roundtrip<T>(raw: &str)
    where
        T: for<'de> Deserialize<'de> + Serialize + PartialEq + std::fmt::Debug,
    {
        let items: Vec<Value> = serde_json::from_str(raw).expect("fixture is a JSON array");
        assert!(!items.is_empty());
        for item in items {
            let msg: T = serde_json::from_value(item.clone())
                .unwrap_or_else(|e| panic!("cannot parse {item}: {e}"));
            let again: T = serde_json::from_str(&serde_json::to_string(&msg).unwrap()).unwrap();
            assert_eq!(msg, again);
        }
    }

    #[test]
    fn client_fixtures_roundtrip() {
        roundtrip::<ClientMsg>(CLIENT);
    }

    #[test]
    fn server_fixtures_roundtrip() {
        roundtrip::<ServerMsg>(SERVER);
    }

    #[test]
    fn unknown_type_is_rejected() {
        assert!(serde_json::from_str::<ClientMsg>(r#"{"type":"nope"}"#).is_err());
    }
}
