// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tiny fixed-window rate limiter keyed by string (IP, connection id, ...).

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub struct RateLimiter {
    max: u32,
    window: Duration,
    hits: Mutex<HashMap<String, (Instant, u32)>>,
}

impl RateLimiter {
    pub fn new(max: u32, window: Duration) -> Self {
        Self { max, window, hits: Mutex::new(HashMap::new()) }
    }

    pub fn check(&self, key: &str) -> bool {
        self.check_at(key, Instant::now())
    }

    pub fn check_at(&self, key: &str, now: Instant) -> bool {
        let mut map = self.hits.lock().expect("rate limiter poisoned");
        if map.len() > 10_000 {
            let window = self.window;
            map.retain(|_, v| now.duration_since(v.0) < window);
        }
        let entry = map.entry(key.to_string()).or_insert((now, 0));
        if now.duration_since(entry.0) >= self.window {
            *entry = (now, 0);
        }
        if entry.1 >= self.max {
            return false;
        }
        entry.1 += 1;
        true
    }

    pub fn forget(&self, key: &str) {
        self.hits.lock().expect("rate limiter poisoned").remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_after_max_and_recovers_after_window() {
        let rl = RateLimiter::new(2, Duration::from_secs(10));
        let t0 = Instant::now();
        assert!(rl.check_at("ip", t0));
        assert!(rl.check_at("ip", t0));
        assert!(!rl.check_at("ip", t0));
        assert!(rl.check_at("other", t0));
        assert!(rl.check_at("ip", t0 + Duration::from_secs(11)));
    }
}
