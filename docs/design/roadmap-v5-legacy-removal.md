# Roadmap v5: Legacy Removal

## 日本語

### 1. 状態と決定

Roadmap v5 は RepoSeiri 0.2.0 の破壊的整理を固定する実装記録です。目的は、同じ観測を複数の schema、DTO、planner、Codex view へ複製していた構造を廃止し、bounded local observation から一つの canonical analysis を組み立てることです。

この変更は互換 layer を追加する計画ではありません。0.1 系の出力を維持する feature、fallback、serde alias、silent conversion は設けません。外部 consumer が存在する場合は、0.2.0 の `seiri.analysis.v1`、`seiri.patch-plan.v1`、`seiri.codex.v1` へ明示的に移行する必要があります。

実装対象は local workspace です。private calibration の内容、private analysis source、token、remote credential を public report、fixture、docs、commit へ移しません。GitHub 操作は Rust core の権限ではなく、人間が別途与える操作権限です。

### 2. 先に置く批判

旧設計には機能の多さではなく、意味の所有者が多すぎる問題がありました。

| 問題 | 具体的な損失 | v5 の判断 |
| --- | --- | --- |
| evidence の二重生成 | path、span、kind、ID の再構成で provenance がずれる | scanner event から typed draft を直接作り、一つの kernel だけを構築する |
| route state の重複保存 | root structure、README route、target reachability と summary enum が不一致になり得る | typed `RouteAssessment` を正とし、`RouteState` は局所的な表示 projection に限定する |
| analysis 本体の直接 Serialize | private field や内部 field が追加時に wire へ漏れ得る | `RepositoryAnalysis` は Serialize せず、borrowed `AuditWire` を明示する |
| planner 世代の並存 | 同じ候補が異なる safety rule で出力される | `PatchPlan` と `plan_patches` を一つにする |
| Codex schema/view matrix | query が schema ごとに欠落し、fallback の意味が曖昧になる | `CodexQueryKind` と borrowed `CodexView` を一つにする |
| serde の寛容な旧入力受理 | 未知 field が捨てられ、移行失敗が成功に見える | public wire input は必要箇所で `deny_unknown_fields` と version check を使う |
| roadmap の累積 | 実装済み判断と将来案が同時に正として読まれる | v5 を唯一の実装 roadmap とし、過去 roadmap を削除する |
| report の過剰簡略化 | legacy を消す際に scope、freshness、claim provenance まで失う | canonical data から有用な表示を復元し、旧 DTO は復元しない |

名前を消すだけでは不十分です。producer、decision owner、wire owner、error owner、verification owner が一つになって初めて削除完了とします。また、低レイヤ化は byte parser を無目的に増やすことではありません。filesystem boundary、UTF-8、byte span、digest、ID、bounded allocation、stale check のように、正確性と資源境界を型で強制できる場所へ限定します。

### 3. 固定アーキテクチャ

```text
bounded filesystem / Git-local scan
  -> DocumentIndex + typed DocumentEvent
  -> EvidenceDraft
  -> EvidenceKernel
  -> RouteAssessment + RouteContentReport + Facet/Consistency reports
  -> RepositoryAnalysis (non-serializable owner)
       -> AuditWire<'a>
       -> PortableAuditSnapshot -> AuditDeltaReport
       -> PatchPlan
       -> CodexView<'a> -> one requested query
```

依存方向は左から右だけです。renderer、CLI、plugin は repository evidence を再判定しません。remote adapter は opt-in であり、標準 audit は network を開始しません。calibration は明示的 provider として注入し、標準 analysis の事実集合へ private 値を混ぜません。

### 4. 低レイヤ Rust 契約

#### 4.1 Evidence

```rust
pub struct EvidenceFact {
    pub id: EvidenceId,
    pub atom: EvidenceAtom,
    pub provenance: EvidenceProvenance,
    pub confidence: EvidenceConfidence,
}

pub struct EvidenceProvenance {
    pub domain: SourceDomain,
    pub producer: EvidenceProducer,
    pub document: Option<DocumentId>,
    pub span: Option<EvidenceSourceSpan>,
}
```

