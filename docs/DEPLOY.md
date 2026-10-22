# Deploying sharu

sharu has two parts. **The site** is static files. **The signalling server** is one small always-on process
(WebSocket). GitHub Pages can host the first, but not the second.

## Option A: GitHub Pages (site) + any host (signalling)
1. Repo **Settings → Pages → Source: GitHub Actions**. Every push to `main` deploys `web/` to
   `https://agostaxyz.github.io/sharu/` (workflow `pages.yml`).
2. Run `sharu-server` somewhere with HTTPS (see below) and set `ALLOWED_ORIGINS=https://agostaxyz.github.io`.
3. Repo **Settings → Secrets and variables → Actions → Variables**: add `SIGNAL_URL` = `wss://your-host.example/ws`.
   It is baked into the build and into the page CSP (`connect-src`). Re-run the Pages workflow.

Session links look like `https://agostaxyz.github.io/sharu/Caesar-23-Pyramid`. Pages serves its 404 page for them,
and that page *is* the viewer. (A side effect: link crawlers never index session codes.)

## Option B: one Docker image (site + signalling)
```sh
docker build -t sharu .
docker run -p 8080:8080 -e TRUST_PROXY=true sharu     # put Caddy/nginx with TLS in front
```
`deploy/Caddyfile` has a two-line Caddy config (automatic HTTPS, WebSocket works out of the box).
Build the site with `SHARU_BASE=/` for a domain root (the Dockerfile does).

## Option C: bare binary
`cargo build --release -p sharu-server`, then run with `STATIC_DIR=web/dist` (optional) behind a TLS proxy.

## Configuration (environment)
| Variable | Default | Meaning |
|---|---|---|
| `BIND` | `127.0.0.1:8080` | listen address |
| `STATIC_DIR` | unset | serve the built site too |
| `ALLOWED_ORIGINS` | same-origin | comma-separated exact origins allowed to open `/ws` |
| `TRUST_PROXY` | `false` | trust first `X-Forwarded-For` (only behind a proxy that overwrites it) |
| `MAX_VIEWERS` / `MAX_SESSIONS` | `4` / `1000` | caps |
| `CODE_WORDS` | `3` | words per code (2-4); 4 is about 44 bits |
| `HOST_GRACE_SECS` / `LEAVE_GRACE_SECS` | `60` / `10` | wait for a host after a lost connection / closed tab |
| `ICE_SERVERS` | public STUN | JSON array, e.g. add your TURN server |

## STUN and TURN
Peers behind strict NATs need a TURN relay, which costs bandwidth and is not free by nature. Without it some
viewers cannot connect. The default STUN server tells peers their public address; that reveals their IP to the STUN
operator, so self-host `coturn` if that matters to you.

## Free-tier reality check
Public GitHub repositories get free Actions minutes on standard runners and free Pages hosting, as far as we know;
verify current limits at github.com/pricing. The signalling server needs a host that allows long-lived WebSockets.
