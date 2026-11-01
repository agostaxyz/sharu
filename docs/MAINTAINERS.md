# Maintainer checklist (GitHub settings)

**General:** Wiki off · Discussions off · Projects off · Sponsorships off (for now) · Issues on ·
Pull requests on · *Allow auto-merge* off · *Automatically delete head branches* on ·
*Auto-close issues with merged linked pull requests* on · Squash merging only.

**Rulesets / branch protection on `main`:** require a pull request, require 1 approving review (CODEOWNERS),
require status checks (`web`), block force pushes and deletions, require linear history.

**Actions → General:**
- Fork pull request workflows: *Require approval for all outside collaborators*.
- Workflow permissions: *Read repository contents* (jobs request more only when needed).
- Allow actions: GitHub-authored + the few third-party actions we use; pin them to commit SHAs
  (Dependabot keeps them updated).
- Never use `pull_request_target` or put secrets in PR-triggered workflows.

**Pages:** Source *GitHub Actions*. Environment `github-pages`: deployment branches limited to `main`.

**Security:** private vulnerability reporting on · Dependabot alerts and updates on · secret scanning on ·
code scanning (CodeQL default setup) on.