- `EvidenceId` は sorted input から 1 始まりで連続し、同じ入力では決定的です。
- document path は document table に一度だけ保持し、fact は `DocumentId` を参照します。
- Markdown atom は byte span を必須とし、file-presence atom は span を持ちません。
- producer と atom の不一致、inverted span、`u32` に収まらない offset は typed error です。
- source text は kernel に保持しません。

#### 4.2 Analysis と wire

```rust
pub struct RepositoryAnalysis {
    pub evidence_kernel: EvidenceKernel,
    pub route_assessments: Vec<RouteAssessment>,
    pub route_content: RouteContentReport,
    // canonical owned reports only
}

#[derive(Serialize)]
struct AuditWire<'a> {
    schema_version: &'static str,
    evidence_kernel: &'a EvidenceKernel,
    route_assessments: &'a [RouteAssessment],
    route_content: &'a RouteContentReport,
}
```

- analysis 本体は `Serialize` / `Deserialize` を実装しません。
- wire field は明示的 allow-list です。内部 field の追加だけで公開面は変化しません。
- canonical JSON は document index、coverage、facets、consistency、scope、freshness、remote terminal state を含みます。
- private calibration body、local path、token、source text は含みません。

#### 4.3 Route と content

- route の正は root structured presence、inherited presence、README routing、target reachability、conflict、freshness、policy、evidence group を分離した assessment です。
- `Absent` は対象 coverage が `Complete` の場合だけ導出します。partial、invalid UTF-8、limit exceeded、unsupported syntax は `Unknown` です。
- content は一つの `RouteContentReport` にある typed slot assessment で表します。
- slot の `Present` は adequacy、correctness、policy adoption、runtime success を意味しません。
- 日本語と英語の構造 pair は同じ target を持つ候補であり、翻訳同値を意味しません。

#### 4.4 Delta

```rust
fn route_signal_change(before: &PortableRouteRecord, after: &PortableRouteRecord)
    -> (bool, bool);
```

- portable snapshot は source text と private 値を持たず、typed dimensions と SHA-256 comparison guard を持ちます。
- scope、analysis configuration、schema が異なる比較は `Unknown` であり、regression を断定しません。
- README route、root structure、inherited evidence、local target の信号損失だけがあり増加がない場合は `Removed` です。
- digest は署名、真正性、正しさの証明ではありません。

#### 4.5 Patch planner

```rust
pub fn plan_patches(analysis: &RepositoryAnalysis) -> PatchPlan;
```

- planner は既に存在する repository-local target への README link だけを候補にします。
- policy、license text、security SLA、ownership、存在しない file は発明しません。
- proposal は scanner が読んだ base digest、encoding、EOL、byte span、anchor context に bind されます。
- current bytes が変わった場合は preflight が hold/reject し、stale proposal を ready にしません。
- `writes_files` は常に false です。適用、commit、push、merge は planner の責務ではありません。

#### 4.6 Codex surface

```rust
pub enum CodexQueryKind {
    Summary, Routes, Evidence, Documents, Governance,
    Patches, Linter, Actions, Remote, PrBody,
}

pub struct CodexView<'a> {
    analysis: &'a RepositoryAnalysis,
    plan: &'a PatchPlan,
    wording_lint: Option<&'a WordingLintReport>,
}
```

- schema は `seiri.codex.v1` だけです。
- CLI は `codex --query <kind>` だけを公開し、schema/view fallback を持ちません。
- query は必要な collection を borrow し、巨大な canonical report を query ごとに clone しません。
- action は `program` と `args` の typed data であり、plugin は自動実行しません。

### 5. Error と unsafe の境界

| Layer | Error owner | Fail-closed rule |
| --- | --- | --- |
| filesystem | `seiri-fs` | root escape、entry/byte budget、permission、symlink 状態を typed result にする |
| Markdown | `seiri-markdown` | invalid UTF-8 と parser limit を absence に変換しない |
| Git-local | `seiri-git-local` | command 実行なしで bounded header/ref 読み取りを行う |
| GitHub-local | `seiri-github-local` | YAML dynamic value を実行・展開せず unknown として保持する |
| evidence | `seiri-core` | invalid producer/atom/span shape を kernel に入れない |
| calibration | `seiri-calibration` | schema mismatch、counter overflow、stream limit を typed error にする |
| planner | `seiri-planner` | stale base、conflict、unknown relation、missing target を hold/reject にする |
| rendering | `seiri-report` / `seiri-codex` | canonical decision を再計算せず、allow-list projection だけを行う |

