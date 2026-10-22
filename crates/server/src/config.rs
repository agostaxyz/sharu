// SPDX-License-Identifier: AGPL-3.0-or-later
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use sharu_proto::IceServer;

use crate::hub::HubConfig;

#[derive(Clone, Debug)]
pub struct Config {
    pub bind: SocketAddr,
    /// Directory with the built Astro site; when unset only `/ws` and `/healthz` are served.
    pub static_dir: Option<PathBuf>,
    /// Exact origins allowed to open `/ws`. Empty = same-origin only.
    pub allowed_origins: Vec<String>,
    /// Trust the first `X-Forwarded-For` entry. Only enable behind a proxy that overwrites it.
    pub trust_proxy: bool,
    pub hub: HubConfig,
}

fn env_parse<T: FromStr>(key: &str, default: T) -> T {
    match std::env::var(key) {
        Ok(v) => v.parse().unwrap_or_else(|_| panic!("invalid value for {key}: {v}")),
        Err(_) => default,
    }
}

impl Config {
    pub fn default_local() -> Self {
        Self {
            bind: "127.0.0.1:8080".parse().expect("static addr"),
            static_dir: None,
            allowed_origins: Vec::new(),
            trust_proxy: false,
            hub: HubConfig {
                max_viewers: 4,
                max_sessions: 1000,
                code_words: 3,
                host_grace: Duration::from_secs(60),
                leave_grace: Duration::from_secs(10),
                ice_servers: vec![IceServer {
                    urls: vec!["stun:stun.cloudflare.com:3478".to_string()],
                    username: None,
                    credential: None,
                }],
            },
        }
    }

    pub fn from_env() -> Self {
        let mut c = Self::default_local();
        c.bind = env_parse("BIND", c.bind);
        c.static_dir = std::env::var("STATIC_DIR").ok().map(PathBuf::from);
        if let Ok(v) = std::env::var("ALLOWED_ORIGINS") {
            c.allowed_origins = v
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
        c.trust_proxy = env_parse("TRUST_PROXY", c.trust_proxy);
        c.hub.max_viewers = env_parse("MAX_VIEWERS", c.hub.max_viewers);
        c.hub.max_sessions = env_parse("MAX_SESSIONS", c.hub.max_sessions);
        c.hub.code_words = env_parse("CODE_WORDS", c.hub.code_words);
        c.hub.host_grace = Duration::from_secs(env_parse("HOST_GRACE_SECS", 60u64));
        c.hub.leave_grace = Duration::from_secs(env_parse("LEAVE_GRACE_SECS", 10u64));
        if let Ok(v) = std::env::var("ICE_SERVERS") {
            c.hub.ice_servers =
                serde_json::from_str(&v).expect("ICE_SERVERS must be a JSON array of IceServer");
        }
        c
    }
}
