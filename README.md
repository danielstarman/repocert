# repocert

Certification and enforcement for git repositories.

repocert lets a repository define a contract — checks, fixers, and profiles —
then certify exact commits against that contract and enforce certification
on protected branches. The contract lives in the repo and travels with the code.

## Why

CI catches problems after the push. repocert catches them before.

A certification is a cryptographic record binding a specific commit to a
specific profile under a specific contract. If the contract changes, existing
certifications become stale. You cannot certify under weak rules and push
under strong ones.

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

[[protected_refs]]
pattern = "refs/heads/main"
profile = "default"
```

Then:

```sh
repocert check                      # run checks against the contract
repocert fix                        # run fixers to repair what they can
repocert certify                    # certify HEAD if all checks pass
repocert status --assert-certified  # verify certification exists
repocert install-hooks              # wire enforcement into git hooks
```

A push to `main` is blocked unless the target commit is certified.

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

**Certification.** A record that a specific commit passed all checks in a
profile, stamped with a fingerprint of the contract that was in effect.
Certifications are stored locally in `.git`.

**Contract fingerprint.** A SHA-256 hash of the contract and its protected
files. If the contract changes, the fingerprint changes, and existing
certifications are invalidated. This prevents weakening the contract
between certification and push.

**Enforcement.** The `authorize` command checks whether a ref update (push)
targets a protected ref, and if so, whether the target commit is certified
under the required profile with a current contract fingerprint.

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