workspace の production code に `unsafe` を導入しません。将来 unavoidable な FFI や memory mapping を導入する場合は、別 ADR、最小 private module、safety invariant、Miri/targeted test、benchmark evidence を先に要求します。低レイヤという理由だけで `unsafe` を許可しません。

### 6. 実装ブロック

#### Block LR0: Critical Freeze / 0.2.0 Boundary

- 0.2.0 を breaking release とする。
- canonical schema 名と削除対象を固定する。
- private data 非公開、standard audit network-off、planner dry-run、GitHub authority 分離を不変条件にする。

完了条件: workspace version が 0.2.0、roadmap が日英同内容、互換 feature が存在しない。

#### Block LR1: Evidence Producer Collapse

- scanner/document event から `EvidenceDraft` を直接生成する。
- document table、contiguous ID、producer/atom/span validation を kernel に集約する。
- line-number prose の再parse と二次 evidence ledger を削除する。

完了条件: canonical evidence invariant test が決定性と invalid shape rejection を検証し、decision code が一つの kernel だけを読む。

#### Block LR2: Route / Content Ownership

- typed `RouteAssessment` を route decision owner にする。
- route summary は render 時の derived projection に限定する。
- content contract を一つの report に統合し、facet condition と coverage を保持する。
- root-only pattern は path predicate で制約し、nested file を root policy として数えない。

完了条件: nested LICENSE fixture が root license を満たさず、partial coverage が absence を生成せず、route/content wire が一つずつである。

#### Block LR3: Delta / Planner Consolidation

- portable route record を typed dimensions から構築する。
- mixed signal、partial coverage、configuration mismatch の分類を固定する。
- planner API と plan schema を一つにし、existing-target-only と stale binding を強制する。

完了条件: route removal/addition、partial unknown、scope mismatch、stale base、paired-language hold、conflict hold が regression test を通る。

#### Block LR4: Report / Codex / CLI / Plugin Collapse

- explicit `AuditWire`、一つの Markdown renderer、一つの `CodexView` を使う。
- 10 query を CLI と plugin で同じ名称にする。
- obsolete schema/view flags、wrapper script parameter、CI artifact 名を削除する。
- scope、freshness、structured GitHub semantics、claim provenance は canonical report に残す。

完了条件: 全 query が JSON/Markdown で動き、削除済み CLI flag は non-zero、plugin script は canonical argv だけを渡す。

#### Block LR5: Strict Wire / Calibration Boundary

- analysis、evidence、patch、Codex、calibration の schema owner を分離する。
- calibration run の resource trace を必須にし、materialized/streaming mode を意味で命名する。
- 旧 field を unknown-field ignore で受理しない。
- local/private calibration の identity は redacted configuration だけへ投影する。

完了条件:旧 key、旧 enum value、trace 欠落、schema mismatch が error になり、private sentinel/path/value が JSON、Markdown、Debug、Codex output に出ない。

#### Block LR6: Regression / Documentation Replacement

- 世代番号・block 番号中心の test file を behavior 名へ変更する。
- compatibility golden を削除し、canonical invariant、negative removed-surface test、privacy test を残す。
- 旧 roadmap を削除し、README、docs topology、self-audit、publication checklist、plugin skill、CI を更新する。
- Japanese-first / English-second の主要文書を同じ判断にする。

完了条件: active docs の command が実際の CLI help と一致し、リンク切れがなく、旧 roadmap が design index に残らない。

#### Block LR7: Verification / Git Integration

