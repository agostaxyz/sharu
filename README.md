<h1 align="center">sharu</h1>

<p align="center">
  <b>Peer-to-peer screen sharing in your browser. Share your screen, send the link, done.</b>   
  <br>
  No account, no install, no processing of your audio or video.
</p>

<p align="center">
    <a href="https://github.com/agostaxyz/sharu/">
      <img alt="Website" src="https://img.shields.io/badge/agostaxyz.github.io/sharu-3fb950?style=for-the-badge">
    </a>
</p>

<p align="center">
    <a href="https://github.com/agostaxyz/sharu/actions/workflows/ci.yml">
      <img alt="CI" src="https://img.shields.io/github/actions/workflow/status/agostaxyz/sharu/ci.yml?style=flat-square&label=CI&logo=github&labelColor=0B120E">
    </a>
    <a href="https://github.com/agostaxyz/sharu/actions/workflows/pages.yml">
      <img alt="Deploy" src="https://img.shields.io/github/actions/workflow/status/agostaxyz/sharu/pages.yml?style=flat-square&label=Deploy&logo=github&labelColor=0B120E">
    </a>
    <img alt="Astro" src="https://img.shields.io/badge/Astro-0B120E?style=flat-square&logo=astro">
    <img alt="Svelte" src="https://img.shields.io/badge/Svelte-0B120E?style=flat-square&logo=svelte">
    <img alt="PWA" src="https://img.shields.io/badge/PWA-0B120E?style=flat-square&logo=pwa">
    <img alt="License: AGPL-3.0" src="https://img.shields.io/badge/license-AGPL--3.0-3a3a3a?style=flat-square&labelColor=0B120E&color=3fb950">
</p>

<table>
  <tr>
    <td width="25%" valign="top"><h3>🔗 One link</h3>Start a session and send one link, like <code>Caesar-23-Pyramid</code>. Easy to read aloud, hard to guess, and protected by a passphrase.</td>
    <td width="25%" valign="top"><h3>🎬 Untouched media</h3>Echo cancellation, noise suppression and auto-gain are switched off, so films keep their SFX and voices.</td>
    <td width="25%" valign="top"><h3>🔒 Private</h3>No account, no tracking, no data gathered. Video and audio go straight from your browser to your viewers'.</td>
    <td width="25%" valign="top"><h3>🪶 Lightweight</h3>A static site with nothing to run: it works from GitHub Pages. Installable as an app.</td>
  </tr>
</table>

**Browser support:** sharing needs a desktop browser with <a href="https://developer.mozilla.org/en-US/docs/Web/API/MediaDevices/getDisplayMedia"><code>getDisplayMedia()</code></a>. Phones and tablets can watch.
Firefox shares video only (no audio streaming yet); use a Chromium browser to share sound. On Linux, Chromium shares the audio of a browser _tab_ only.

## Get started

```sh
git clone https://github.com/agostaxyz/sharu && cd sharu/web
pnpm install
PUBLIC_SIGNALLING=p2p pnpm dev       # http://localhost:4321, no server needed
```

Tests: `pnpm test` and `pnpm e2e` in `web/`, `cargo test --workspace` at the root.
To run with the Rust signalling server instead, start `cargo run -p sharu-server` in another terminal and use plain `pnpm dev`.
Deployment is covered in [docs/DEPLOY.md](docs/DEPLOY.md). Screen capture needs a secure context: `localhost` works, a phone needs HTTPS.

## Under the hood

Video and audio are always peer to peer (WebRTC, one connection per viewer, up to four). Only the introduction differs:

```
serverless (GitHub Pages)   host ◄── public relays: the session code is the room ──► viewer
self-hosted                 host ◄──── WebSocket ──── sharu-server ──── WebSocket ────► viewer
```

- **`web/`**: Astro static site, Svelte 5 islands mounted by hand (keeps a strict CSP), Tailwind CSS.
- **`crates/server`**: optional Rust signalling server (axum): session codes, viewers, kicks, rate limits. **`crates/proto`**: its JSON protocol, shared with the web app through test fixtures.
- In serverless mode the host's browser plays the server's role, using [Trystero](https://github.com/dmotz/trystero) to find peers. Viewers can then see each other's connection details, and there is no relay for strict networks, so the self-hosted server remains the more robust option.
- Session codes carry about 34 bits; the passphrase (suggested for you, or your own) is the second factor.

## Prior art and credits

[Screego](https://github.com/screego/server) (GPL-3.0, Go) solves a similar problem as a self-hosted server and inspired the "introductions only, media peer to peer" shape. No Screego code is used. Everything we depend on is listed in [CREDITS.md](CREDITS.md).

## License

AGPL-3.0-or-later. If you run a modified sharu as a service, you must offer your users its source; the site footer links to it (`PUBLIC_SOURCE_URL`). Contributions are accepted under the same license.
