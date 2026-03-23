# Changelog

## 0.1.1 - 2026-03-22

- Made `install-hooks` worktree-safe so installing hooks in one linked worktree no longer hijacks another checkout.
- Wrote `core.hooksPath` with worktree-scoped git config and kept generated hooks checkout-local.
- Added regression coverage for linked-worktree installs and local-policy enforcement under per-worktree hook installation.

## 0.1.0 - 2026-03-22

- Initial public release.