```powershell
cargo fmt --all -- --check
cargo test --workspace --no-fail-fast
cargo clippy --workspace --all-targets -- -D warnings
cargo +1.76.0 check --workspace --all-targets --locked
cargo run --quiet -p seiri-cli -- audit --path . --profile library --format json
cargo run --quiet -p seiri-cli -- plan --path . --profile library --format json
cargo run --quiet -p seiri-cli -- codex --path . --profile library --query summary --format json
cargo run --quiet -p seiri-cli -- codex --path . --profile library --query evidence --format json
cargo run --quiet -p seiri-cli -- codex --path . --profile library --query linter --format markdown
git diff --check
```

- 同じ input の audit/Codex output を2回生成し、決定性を確認する。
- tracked text の secret pattern と private-analysis marker を検査する。
- RepoSeiri self-audit の finding、hold、manual boundary を人間が確認する。
- branch を commit/push し、PR の CI を確認してから main へ merge する。

完了条件: format、test、clippy、MSRV、self-audit、secret scan、diff check、PR checks が成功し、main が merge commit を含み、worktree が clean である。

### 7. 削除対象と canonical replacement

| Removed surface | Canonical replacement |
| --- | --- |
| duplicate evidence ledger and versioned kernel | `EvidenceKernel` |
| serialized route-state collection | `Vec<RouteAssessment>` plus derived summary |
| versioned route-content report | `RouteContentReport` |
| multiple planner generations and versioned plan command | `PatchPlan`, `plan_patches`, `seiri plan` |
| compatibility/native Codex schemas and view selector | `seiri.codex.v1`, `codex --query` |
| direct serialization of the analysis owner | borrowed `AuditWire` |
| observation-count serde aliases | typed `AggregateRepositoryEstimate` with strict fields |
| historical implementation roadmaps | this Roadmap v5 |

### 8. Final acceptance

v5 は、単に旧 symbol が検索に出ないことではなく、次を同時に満たしたとき完了です。

1. 一つの事実に一つの canonical owner がある。
2. absence、present、conflict、unknown が coverage と evidence から型付きで導出される。
3. renderer、CLI、plugin が判断を再実装しない。
4. public wire は explicit、strict、versioned である。
5. planner は既存 target、current bytes、dry-run 境界を破らない。
6. private calibration と credential が public artifact に入らない。
7. Rust 1.76、clippy `-D warnings`、全 regression、self-audit が通る。
8. README と docs は現在のアプリ、command、境界を説明し、過去設計を正として案内しない。

---

## English

### 1. Status And Decision

Roadmap v5 is the frozen implementation record for the breaking RepoSeiri 0.2.0 cleanup. Its purpose is to remove the structure that duplicated the same observation across multiple schemas, DTOs, planners, and Codex views, and to build one canonical analysis from bounded local observations.

This is not a plan to add another compatibility layer. There is no feature, fallback, serde alias, or silent conversion that preserves 0.1 output. Any external consumer must explicitly migrate to the 0.2.0 `seiri.analysis.v1`, `seiri.patch-plan.v1`, and `seiri.codex.v1` contracts.

Implementation is scoped to the local workspace. Private calibration content, private analysis sources, tokens, and remote credentials must not move into public reports, fixtures, docs, or commits. GitHub operations are human-granted operational authority, not authority owned by the Rust core.

### 2. Critique Before Implementation

The central problem in the previous design was not the number of features. It was the number of owners for the same meaning.

| Problem | Concrete loss | v5 decision |
| --- | --- | --- |
| Duplicate evidence production | Reconstructing paths, spans, kinds, and IDs could drift provenance | Build typed drafts directly from scanner events and construct one kernel |
| Stored route-state duplication | Root structure, README routing, target reachability, and a summary enum could disagree | Make typed `RouteAssessment` canonical and limit `RouteState` to local display projection |
| Direct serialization of the analysis owner | New private or internal fields could leak onto the wire | Keep `RepositoryAnalysis` non-serializable and define a borrowed `AuditWire` |
| Multiple planner generations | The same candidate could be emitted under different safety rules | Keep one `PatchPlan` and one `plan_patches` function |
| Codex schema/view matrix | Queries differed by schema and fallback meaning was ambiguous | Keep one `CodexQueryKind` and one borrowed `CodexView` |
| Permissive old-input serde behavior | Unknown fields were discarded, making migration failures look successful | Use `deny_unknown_fields` and version checks where public wire input requires them |
| Accumulated roadmaps | Implemented decisions and future plans were simultaneously presented as authoritative | Make v5 the only implementation roadmap and delete prior roadmaps |
| Over-simplified reporting | Removing old output also removed scope, freshness, and claim provenance | Restore useful views from canonical data without restoring old DTOs |

