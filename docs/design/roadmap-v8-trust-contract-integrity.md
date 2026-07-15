# Roadmap v8: Trust And Contract Integrity

## 日本語

Roadmap v8 は、RepoSeiri 1.0.0 の公開 schema 名を維持しながら、入力境界、意味投影、patch 根拠、資源上限、CLI、completion record を同じ trust contract に接続します。非公開分析本文や exact calibration 値は入力にも成果物にも追加しません。

### 固定境界

- `seiri.analysis.v2`、`seiri.patch-plan.v2`、`seiri.codex.v2`を維持します。
- repository 外 path、symlink escape、case mismatch、Markdown の非表示 context を repository evidence に昇格させません。
- observed claim は evidence を必須とし、意味、boundary、machine projection を canonical owner から導出します。
- patch plan は既存 repository-local target の dry-run に限定し、claim、evidence fingerprint、gate、priority、semantic revision を根拠として持ちます。
- standard audit は network、file write、Git、GitHub、plugin install、restart、visibility change を開始しません。

### 実装 block

| Block | Scope | 完了条件 |
| --- | --- | --- |
| TI0 | baseline / protocol | clean baseline、権限、停止条件、検証 command が固定される |
| TI1 | path containment | absolute、escape、encoded escape、symlink escape、case mismatch が fail closed になる |
| TI2 | Markdown context | fence、HTML comment、inline code、escape が offset を保ったまま route 抽出から除外される |
| TI3 | public path privacy | canonical JSON / Markdown / Codex に host absolute path が出ない |
| TI4 | portable actions | action は `seiri` と argv、runtime resolver order を持ち、Cargo workspace に依存しない |
| TI5 | canonical claims | constructor / Deserialize が evidence、ID、meaning、boundary 不変条件を強制する |
| TI6 | semantic revisions | claim projection と contract manifest が machine-readable revision を公開する |
| TI7 | patch provenance | operation / hold が gate、priority、claim、stable evidence fingerprint を持つ |
| TI8 | resource hardening | JSONL総量、record、string、private pack identity が bounded / checked になる |
| TI9 | facet / Git boundary | fixture path の facet 誤検出と exact-budget の false partial を除く |
| TI10 | CLI / completion v2 | parse error が typed、検証は locked、completion が source digest に結び付く |
| TI11 | schema / docs / regression | JSON Schema、日英 docs、self-audit、全 test / clippy / diff check が整合する |

### 完了判定

完了は同じ worktree に対する `cargo fmt --all -- --check`、`cargo test --workspace --locked`、`cargo clippy --workspace --all-targets --locked -- -D warnings`、MSRV check、self-audit、privacy scan、`git diff --check` の結果で判定します。ローカルで host evidence がない場合、completion は `incomplete` のままです。完了判定は commit、push、merge、release、plugin 再インストールの権限を与えません。

---

## English

Roadmap v8 connects input boundaries, semantic projection, patch provenance, resource limits, the CLI, and completion records into one trust contract while retaining RepoSeiri 1.0.0 public schema names. It adds neither private analysis bodies nor exact calibration values to inputs or artifacts.

### Fixed Boundaries

- Retain `seiri.analysis.v2`, `seiri.patch-plan.v2`, and `seiri.codex.v2`.
- Do not promote outside-repository paths, symlink escapes, case mismatches, or hidden Markdown contexts into repository evidence.
- Require evidence for observed claims and derive meanings, boundaries, and machine projections from canonical owners.
- Limit patch plans to dry-run links to existing repository-local targets and attach claim, evidence-fingerprint, gate, priority, and semantic-revision provenance.
- Standard audit starts no network, file writes, Git, GitHub, plugin installation, restart, or visibility change.

### Implementation Blocks

| Block | Scope | Completion condition |
| --- | --- | --- |
| TI0 | baseline / protocol | Freeze the clean baseline, authority, stop conditions, and verification commands |
| TI1 | path containment | Fail closed on absolute paths, escapes, encoded escapes, symlink escapes, and case mismatches |
| TI2 | Markdown context | Exclude fences, HTML comments, inline code, and escapes while preserving offsets |
| TI3 | public path privacy | Emit no host absolute path in canonical JSON, Markdown, or Codex output |
| TI4 | portable actions | Use `seiri` plus argv and runtime resolver order without a Cargo-workspace dependency |
| TI5 | canonical claims | Enforce evidence, ID, meaning, and boundary invariants in construction and deserialization |
| TI6 | semantic revisions | Publish machine-readable claim projection and contract revisions |
| TI7 | patch provenance | Attach gate, priority, claim, and stable evidence fingerprints to operations and holds |
| TI8 | resource hardening | Bound and check JSONL totals, records, strings, and private-pack identity |
| TI9 | facet / Git boundary | Remove fixture-path facet false positives and exact-budget false partial states |
| TI10 | CLI / completion v2 | Type parse errors, lock verification, and bind completion to a source digest |
| TI11 | schema / docs / regression | Align JSON Schema, bilingual docs, self-audit, tests, clippy, and diff checks |

### Completion Decision

Completion is evaluated against one worktree using `cargo fmt --all -- --check`, `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, the MSRV check, self-audit, privacy scan, and `git diff --check`. Completion remains `incomplete` locally when host evidence is absent. A completion result grants no commit, push, merge, release, or plugin-reinstallation authority.
