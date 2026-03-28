# repocert

Certification and enforcement for git repositories.

repocert lets a repository define a contract — checks, fixers, and profiles —
then certify exact commits against that contract and enforce certification
on protected branches. The contract lives in the repo and travels with the code.

## Why

CI catches problems after the push. repocert catches them before.

A certification is a local certification record binding a specific commit to a
specific profile under a specific contract fingerprint. Certifications are
SSH-signed, so protected ref enforcement can trust only
records signed by repo-trusted signer keys. If the contract changes, existing
certifications become stale. You cannot certify under weak rules and push under
strong ones.

## Quick start

Add `.repocert/config.toml` to your repository:

```toml
schema_version = 1

[checks.fmt]
argv = ["cargo", "fmt", "--check"]

[checks.test]
argv = ["cargo", "test"]

[fixers.fmt]
argv = ["cargo", "fmt"]
probe_argv = ["cargo", "fmt", "--check"]

[profiles.default]
checks = ["fmt", "test"]
fixers = ["fmt"]
certify = true
default = true

[certification]
mode = "ssh-signed"

[[certification.trusted_signer]]
name = "local-repocert"
public_key = "ssh-ed25519 AAAA... your-public-key"

[[protected_refs]]
pattern = "refs/heads/main"
profile = "default"

[hooks]
mode = "generated"
```

Then:

```sh
repocert check                      # run checks against the contract
repocert fix                        # run fixers to repair what they can
repocert certify --signing-key ~/.ssh/repocert-signing
repocert status --assert-certified  # verify certification exists
repocert install-hooks              # wire enforcement into git hooks
```

A push to `main` is blocked unless the target commit is certified.
In generated mode, repocert derives the hooks it manages from the contract:
`pre-push` and `update` for protected ref enforcement, plus `pre-commit` and
`pre-merge-commit` when local policy is enabled.

In worktree-based repos, run `repocert install-hooks` once per checkout/worktree.
As of `0.1.1`, generated hook installation is checkout-local so one worktree
cannot hijack another worktree's active hooks.

## Commands

| Command | Purpose |
|---------|---------|
| `validate` | Check that `.repocert/config.toml` is well-formed |
| `check` | Run checks and fixer probes for a profile |
| `fix` | Run fixers to automatically repair violations |
| `certify` | Run all checks; if they pass on a clean worktree, write a certification record |
| `status` | Check whether a commit is certified for a profile |
| `authorize` | Decide whether a ref update should be allowed (designed for git hooks) |
| `install-hooks` | Generate and wire git hooks that enforce the contract |

All commands support `--format json` for machine-readable output.

## Key concepts

**Contract.** The `.repocert/config.toml` file. Defines checks, fixers,
profiles, protected refs, hooks, and local policy. The contract is the single
source of truth for what "acceptable" means in this repository.

**Profile.** A named set of checks and fixers. A repository can have multiple
profiles (e.g., `lint`, `full`, `release`) with different strictness levels.

**Certification.** A local record that a specific commit passed all checks in a
profile, stamped with the contract fingerprint that was in effect.
Certifications are SSH-signed and stored locally in `.git`.

**Contract fingerprint.** A SHA-256 hash of the contract and its protected
files. If the contract changes, the fingerprint changes, and existing
certifications are invalidated. This prevents weakening the contract
between certification and push.

**Enforcement.** The `authorize` command checks whether a ref update (push)
targets a protected ref, and if so, whether the target commit is certified
under the required profile with a current contract fingerprint. Enforcement
trusts only valid signed certification records from repo-trusted signer keys.
In signed mode, only the `certified` state satisfies certification or
protected-ref enforcement; other states are diagnostic non-certification states.

**Local policy.** Optional rules enforced during development, such as
preventing direct commits to protected branches or requiring a clean
primary checkout (pushing work toward worktrees).

## What it is not

- A CI replacement
- A build system
- A general policy platform
- An agent supervision runtime

The goal is narrower: the repository itself as the durable steering
mechanism for trusted development and protected ref updates.

## License

MIT