Deleting names alone is insufficient. Removal is complete only when each producer, decision owner, wire owner, error owner, and verification owner is singular. Low-level work also does not mean adding byte parsers without purpose. It is limited to boundaries where types can enforce correctness and resource limits: filesystem boundaries, UTF-8, byte spans, digests, IDs, bounded allocation, and stale checks.

### 3. Frozen Architecture

```text
bounded filesystem / Git-local scan
  -> DocumentIndex + typed DocumentEvent
  -> EvidenceDraft
  -> EvidenceKernel
  -> RouteAssessment + RouteContentReport + Facet/Consistency reports
  -> RepositoryAnalysis (non-serializable owner)
       -> AuditWire<'a>
       -> PortableAuditSnapshot -> AuditDeltaReport
       -> PatchPlan
       -> CodexView<'a> -> one requested query
```

Dependencies flow only from left to right. Renderers, the CLI, and the plugin do not re-decide repository evidence. The remote adapter is opt-in, and standard audit does not start network access. Calibration is injected through an explicit provider and does not mix private values into the standard analysis fact set.

### 4. Low-Level Rust Contracts

#### 4.1 Evidence

```rust
pub struct EvidenceFact {
    pub id: EvidenceId,
    pub atom: EvidenceAtom,
    pub provenance: EvidenceProvenance,
    pub confidence: EvidenceConfidence,
}

pub struct EvidenceProvenance {
    pub domain: SourceDomain,
    pub producer: EvidenceProducer,
    pub document: Option<DocumentId>,
    pub span: Option<EvidenceSourceSpan>,
}
```

- `EvidenceId` is contiguous from one over sorted input and deterministic for the same input.
- A document path is stored once in the document table, and facts reference a `DocumentId`.
- Markdown atoms require byte spans; file-presence atoms must not carry spans.
- Producer/atom mismatch, inverted spans, and offsets that do not fit in `u32` are typed errors.
- The kernel does not retain source text.

#### 4.2 Analysis And Wire

```rust
pub struct RepositoryAnalysis {
    pub evidence_kernel: EvidenceKernel,
    pub route_assessments: Vec<RouteAssessment>,
    pub route_content: RouteContentReport,
    // canonical owned reports only
}

#[derive(Serialize)]
struct AuditWire<'a> {
    schema_version: &'static str,
    evidence_kernel: &'a EvidenceKernel,
    route_assessments: &'a [RouteAssessment],
    route_content: &'a RouteContentReport,
}
```

- The analysis owner does not implement `Serialize` or `Deserialize`.
- Wire fields are an explicit allow-list. Adding an internal field alone cannot change public output.
- Canonical JSON includes the document index, coverage, facets, consistency, scope, freshness, and remote terminal state.
- It excludes private calibration bodies, local private paths, tokens, and source text.

#### 4.3 Routes And Content

- The canonical route representation separates root structured presence, inherited presence, README routing, target reachability, conflicts, freshness, policy, and evidence groups.
- `Absent` is derived only when relevant coverage is `Complete`. Partial scans, invalid UTF-8, exceeded limits, and unsupported syntax remain `Unknown`.
- Content is represented by typed slot assessments in one `RouteContentReport`.
- A `Present` slot does not mean adequacy, correctness, policy adoption, or runtime success.
- A Japanese/English structural pair with the same targets is a candidate relationship, not proof of translation equivalence.

#### 4.4 Delta

```rust
fn route_signal_change(before: &PortableRouteRecord, after: &PortableRouteRecord)
    -> (bool, bool);
```

