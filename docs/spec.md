# repocert Spec Draft

## Purpose

`repocert` defines a repo-owned contract for AI-assisted development.

It exists because coding agents are useful but unreliable collaborators. They often optimize for local task completion instead of repository-specific engineering discipline, may violate process or architectural preferences, and may weaken constraints like runtime budgets or quality thresholds without approval.

`repocert` makes the repository itself the durable steering mechanism:

- enforceable
- repo-owned
- tied to exact commits rather than vague recent green runs

It is:

- a local-first certification and enforcement system
- callable proactively by humans or agents during development
- able to block protected ref updates unless the exact target commit has been certified

It is not:

- a CI replacement
- a continuous agent supervisor
- a remote branch protection manager in v1

## Audience

Primary audience for v1:

- solo developers
- small teams
- repos supervised by humans while using shell-capable coding agents

Secondary audience:

- repos that want local-first contract enforcement even without active agent usage

Not a v1 target:

- large-organization centralized policy management
- multi-repo governance platforms
- remote-only merge enforcement systems

## Threat Model and Non-Goals

`repocert` v1 is designed to constrain and steer careless, opportunistic, or over-eager automation.

It is explicitly valuable against patterns like:

- agents optimizing for task completion over repo discipline
- agents weakening local constraints such as budgets or thresholds without approval
- agents mutating repo contract files through normal repair flows
- agents advancing protected refs without first satisfying the repo-defined contract

It is not a full defense against a determined actor with unrestricted write access to the repository metadata store.

In particular, v1 does not attempt to defend against:

- direct tampering with git-local certification records
- direct tampering with generated hook state outside ordinary `install-hooks` flows
- hostile shell-capable actors intentionally forging local trust state

Stronger anti-forgery guarantees are future work and may require:

- signed certification records
- external attestation
- privilege separation
- storage outside ordinary git-local writable metadata

## Core Model

- `check` evaluates the current worktree.
- `fix` mutates the current worktree using repo-declared fixers.
- `certify` is non-mutating, requires a clean worktree, evaluates exact `HEAD`, and records certification per `(commit, profile)`.
- enforcement trusts only certified commits, never loose worktree state.

Certification validity requires:

- matching commit SHA
- matching current contract fingerprint

The contract fingerprint is computed from:

- `.repocert/config.toml`
- any additional protected contract paths declared by the repo

Fingerprint computation in v1 is byte-exact and deterministic:

- protected contract paths are ordered by sorted normalized path
- fingerprint input is the exact file bytes of each protected contract file
- any byte-level change invalidates prior certification for that fingerprint

This is intentional and includes:

- comment-only changes
- whitespace-only changes
- formatting-only changes

Tool/framework version may be recorded for diagnostics, but does not invalidate certification by itself.

## Repo Layout

Required:

- `.repocert/config.toml`

Optional:

- repo-owned/versioned hooks
- generated/tool-managed hooks
- repo-local policy or helper scripts anywhere in the repo, referenced from config

Protected by default:

- `.repocert/**`

Additional protected contract paths may be declared in config.

## Command Namespace

The final binary name is TBD, but v1 assumes a single top-level CLI namespace with subcommands:

- `check`
- `fix`
- `certify`
- `status`
- `validate`
- `install-hooks`

Default config discovery walks upward from the current directory. CLI should also support explicit repo/config overrides.

## Profiles

Checks and fixers are declared globally by unique human-readable `name` within their own registry.

Profiles:

- explicitly reference checks
- explicitly reference fixers
- may include other profiles
- must be acyclic
- are flattened and deduped by name

Rules:

- if only one profile exists, it is the implicit default
- otherwise one default profile may be designated
- profiles explicitly declare whether they are certification-eligible
- `certify` may only target certification-eligible profiles
- certification profiles must include at least one check

`check` may run one or more profiles in one invocation.

`certify` may run one or more profiles in one invocation.

`fix` accepts at most one profile in v1.

## Checks

Checks are opaque repo-declared external commands.

The framework does not:

- auto-discover tools
- infer tool scope
- understand tool-specific semantics

Repo authors explicitly declare checks and choose which profiles use them.

Conceptual check categories:

- tool checks
- policy checks
- budget checks
- contract integrity checks
- environment/setup checks

Built-in integrity remains limited to structural contract validation. Repo-defined integrity-like rules are just normal checks.

Behavior:

- `check` runs all selected checks by default
- `check` reports all failures
- `check` does not fail fast
- `check` also supports direct named-check execution

## Fixers and Probes

Fixers are opaque repo-declared mutating commands.

