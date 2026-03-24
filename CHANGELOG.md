# Changelog

## 0.2.0 - 2026-03-24

- Changed `install-hooks` generated mode to derive managed hook wrappers from contract semantics (`protected_refs` and `local_policy`) instead of requiring `hooks.generated.hooks`.
- Removed `repo-owned` hook mode; `hooks.mode = "generated"` is now the only supported hook installation mode.
- Hard-cut old repo-owned hook config at the parser/validator boundary so legacy `[hooks.repo_owned]` and `[hooks.generated]` tables now fail instead of flowing through compatibility behavior.

## 0.1.1 - 2026-03-22

- Made `install-hooks` worktree-safe so installing hooks in one linked worktree no longer hijacks another checkout.
- Wrote `core.hooksPath` with worktree-scoped git config and kept generated hooks checkout-local.
- Added regression coverage for linked-worktree installs and local-policy enforcement under per-worktree hook installation.

## 0.1.0 - 2026-03-22

- Initial public release.