- Portable snapshots exclude source text and private values and contain typed dimensions plus SHA-256 comparison guards.
- Comparisons with different scopes, analysis configurations, or schemas are `Unknown`; they do not assert regressions.
- A route is `Removed` when README routing, root structure, inherited evidence, or local-target signals only decrease and none increase.
- Digests are not signatures or proof of authenticity or correctness.

#### 4.5 Patch Planner

```rust
pub fn plan_patches(analysis: &RepositoryAnalysis) -> PatchPlan;
```

- The planner proposes only README links to repository-local targets that already exist.
- It does not invent policy, license text, security SLAs, ownership, or missing files.
- A proposal is bound to the scanner base digest, encoding, EOL, byte spans, and anchor context.
- If current bytes changed, preflight holds or rejects the proposal and never marks a stale proposal ready.
- `writes_files` is always false. Apply, commit, push, and merge are outside planner ownership.

#### 4.6 Codex Surface

```rust
pub enum CodexQueryKind {
    Summary, Routes, Evidence, Documents, Governance,
    Patches, Linter, Actions, Remote, PrBody,
}

pub struct CodexView<'a> {
    analysis: &'a RepositoryAnalysis,
    plan: &'a PatchPlan,
    wording_lint: Option<&'a WordingLintReport>,
}
```

- `seiri.codex.v1` is the only schema.
- The CLI exposes only `codex --query <kind>` and has no schema/view fallback.
- Queries borrow required collections and do not clone the full canonical report for every request.
- Actions are typed `program` and `args` data. The plugin does not execute them automatically.

### 5. Error And Unsafe Boundaries

| Layer | Error owner | Fail-closed rule |
| --- | --- | --- |
| filesystem | `seiri-fs` | Represent root escape, entry/byte budgets, permission, and symlink states as typed results |
| Markdown | `seiri-markdown` | Never convert invalid UTF-8 or parser limits into absence |
| Git-local | `seiri-git-local` | Read bounded headers and refs without executing commands |
| GitHub-local | `seiri-github-local` | Preserve dynamic YAML values as unknown without executing or expanding them |
| evidence | `seiri-core` | Reject invalid producer/atom/span shapes before they enter the kernel |
| calibration | `seiri-calibration` | Return typed errors for schema mismatch, counter overflow, and streaming limits |
| planner | `seiri-planner` | Hold or reject stale bases, conflicts, unknown relations, and missing targets |
| rendering | `seiri-report` / `seiri-codex` | Project allow-listed canonical decisions without recomputing them |

Production workspace code introduces no `unsafe`. Any future unavoidable FFI or memory mapping requires a separate ADR, a minimal private module, safety invariants, Miri or targeted tests, and benchmark evidence first. Being low-level is not sufficient justification for `unsafe`.

### 6. Implementation Blocks

#### Block LR0: Critical Freeze / 0.2.0 Boundary

- Declare 0.2.0 a breaking release.
- Freeze canonical schema names and removal targets.
- Make private-data non-publication, network-off standard audit, dry-run planning, and separate GitHub authority invariants.

Completion: workspace version is 0.2.0, the roadmap has equivalent Japanese and English content, and no compatibility feature exists.

#### Block LR1: Evidence Producer Collapse

- Generate `EvidenceDraft` directly from scanner/document events.
- Centralize document tables, contiguous IDs, and producer/atom/span validation in the kernel.
- Remove reparsing of prose line numbers and the secondary evidence ledger.

Completion: canonical evidence tests verify determinism and invalid-shape rejection, and decision code reads only one kernel.

#### Block LR2: Route / Content Ownership

- Make typed `RouteAssessment` the route decision owner.
- Limit route summaries to derived render-time projections.
- Consolidate content contracts into one report while retaining facet conditions and coverage.
- Constrain root-only patterns with path predicates so nested files do not count as root policy.

Completion: a nested LICENSE fixture does not satisfy the root license pattern, partial coverage does not emit absence, and there is one route/content wire each.

#### Block LR3: Delta / Planner Consolidation

- Build portable route records from typed dimensions.
- Freeze classification of mixed signals, partial coverage, and configuration mismatch.
- Keep one planner API and schema and enforce existing-target-only and stale binding.

