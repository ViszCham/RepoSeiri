---
name: reposeiri
description: Generate RepoSeiri repository audit summaries, bounded Codex queries, dry-run patch plans, and draft PR bodies from the RepoSeiri Rust core. Use when the user asks RepoSeiri to review repository organization, inspect evidence or routes, prepare a patch-plan review, or draft a PR body.
---

# RepoSeiri Codex Adapter

Use this skill for repository organization reviews backed by the RepoSeiri Rust implementation.

## Rules

- Run the Rust core through `cargo run --quiet -p seiri-cli`; do not reproduce audit, pattern, profile, calibration, delta, planner, or claim decisions in the skill.
- Use the single `seiri.codex.v1` query surface. Do not add schema selectors, view selectors, or fallback behavior.
- Select the narrowest query that answers the request.
- Treat actions and patch operations as review data. Do not execute commands, write files, create branches, commit, push, call GitHub, or merge unless the user explicitly authorizes those separate operations.
- Standard audit is local and does not initiate remote access. `remote` reports the current typed terminal state.
- Do not invent policy, license text, security commitments, ownership, support promises, or files that do not already exist.
- Do not claim guarantees of popularity, trust, security, quality, legal fitness, or publication readiness.
- Private calibration inputs and private analysis data must not be copied into responses or repository artifacts.

## Commands

Run from the RepoSeiri repository root or replace `--path .` with the target repository.

```powershell
cargo run --quiet -p seiri-cli -- codex --path . --profile common --query summary --format markdown
cargo run --quiet -p seiri-cli -- codex --path . --profile common --query routes --format json
cargo run --quiet -p seiri-cli -- codex --path . --profile common --query evidence --format json
cargo run --quiet -p seiri-cli -- codex --path . --profile common --query documents --format json
cargo run --quiet -p seiri-cli -- codex --path . --profile common --query governance --format json
cargo run --quiet -p seiri-cli -- codex --path . --profile common --query patches --format markdown
cargo run --quiet -p seiri-cli -- codex --path . --profile common --query linter --format markdown
cargo run --quiet -p seiri-cli -- codex --path . --profile common --query actions --format json
cargo run --quiet -p seiri-cli -- codex --path . --profile common --query remote --format markdown
cargo run --quiet -p seiri-cli -- codex --path . --profile common --query pr-body --format markdown
```

Query kinds are `summary`, `routes`, `evidence`, `documents`, `governance`, `patches`, `linter`, `actions`, `remote`, and `pr-body`.

Available profiles are:

```text
common, library, cli, infra, product, runtime, docs, tutorial, ml, research, template
```

Use `--scope repository` by default. Use `--scope workspace` or `--scope subtree` only when the user requests that boundary or the target layout requires it. Different scopes are not silently comparable in audit delta.

## Query Selection

| Request | Query |
| --- | --- |
| concise orientation | `summary` |
| README and root route state | `routes` |
| typed facts and coverage | `evidence` |
| bounded Markdown and GitHub-local parse results | `documents` |
| facets, content slots, consistency, scope, freshness | `governance` |
| existing-target-only dry-run edits and holds | `patches` |
| evidence-scoped wording findings | `linter` |
| typed program/argv suggestions | `actions` |
| opt-in remote terminal state | `remote` |
| draft pull request body | `pr-body` |

## Response Shape

Report only the high-signal result:

- profile and scope used
- requested query
- relevant finding, route, evidence, operation, and hold counts
- any `Unknown`, conflict, stale binding, or manual-policy boundary
- whether a PR body was generated as a draft
- verification or command failures
