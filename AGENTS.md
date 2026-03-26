# AGENTS.md

## Purpose

This repository builds `repocert`, a repository contract, certification, and enforcement system for AI-assisted development.

## Source Of Truth

1. Code and README
   Source of truth for what repocert is and does.

2. GitHub issues
   Source of truth for planned work, sequencing, and decomposition.

## Design Principles

- Preserve trust semantics over convenience.
- Keep certification non-mutating.
- Keep enforcement tied to repo-defined contracts, not ad hoc behavior.
- Prefer explicit configuration over inference.
- Prefer opaque command orchestration over tool-specific built-ins unless built-in behavior is clearly justified.
- Do not let normal repair flows mutate the contract that defines enforcement.
- Treat human-readable and machine-readable interfaces as equally important.
- Prefer deterministic behavior and stable outputs.
- Prefer elegant, semantically honest designs over forced narrow patches.

## Working Style

- Keep the big picture in mind while implementing local changes.
- If the correct solution is broader than the immediate task, flag it and follow the broader design when justified.
- Do not force implementations into an artificially narrow scope when the surrounding design suggests a better abstraction.
- Refactoring is normal and expected when it improves clarity, ownership, or correctness.
- Leave the code better than you found it.
- Prefer complete changes over temporary compatibility layers.
- Do not add compatibility shims by default.
- When interfaces change, update in-repo callers in the same change when practical.

## Implementation Guidance

- Prefer simple, testable units and explicit data flow.
- Separate core semantics from adapters and integration glue.
- Keep enforcement logic separate from git hook plumbing.
- Keep storage, validation, execution, and presentation concerns distinct.
- Prefer clear, inspectable behavior over cleverness.
- Do not treat passing tests alone as sufficient if intended semantics would be violated.
- Use `mod.rs` as a facade, not as a home for growing implementation logic.
- Avoid dangling root-level internal helpers; prefer small internal namespaces that match the domain they serve.
- When the same semantic concept appears in multiple places, prefer promoting it into a named model over repeating ad hoc field pairs.
- Keep shared mechanism separate from command-specific semantics; reusable execution or integration layers should not live under command-owned modules unless they are truly command-specific.
- When extracting shared behavior, prefer the narrowest honest seam over generic frameworks or “render anything / do anything” abstractions.
- For local dogfooding, prefer invoking the built `repocert` binary directly over `cargo run` when practical. Use `cargo run` when its rebuild behavior is specifically helpful, but prefer direct CLI execution when validating real command behavior or hook-driven workflows.

## Protected Checkout Workflow

- This repo now treats the primary checkout on `main` as protected local workflow space.
- If local policy blocks commits in the primary checkout, create a dedicated worktree/branch for implementation work.
- Do normal development in that worktree, commit there, and run `repocert certify` on the exact worktree `HEAD` using the repo's configured signing flow.
- Bring the certified commit back to `main` via merge or fast-forward instead of developing directly on protected `main`.
- This repo may keep `main` gated by the fast default profile while reserving a stricter `release` profile for `release/*` branches and `v*` release tags when release-only checks like docs builds should not slow down everyday certification.

## Issue Hygiene

- If a PR closes an issue via GitHub automation, rely on that closure.
- If work lands directly on `main`, close the issue explicitly and leave a short note pointing to the landing commit.

## Agent Behavior

- Do not weaken guarantees to make a task easier.
- Do not silently relax validation rules, protected paths, or enforcement semantics.
- When unsure, choose the interpretation that better preserves trust semantics and architectural clarity.
- Surface meaningful ambiguities instead of papering over them.
