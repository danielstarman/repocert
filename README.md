# repocert

`repocert` is a local-first repository contract, certification, and enforcement system for AI-assisted development.

It is not a CI replacement.
It is not an agent supervision runtime.

It is a repo-owned trust boundary for shell-capable agents and humans.

`repocert` lets repositories define what acceptable local state and acceptable protected-ref updates mean, then evaluate and enforce that contract explicitly.

It lets humans and agents:

- check whether current worktree state satisfies the repo contract
- apply repo-approved automatic fixes
- certify exact commits against repo-defined profiles
- enforce protected ref updates so uncertified commits cannot advance protected branches

## What It Is Not

`repocert` should not drift into:

- a build system
- a CI orchestration layer
- a general policy platform
- a continuous agent runtime or turn supervisor

The goal is narrower and sharper: keep the repository itself as the durable steering mechanism for trusted local development and protected ref updates.

## Status

This repository currently contains the first draft of the product/specification document.

## Draft

- [Initial Spec](docs/spec.md)