Completion: route removal/addition, partial unknown, scope mismatch, stale base, paired-language hold, and conflict hold regression tests pass.

#### Block LR4: Report / Codex / CLI / Plugin Collapse

- Use an explicit `AuditWire`, one Markdown renderer, and one `CodexView`.
- Use the same names for all ten queries in the CLI and plugin.
- Remove obsolete schema/view flags, wrapper parameters, and CI artifact names.
- Preserve scope, freshness, structured GitHub semantics, and claim provenance in canonical reports.

Completion: every query works in JSON and Markdown, removed CLI flags return non-zero, and the plugin script passes canonical argv only.

#### Block LR5: Strict Wire / Calibration Boundary

- Separate schema ownership for analysis, evidence, patching, Codex, and calibration.
- Require resource traces in calibration runs and name materialized/streaming modes by meaning.
- Do not accept removed fields through unknown-field ignoring.
- Project local/private calibration identity only through redacted configuration.

Completion: removed keys, removed enum values, missing traces, and schema mismatch fail; private sentinels, paths, and values do not appear in JSON, Markdown, Debug, or Codex output.

#### Block LR6: Regression / Documentation Replacement

- Rename generation- and block-number-oriented tests by behavior.
- Delete compatibility goldens while retaining canonical invariants, negative removed-surface tests, and privacy tests.
- Delete prior roadmaps and update README, docs topology, self-audit, publication checklist, plugin skill, and CI.
- Keep major documents equivalent in Japanese-first and English-second form.

Completion: active doc commands match actual CLI help, links resolve, and prior roadmaps are absent from the design index.

#### Block LR7: Verification / Git Integration

```powershell
cargo fmt --all -- --check
cargo test --workspace --no-fail-fast
cargo clippy --workspace --all-targets -- -D warnings
cargo +1.76.0 check --workspace --all-targets --locked
cargo run --quiet -p seiri-cli -- audit --path . --profile library --format json
cargo run --quiet -p seiri-cli -- plan --path . --profile library --format json
cargo run --quiet -p seiri-cli -- codex --path . --profile library --query summary --format json
cargo run --quiet -p seiri-cli -- codex --path . --profile library --query evidence --format json
cargo run --quiet -p seiri-cli -- codex --path . --profile library --query linter --format markdown
git diff --check
```

- Generate audit/Codex output twice for the same input and verify determinism.
- Scan tracked text for secret patterns and private-analysis markers.
- Manually inspect RepoSeiri self-audit findings, holds, and manual boundaries.
- Commit and push the branch, inspect PR CI, and merge into main.

Completion: formatting, tests, clippy, MSRV, self-audit, secret scan, diff check, and PR checks succeed; main contains the merge commit and the worktree is clean.

### 7. Removed Targets And Canonical Replacements

| Removed surface | Canonical replacement |
| --- | --- |
| duplicate evidence ledger and versioned kernel | `EvidenceKernel` |
| serialized route-state collection | `Vec<RouteAssessment>` plus derived summary |
| versioned route-content report | `RouteContentReport` |
| multiple planner generations and versioned plan command | `PatchPlan`, `plan_patches`, `seiri plan` |
| compatibility/native Codex schemas and view selector | `seiri.codex.v1`, `codex --query` |
| direct serialization of the analysis owner | borrowed `AuditWire` |
| observation-count serde aliases | typed `AggregateRepositoryEstimate` with strict fields |
| historical implementation roadmaps | this Roadmap v5 |

### 8. Final Acceptance

v5 is complete only when all of the following are true, not merely when old symbols disappear from search:

1. Each fact has one canonical owner.
2. Absence, presence, conflict, and unknown are derived from typed coverage and evidence.
3. Renderers, the CLI, and the plugin do not reimplement decisions.
4. Public wire contracts are explicit, strict, and versioned.
5. The planner preserves existing-target, current-byte, and dry-run boundaries.
6. Private calibration and credentials never enter public artifacts.
7. Rust 1.76, clippy with `-D warnings`, all regressions, and self-audit pass.
8. README and docs describe the current application, commands, and boundaries without routing readers to prior designs as authoritative.
