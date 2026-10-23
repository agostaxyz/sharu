<h1 align="center">sharu</h1>

<p align="center">
  <b>Peer-to-peer screen sharing in your browser. Share a screen, send a link, done.</b><br>
  No account, no install, no processing of your audio or video. A small Rust signalling server and a<br>
  static, accessible, mobile-first web app. Open source (AGPL-3.0).
</p>

<p align="center">
  <img alt="Rust" src="https://img.shields.io/badge/Rust-signalling-b7410e?style=flat-square&logo=rust">
  <img alt="TypeScript" src="https://img.shields.io/badge/TypeScript-frontend-3178c6?style=flat-square&logo=typescript&logoColor=white">
  <img alt="Astro" src="https://img.shields.io/badge/Astro-static%20site-ff5d01?style=flat-square&logo=astro&logoColor=white">
  <img alt="Svelte" src="https://img.shields.io/badge/Svelte%205-islands-ff3e00?style=flat-square&logo=svelte&logoColor=white">
  <img alt="WebRTC" src="https://img.shields.io/badge/WebRTC-P2P-333333?style=flat-square&logo=webrtc&logoColor=white">
  <img alt="PWA" src="https://img.shields.io/badge/PWA-installable-5a0fc8?style=flat-square&logo=pwa&logoColor=white">
  <br>
  <img alt="License: AGPL-3.0" src="https://img.shields.io/badge/license-AGPL--3.0-3a3a3a?style=flat-square">
  <img alt="Status: early alpha" src="https://img.shields.io/badge/status-early%20alpha-d69e2e?style=flat-square">
  <img alt="Dark mode only" src="https://img.shields.io/badge/theme-dark%20only-0b120e?style=flat-square">
  <img alt="Accessibility: axe in CI" src="https://img.shields.io/badge/a11y-axe%20in%20CI-3fb950?style=flat-square">
  <br>
  <a href="https://github.com/agostaxyz/sharu/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/agostaxyz/sharu/actions/workflows/ci.yml/badge.svg"></a>
  <a href="https://github.com/agostaxyz/sharu/actions/workflows/pages.yml"><img alt="Deploy" src="https://github.com/agostaxyz/sharu/actions/workflows/pages.yml/badge.svg"></a>
</p>

<p align="center">
  <a href="https://agostaxyz.github.io/sharu/"><b>Use it: agostaxyz.github.io/sharu</b></a> ·
  <a href="#features">Features</a> ·
  <a href="#status">Status</a> ·
  <a href="#get-started">Get started</a> ·
  <a href="docs/DEPLOY.md">Deploy</a> ·
  <a href="CONTRIBUTING.md">Contribute</a>
</p>

> [!IMPORTANT]
> **Status: early alpha.** The signalling server, session codes, host/viewer lifecycle and the web shell exist
> and are tested. **Video does not flow yet**: the WebRTC mesh is the next milestone. See [Status](#status).

<table>
  <tr>
    <td width="25%" valign="top"><h3>🔗 One link</h3>Create a session, share <code>Caesar-23-Pyramid</code>-style link. Hard to guess, case-insensitive, optional passphrase on top.</td>
    <td width="25%" valign="top"><h3>🎬 Untouched media</h3>Echo cancellation, noise suppression and auto-gain are switched off on capture, so films keep their SFX and voices.</td>
    <td width="25%" valign="top"><h3>♿ Accessible</h3>Screen-reader tested markup, live regions, 3 rem touch targets, <code>rem</code>-based layout, reduced motion, forced colours, optional OpenDyslexic.</td>
    <td width="25%" valign="top"><h3>🪶 Lightweight</h3>Static site, no tracking, no accounts. The server only introduces peers; media goes browser to browser.</td>
  </tr>
</table>

## Features

| | Feature | State |
|---|---|---|
| 🛰️ | Rust signalling server (axum, WebSocket), in-memory sessions, rate limits, origin checks | ✅ implemented, unit-tested |
| 🔑 | Session codes (2-4 words + number), host token, optional passphrase | ✅ implemented, unit-tested |
| 👥 | Watcher count for everyone, nickname list and kick for the host, random `LemonOtter`-style nicknames | ✅ implemented |
| 🔌 | Host lifecycle: closed tab vs lost connection, grace periods, re-cast with a stored token | ✅ implemented |
| 🌐 | Static Astro + Svelte site, PWA manifest, dark-only green theme, dyslexia-friendly font toggle | ✅ implemented |
| 📺 | WebRTC mesh (up to 4 viewers), per-viewer bitrate caps, source switching with `replaceTrack` | 🚧 next |
| 🔊 | Volume slider, fullscreen, "host is sharing without audio" state | ✅ UI done, needs the stream |
| 📈 | Optional SFU for more viewers; optional embedded TURN | 🗓️ planned |

**Browser support.** Sharing needs a desktop browser with `getDisplayMedia`. Phones and tablets can watch.
Firefox shares video only (no audio via `getDisplayMedia` yet); use a Chromium browser to share sound. On Linux and
macOS, Chromium captures *tab* audio only.

## Status

We would rather be honest than impressive. What is verified and what is not:

- The Rust code and the web app were written without a compiler in the loop. Expect to fix compile errors on the
  first `cargo test` / `pnpm test`; [CONTRIBUTING.md](CONTRIBUTING.md) explains how to report them.
- Tests: Rust unit tests (hub state machine, codes, nicknames, rate limits, origin rules), TypeScript unit tests,
  a **shared contract** (`crates/proto/fixtures/`) checked by both languages, and Playwright + axe end-to-end checks.
- Known gaps are tracked as [issues](https://github.com/agostaxyz/sharu/issues).

## Get started

```sh
git clone https://github.com/agostaxyz/sharu && cd sharu
cargo test --workspace               # Rust tests
cd web && pnpm install && pnpm exec playwright install chromium firefox
pnpm test                            # TypeScript tests

# two terminals for local development
cargo run -p sharu-server            # signalling on 127.0.0.1:8080
cd web && SHARU_BASE=/ pnpm dev      # site on http://localhost:4321 (proxies /ws)
```

Screen capture needs a secure context: `localhost` works; to test from a phone use HTTPS (`mkcert` or a tunnel).
Deployment options (GitHub Pages + any host for the signalling server, or one Docker image) are in
[docs/DEPLOY.md](docs/DEPLOY.md).

## Under the hood

```
browser (host) ──WebSocket──► sharu-server ◄──WebSocket── browser (viewer)
      └────────────── WebRTC media, peer to peer ───────────────┘
```

- **`crates/proto`**: the JSON wire protocol (serde). **`crates/server`**: sessions, codes, limits, WebSocket.
- **`web/`**: Astro static site, Svelte 5 islands mounted manually (keeps a strict CSP), plain CSS tokens.
- The hub is synchronous and I/O-free, so the state machine is unit-tested without a runtime.
- Session codes carry about 34 bits with the default 3 words; a global join limiter caps guessing speed. Use
  `CODE_WORDS=4` (about 44 bits) or a passphrase for sensitive sessions.

## Prior art and credits

[Screego](https://github.com/screego/server) (GPL-3.0, Go) solves the same problem as a self-hosted server and
inspired the "signalling only, media peer to peer" shape. sharu differs in being a static, accessibility-first
web app with a Rust signalling server. No Screego code is used. Everything we depend on, and everything we were
inspired by, is listed in [CREDITS.md](CREDITS.md).

## License

AGPL-3.0-or-later. If you run a modified sharu as a service, you must offer your users its source; the site footer
links to it (`PUBLIC_SOURCE_URL`). Contributions are accepted under the same license.
