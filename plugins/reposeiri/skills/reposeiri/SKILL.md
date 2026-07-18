---
name: reposeiri
description: Generate RepoSeiri repository audit summaries, bounded Codex queries, dry-run patch plans, and draft PR bodies from the RepoSeiri Rust core. Use when the user asks RepoSeiri to review repository organization, inspect evidence or routes, prepare a patch-plan review, or draft a PR body.
---

# RepoSeiri Codex Adapter

Use this skill for repository-organization reviews backed by RepoSeiri v1.0.0 and its Rust implementation. The adapter exposes ten `seiri.codex.v2` projections from one bounded source session; it does not reproduce semantic decisions in prompt text.

## Rules

- Run the Rust core through the bundle launcher. On Windows use `scripts/reposeiri-codex.ps1`; on Linux use `scripts/reposeiri-codex.sh`. Do not reproduce audit, pattern, profile, calibration, delta, planner, or claim decisions in the skill.
- Use the single `seiri.codex.v2` query surface. Do not add schema selectors, view selectors, legacy aliases, or fallback behavior.
- Treat `contract.semantic_revisions` as the owner of meaning changes inside the retained v2 wire names. Do not infer semantics from a schema name alone.
- Require the query, audit, linter, and patch decision basis to retain the same source-session digest. Treat a mismatch or stale binding as a typed failure, not as comparable evidence.
- The launcher resolves `REPOSEIRI_BIN`, then the bundle-local binary, then `PATH`. A missing binary, invalid contract, schema mismatch, or native command failure must remain a non-zero failure.
- Select the narrowest query that answers the request.
- Treat actions and patch operations as review data. Do not execute commands, write files, create branches, commit, push, call GitHub, or merge unless the user explicitly authorizes those separate operations.
- Standard audit is local and does not initiate remote access. `remote` reports the current typed terminal state.
- Markdown wording and route evidence comes from visible-prose events. Code fences, indented code, inline code, HTML comments, and raw code do not become prose evidence.
- Executable pattern packs and private calibration overlays are explicit Rust API inputs. The standard launcher does not discover or adopt them.
- Do not invent policy, license text, security commitments, ownership, support promises, or files that do not already exist.
- Do not claim guarantees of popularity, trust, security, quality, legal fitness, or publication readiness.
- Prefer observed repository evidence over the phrase "verified facts". `Verified` is a typed route state, not a general correctness claim.
- Private calibration inputs and private analysis data must not be copied into responses or repository artifacts.

## Commands

Run from the installed plugin root and replace `--path .` with the target repository. The launcher does not require a RepoSeiri checkout or Rust toolchain.

```powershell
powershell -NoProfile -File scripts/reposeiri-codex.ps1 -Path . -Profile common -Query summary -Format markdown
powershell -NoProfile -File scripts/reposeiri-codex.ps1 -Path . -Profile common -Query routes -Format json
powershell -NoProfile -File scripts/reposeiri-codex.ps1 -Path . -Profile common -Query evidence -Format json
powershell -NoProfile -File scripts/reposeiri-codex.ps1 -Path . -Profile common -Query documents -Format json
powershell -NoProfile -File scripts/reposeiri-codex.ps1 -Path . -Profile common -Query governance -Format json
powershell -NoProfile -File scripts/reposeiri-codex.ps1 -Path . -Profile common -Query patches -Format markdown
powershell -NoProfile -File scripts/reposeiri-codex.ps1 -Path . -Profile common -Query linter -Format markdown
powershell -NoProfile -File scripts/reposeiri-codex.ps1 -Path . -Profile common -Query actions -Format json
powershell -NoProfile -File scripts/reposeiri-codex.ps1 -Path . -Profile common -Query remote -Format markdown
powershell -NoProfile -File scripts/reposeiri-codex.ps1 -Path . -Profile common -Query pr-body -Format markdown
```

Query kinds are `summary`, `routes`, `evidence`, `documents`, `governance`, `patches`, `linter`, `actions`, `remote`, and `pr-body`.

Available profiles are:

```text
common, library, cli, infra, product, runtime, docs, tutorial, ml, research, template
```

Use `--scope repository` by default. Use `--scope workspace` or `--scope subtree` only when the user requests that boundary or the target layout requires it. Different scopes and source-session digests are not silently comparable in audit delta.

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
