# AGENTS.md — Code Agent Collaboration Specification

This document serves as the **high-density, lightweight entry point (attention steering weight)** for all AI coding agents across this repository. Its primary purpose is to eliminate hallucinations, prevent piggybacking changes, and ensure reproducible commits and PRs. Detailed guidelines are split across [`docs/ai/agents/`](docs/ai/agents/) and `.agents/skills/`; **do NOT load all chapters into context by default**.

## Knowledge Tiering & Token Budget Discipline

| Tier | Content | Entry Point |
| --- | --- | --- |
| **Standing Constraints** | Inviolable rules across all tasks | This file ("Always Active"); expanded in [`docs/ai/agents/`](docs/ai/agents/) |
| **Reusable Workflows** | Domain-specific procedures & validation commands (Token Relief Valve) | `.agents/skills/*/SKILL.md` (Authoritative) |
| **Vendor/Env Bridges** | Tooling or Cloud VM nuances | [`docs/ai/`](docs/ai/) (No root bifurcation allowed) |
| **Domain Specs** | Architecture / Data abstractions | [`docs/architecture.md`](docs/architecture.md) |

- **Token Budget & Zero-Sum Updates**: As a resident system prompt weight, this file has a strict hard limit of **80 lines / 1200 Tokens**. Near the limit, follow the **zero-sum rule (add one, remove one)**.
- **Anti-Anecdote & Batch Threshold**: Never add global rules based on isolated single-session mistakes. Rules must appear in **≥ 2 independent session transcripts** and be refined via skill [`promote-lesson`](.agents/skills/promote-lesson/SKILL.md) with explicit human review.

## Agent Reading Map

| Task Signal | Required Reading |
| --- | --- |
| `git stash` operations | skill [`git-stash-safe`](.agents/skills/git-stash-safe/SKILL.md) |
| Adding SQL migrations (`migrations/NNN_*.sql`) | skill [`add-sql-migration`](.agents/skills/add-sql-migration/SKILL.md) |
| Writing design docs in `docs/` / DDL / Mermaid | skill [`verify-design-doc`](.agents/skills/verify-design-doc/SKILL.md) |
| Release / tagging / production deployment | skill [`release`](.agents/skills/release/SKILL.md) |
| Code review / PR audit / acceptance verification | skill [`review`](.agents/skills/review/SKILL.md) (Independent read-only context) |
| Autonomous agent loops / cron tasks | [`loop-charter.md`](docs/ai/agents/loop-charter.md) |
| `tokio::spawn` / daemons / script modifications / exit codes | [`engineering.md`](docs/ai/agents/engineering.md) |
| Commit message conventions | [`commit-style.md`](docs/ai/agents/commit-style.md) |
| Cross-module boundaries / crate splitting / SQL joins | [`module-boundaries.md`](docs/architecture/module-boundaries.md) |

## Always Active (Highest Standing Constraints)

1. **No Piggybacking**: Commits/PRs must not carry unrelated changes; unannounced tuning, repo-wide formatting, undocumented `#[allow]`, and cross-module opportunistic refactoring are strictly prohibited. Violations must be split via `git reset --mixed HEAD~1`.
2. **Zero Hallucination Code**: Every definition must have callers; every cache field must have a store policy; metrics must track both success and failure; every `TODO` must reference an issue. Design docs must never cite skeleton-only features as existing capabilities.
3. **Safe Stash**: Honest stash naming; `git diff --stat` before stash; `cargo check --tests` required after pop; never stash `Cargo.toml`, `Cargo.lock`, or build scripts.
4. **Release Guardrail**: Explain outcomes to users after local closed-loop verification; **merging to main or creating release tags is strictly prohibited without explicit human approval**.
5. **Core Data Plane vs. Plugin Boundary**: Custom business logic (headers, auth decoration, masking, session tracking) **must be implemented as Plugins/Middleware**, never hardcoded into the core data pipeline.
6. **Release Promoter Process**: Tagging and releasing must follow skill [`release`](.agents/skills/release/SKILL.md) (full local gate re-run → multi-point check → human approval hard stop → deployment verification).
7. **Pre-push Local Quality Gate**: Never use CI as a local sandbox; run full local quality gates (fmt, clippy, tests) via skill [`pre-push-local-gates`](.agents/skills/pre-push-local-gates/SKILL.md) before pushing.

## Skills Index

Authoritative skills are located under `.agents/skills/`.

- [`git-stash-safe`](.agents/skills/git-stash-safe/SKILL.md)
- [`add-sql-migration`](.agents/skills/add-sql-migration/SKILL.md)
- [`promote-lesson`](.agents/skills/promote-lesson/SKILL.md)
- [`verify-design-doc`](.agents/skills/verify-design-doc/SKILL.md)
- [`pre-push-local-gates`](.agents/skills/pre-push-local-gates/SKILL.md)
- [`release`](.agents/skills/release/SKILL.md)
- [`review`](.agents/skills/review/SKILL.md)
