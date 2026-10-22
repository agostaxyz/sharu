# Contributing to sharu

Thanks for helping. Issues and pull requests are welcome.

## Before you start
- Small fixes: open a PR directly. Bigger changes: open an issue first so we agree on the approach.
- Be kind. Be specific. Include OS, browser and version in bug reports.
- Security problems: **do not** open an issue; see [SECURITY.md](SECURITY.md).

## Setup
Rust (stable) and Node 20+ with **pnpm**. We do not use npm or Bun.
```sh
cargo test --workspace
cd web && ./bootstrap.sh && pnpm test && pnpm e2e
```

## What a good PR looks like
1. One concern per PR; explain the why in the description.
2. Tests for behaviour changes. The wire protocol is a contract: change `crates/proto/src/lib.rs`,
   `web/src/lib/proto.ts` **and** the fixtures together.
3. `cargo fmt`, `cargo clippy -- -D warnings`, `pnpm check`, `pnpm test` pass.
4. Accessibility is a feature: keyboard operable, labelled controls, announced status changes, `rem` units,
   no inline styles or scripts (strict CSP). `pnpm e2e` runs axe.
5. Do not add runtime dependencies lightly. New dependencies must have a license compatible with AGPL-3.0
   (MIT, Apache-2.0, BSD, ISC, MPL-2.0, LGPL/GPL/AGPL) and be added to [CREDITS.md](CREDITS.md).

## Licensing rules (important)
- Contributions are licensed AGPL-3.0-or-later. Add `SPDX-License-Identifier: AGPL-3.0-or-later` to new source files.
- **Never copy code you cannot relicense.** If you adapt code from another project, say so in the PR, keep its
  notices, and add it to CREDITS.md. GPL-3.0 and AGPL-3.0 code is compatible; code under other copyleft or
  proprietary licenses is not.
- AI-assisted code is welcome, but you are responsible for it: review it, run it, and make sure it copies nothing.

## How CI treats your PR
PRs run `ci.yml` with a read-only token and no secrets. First-time contributors need a maintainer to approve
the run. Merging to `main` (maintainers only) is what deploys the site. PRs never deploy.

## Testing stack
`cargo test` (+ `cargo nextest` optional) for Rust, Vitest for TypeScript, Playwright + axe-core for end-to-end
and accessibility, shared JSON fixtures as the Rust/TypeScript contract.
