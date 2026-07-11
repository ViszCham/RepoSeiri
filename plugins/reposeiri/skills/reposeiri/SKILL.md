---
name: reposeiri
description: Generate RepoSeiri repository audit summaries, dry-run patch plans, Codex review context, and draft PR bodies from the RepoSeiri Rust core. Use when the user asks RepoSeiri to review a repository, prepare a PR body, inspect safe/guarded/manual recommendations, or produce Codex-facing repository organization context.
---

# RepoSeiri Codex Adapter

Use this skill when the user asks for RepoSeiri review context, a RepoSeiri PR draft, or a repository organization review based on RepoSeiri.

## Rules

- Use the Rust core through `cargo run -p seiri-cli`; do not reimplement audit, baseline, profile, planner, calibration, or Codex context logic in the skill.
- Prefer `seiri codex` for Codex-facing output because it bundles audit, dry-run plan, user actions, and PR draft context.
- Keep the default compatibility-v1 context for existing consumers. Use native-v3 bounded queries when typed evidence, documents, governance, patch bindings, argv, remote status, or full wording findings are needed.
- Do not create branches, commit, push, call GitHub, open PRs, apply safe operations, or mutate repository files unless the user explicitly requests that separate action.
- Treat all output as a draft review artifact. Do not claim popularity, trust, security, quality, or external validation guarantees.
- If the user requests a PR body, provide the generated body and state that it is a draft.

## Commands

From the RepoSeiri repository root:

```powershell
cargo run --quiet -p seiri-cli -- codex --path . --profile common --format markdown
```

For JSON context:

```powershell
cargo run --quiet -p seiri-cli -- codex --path . --profile common --format json
```

For PR body only:

```powershell
cargo run --quiet -p seiri-cli -- codex --path . --profile common --view pr-body --format markdown
```

For the native v2 typed context:

```powershell
cargo run --quiet -p seiri-cli -- codex --path . --profile common --schema native-v2 --format json
```

For a bounded query view:

```powershell
cargo run --quiet -p seiri-cli -- codex --path . --profile common --schema native-v3 --view query --query routes --format json
```

For bounded evidence and remote-status views:

```powershell
cargo run --quiet -p seiri-cli -- codex --path . --profile common --schema native-v3 --view query --query evidence --format json
cargo run --quiet -p seiri-cli -- codex --path . --profile common --schema native-v3 --view query --query remote --format markdown
```

For full linter context:

```powershell
cargo run --quiet -p seiri-cli -- codex --path . --profile common --view linter --format markdown
```

Native-v3 query kinds are `summary`, `routes`, `evidence`, `documents`, `governance`, `patches`, `linter`, `actions`, and `remote`. Compatibility-v1 and native-v2 query views support only `summary`, `routes`, `patches`, `linter`, and `actions`; unsupported combinations fail instead of falling back to another schema or view.

Native actions expose `program` plus `args` as review context only. The plugin does not execute them. The `remote` query reports canonical remote-evidence state; the default local audit returns `NotRequested` and does not initiate network access.

Native-v3 is a borrowed, query-first projection over canonical local analysis. It does not retain document source text, write files, execute commands, call GitHub, adopt repository policy, or guarantee popularity, trust, security, quality, or publication readiness.

Available profiles are:

```text
common, library, cli, infra, product, runtime, docs, tutorial, ml, research, template
```

## Response Shape

When reporting results to the user, summarize:

- profile used
- finding count
- safe operation count
- guarded/manual blocked item count
- whether a draft PR body was generated
- verification or command failures, if any
