# Credits and third-party licenses

We credit what we use and what shaped us. If we missed something, open an issue.

## Inspiration (no code copied)
- **Screego** (GPL-3.0): "signalling server, media peer to peer" shape for screen sharing.
- **PhotoCraft** (MIT OR Apache-2.0): README structure, honest-status callout, corpus-style contract testing.
- **EFF Dice wordlists** and passphrase literature: reason we use word-based codes (our own word list is original).

## Runtime dependencies (see `Cargo.lock` and `web/pnpm-lock.yaml` for exact versions)
Rust: axum, tokio, tower-http, serde, serde_json, tracing, rand (MIT / Apache-2.0).
Web: Astro, Svelte, @vite-pwa/astro (MIT). Fonts: Atkinson Hyperlegible Next (OFL-1.1, Braille Institute),
JetBrains Mono (OFL-1.1), OpenDyslexic (see its license) via Fontsource.

## Dev/test only
Vitest (MIT), Playwright (Apache-2.0), axe-core (MPL-2.0).

Audit commands: `cargo install cargo-deny && cargo deny check licenses` and `pnpm licenses list`.
