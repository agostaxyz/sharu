# Contributing to sharu

Thanks for helping. Issues and pull requests are welcome.

- Small fixes: open a PR directly. Bigger changes: open an issue first so we agree on the approach.
- Bug reports: include browser, version and OS for both the host and the viewer.
- Security problems: do **not** open an issue; see [SECURITY.md](SECURITY.md).

## Setup
Node 22+ and **pnpm** (we do not use npm or Bun).
```sh
pnpm install
pnpm dev                 # http://localhost:4321
pnpm check && pnpm test  # types and unit tests
pnpm e2e                 # Playwright + axe (installs browsers on first run: pnpm exec playwright install)
```

## What a good PR looks like
1. One concern per PR; explain the why in the description.
2. Tests for behaviour changes.
3. Accessibility is a feature: keyboard operable, labelled controls, announced status changes, `rem` units, no inline styles or scripts (strict CSP). `pnpm e2e` runs axe.
4. Few dependencies. A new one must have an AGPL-compatible license (MIT, Apache-2.0, BSD, ISC, MPL-2.0, LGPL/GPL/AGPL) and be added to [CREDITS.md](CREDITS.md).

## Licensing rules (important)
- Contributions are licensed AGPL-3.0-or-later.
- **Never copy code you cannot relicense.** If you adapt code from another project, say so in the PR, keep its notices, and add it to CREDITS.md. GPL-3.0 and AGPL-3.0 code is compatible; other copyleft or proprietary code is not.
- AI-assisted code is welcome, but you are responsible for it: review it, run it, and make sure it copies nothing.

## How CI treats your PR
PRs run `ci.yml` with a read-only token and no secrets. First-time contributors need a maintainer to approve the run. Only a merge to `main` (maintainers only) deploys the site; PRs never deploy.
