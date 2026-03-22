# AGENTS.md

## Purpose

This repository builds `repocert`, a repository contract, certification, and enforcement system for AI-assisted development.

## Source Of Truth

Use these sources for different purposes:

1. `docs/spec.md`
   Initial design document and canonical statement of product intent, semantics, and major architectural decisions.

2. Code
   Source of truth for what is actually implemented today.

3. GitHub issues
   Source of truth for planned work, sequencing, and decomposition.

Do not assume these always match perfectly during active development.

If they diverge:
- do not silently pick whichever is most convenient
- reconcile the mismatch in a way that keeps the project coherent
- do not treat the spec as a document that must be continuously updated for every implementation detail

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

## Agent Behavior

- Do not weaken guarantees to make a task easier.
- Do not silently relax validation rules, protected paths, or enforcement semantics.
- When unsure, choose the interpretation that better preserves trust semantics and architectural clarity.
- Surface meaningful ambiguities instead of papering over them.