Each fixer may define a paired non-mutating probe as part of its declaration.

If a profile includes a fixer, that fixer must have a probe.

Probe exit semantics:

- `0`: no repair needed
- `1`: repair needed
- `2+`: probe/config/tool failure

Behavior:

- `fix` runs fixers in declared order
- `fix` stops on first failure
- `fix` may run on a dirty worktree
- `fix` supports direct named-fixer execution
- `check` includes selected fixer probes by default
- `certify` runs selected fixer probes and fails if repair is needed

`certify` remains non-mutating and should direct the caller to run `fix` when probes report pending repair.

`fix` accepts at most one profile in v1. This is an intentional simplification to keep mutating execution semantics clear even though multi-profile deduplication would be mechanically feasible.

## Protected Contract Paths

Ordinary `fix` is not allowed to modify protected contract paths.

This is enforced structurally using git-visible file changes.

If a fixer touches a protected contract path:

- `fix` fails immediately

Repo-declared fixers are not exempt.

`check` may run while contract files are dirty.

`fix` may run on a dirty worktree, but only with structurally valid config.

`certify` requires a clean worktree.

## Validation and Preflight

Structural contract validation is built in and includes:

- schema validation
- profile and reference validation
- profile inclusion cycle detection
- protected-ref rule validity
- certifiability constraints
- hook mode config validity
- certification metadata readability

`validate` exposes this structural phase directly.

The same structural validation runs implicitly before:

- `check`
- `fix`
- `certify`
- `status`
- `install-hooks`

## Certification Records

Certification records live in git-local metadata under the git metadata store/common dir, not in the checked-in repo.

If multiple worktrees share a git common dir, shared certification state is valid and expected.

Records are keyed by `(commit, profile)`.

Re-certifying the same pair updates that record.

No pruning or cleanup command is included in v1.

This means certification storage grows linearly with certified `(commit, profile)` pairs. V1 assumes that local-first usage keeps this footprint acceptable. Pruning is deferred, not forgotten.

In multi-profile certification:

- each passing profile is recorded immediately
- later failures do not erase earlier successful profile certifications
- overall command status still fails if any requested profile fails

## Protected Refs and Enforcement

Protected refs and required certification profiles live in `.repocert/config.toml`.

Each protected-ref rule contributes one required profile.

Matching rules are cumulative.

Enforcement requires certification for the union of all matched profiles.

Enforcement is local-first in v1:

- git hooks are the primary integration
- the same enforcement logic must also be invocable directly for testing and scripting

Hook installation supports two valid modes:

- repo-owned/versioned hooks
- generated/tool-managed hooks

`install-hooks` is:

- idempotent
- mode-aware
- the only command that writes or repairs hook files

It requires valid schema and enforcement-related config, but does not require every declared repo command/tool to be runnable on the current machine.

Remote branch protection / PR automation is out of scope for v1. CI can re-run `check` independently.

## Environment, Working Directory, and Budgets

Declared commands run from repo root by default.

Commands inherit the caller environment by default, with config able to add or override environment variables.

Framework-native budget support in v1 is timeout only.

Per-command timeouts are supported.

Timeout is reported as a distinct failure category.

Richer budgets remain repo-owned logic inside declared commands.

## Output

Output is human-first but structured enough for agents to parse from plain text.

Behavior:

- stream live command output
- emit a concise structured summary at the end

Structured output is also first-class in v1:

- major commands should support `--format json`
- JSON output is intended for reliable agent and script consumption
- human-readable output remains the default

Representative result labels:

- `PASS`
- `FAIL`
- `TIMEOUT`
- `REPAIR_NEEDED`

## Status

`status` is included in v1.

It is primarily observational, but should also support assertion-style use via flags.

It defaults to current `HEAD`, but may inspect arbitrary commit/profile pairs when asked.

It should report:

- which profiles current `HEAD` is certified under
- whether certification is stale because of commit mismatch or contract fingerprint mismatch
- what protected refs would currently require
- whether hook installation is valid for the configured mode

## Implementation Plan

Recommended implementation order:

1. `validate`
2. `check`
3. `fix`
4. `certify`
5. `status`
6. `install-hooks`
7. direct enforcement command
8. git hook adapter

Recommended first implementation scope:

- TOML config
- opaque external commands
- exact-byte contract fingerprinting
- git-local certification records
- human-readable output plus `--format json`
- repo-owned hooks first
- generated hooks second

## Open Items

Still intentionally unresolved:

- final implementation language
- final binary/repo branding beyond `repocert`
- exact CLI flag shapes
- exact TOML field names
