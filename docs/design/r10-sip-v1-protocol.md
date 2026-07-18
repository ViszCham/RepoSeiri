# R10-SIP-v1: Continuous Sequential Implementation Protocol

## 日本語

### 1. 目的

R10-SIP-v1 は、一つの明示指示から Roadmap v10 の C0-C8 を内部 slice へ分解し、実装、局所検証、自動修復、backflow、統合検証、最終報告まで対話待ちなしで進める Codex 実行契約です。

「止まらない」とは、通常の compile/test/lint failure、設計上の局所的不足、環境依存 check を理由に途中でユーザー回答を待たないことを意味します。失敗を pass に変えること、無限 retry、private data の公開、ユーザー変更の破壊、権限外操作の実行は意味しません。継続不能な slice は typed blocked state にし、安全に実行可能な残りの slice と最終検証を処理して必ず terminal report へ到達します。

### 2. Trigger

標準 trigger:

```text
R10-SIP-v1でC0-C8を一括実装してください。
```

次の形式も同じ意味です。

```text
R10-SIP-v1を実行してください。
```

trigger は C0-C8 の repository mutation と local verification を許可します。commit、push、merge、release、publication、visibility change、plugin reinstall、Codex restart は許可しません。

### 3. Authority Envelope

| Authority | 既定値 | 範囲 |
| --- | --- | --- |
| AnalysisAuthority | true | repository、tests、docs、local tool state の読取 |
| MutationAuthority | true | Roadmap v10 の source、test、fixture、schema、public docs |
| VerificationAuthority | true | local build、test、lint、audit、fuzz smoke、benchmark smoke |
| LedgerAuthority | true | ignored `target/r10-sip/` metadata |
| CommitAuthority | false | stage / commit |
| PushAuthority | false | remote push |
| MergeAuthority | false | PR merge |
| ReleaseAuthority | false | tag、release、publish |
| VisibilityAuthority | false | public/private 変更 |
| InstallAuthority | false | plugin install / cache update |
| RestartAuthority | false | Codex、app、process restart |

MutationAuthority から別 authority を推測しません。上位の明示指示が同じ turn で個別 authority を与えた場合だけ変更できます。

### 4. 読取順

実行時は次を順に読みます。

1. `AGENTS.md`
2. `docs/design/roadmap-v10-closure-and-product-integrity.md`
3. `docs/design/r10-sip-v1-protocol.md`
4. `docs/design/r10-sip-v1-template.json`
5. current `ContractManifest`、public schemas、completion harness
6. current worktree と user-owned changes

旧 roadmap/protocol は history と比較資料であり、同じ責務では Roadmap v10 と R10-SIP-v1 が優先します。

### 5. Execution Ledger

ledger は `target/r10-sip/<execution-id>/state.json` に保存します。`target/` は Git 管理外です。

保存可能:

- protocol/roadmap/template digest
- base HEAD、branch、upstream、worktree identity、Cargo.lock digest
- unit/slice ID、dependency、owned path digest、state、attempt count
- command ID、argv digest、exit class、duration、bounded output digest
- source-session digest、contract revision set、verification receipt
- failure class、repair/backflow decision、remaining residual

保存禁止:

- source、diff、README、issue、security report の本文
- private analysis の filename または本文
- private calibration body、exact prior、private digest
- host absolute path、username、credential、token、environment secret

### 6. State Machine

execution state:

```text
armed
  -> baselined
  -> expanded
  -> executing
  -> integrating
  -> closure_verifying
  -> ready_for_git
  -> implemented_with_blocked_evidence
  -> incomplete
```

slice state:

```text
pending
  -> in_progress
  -> passed
  -> repairing -> in_progress
  -> backflow_pending -> in_progress
  -> blocked_environment
  -> blocked_authority
  -> blocked_privacy
  -> blocked_conflict
  -> blocked_dependency
  -> superseded
```

同時に `in_progress` にできる slice は一つです。terminal state は必ず ledger と final report に残します。

### 7. A0: Arm And Baseline

1. root、HEAD、branch、upstream、dirty state、toolchain、Cargo.lock、installed plugin version を記録します。
2. existing changes を `user_owned`, `protocol_owned`, `overlap_unknown` に分類し、user-owned diff を revert、stash、overwrite しません。
3. baseline の fmt、targeted test、workspace check、self-audit を可能な範囲で取り、既存 failure と新規 failure を区別します。
4. roadmap/protocol/template digest と authority envelope を ledger に固定します。
5. private-data marker と public absolute-path scan の baseline を取りますが、private data 本文を ledger に保存しません。

baseline failure だけでは対話停止しません。実装が原因か判別できるよう記録して A1 へ進みます。

### 8. A1: Slice Expansion

各 unit を単一 owner と単一主要 contract を持つ slice へ分けます。各 slice は次を持ちます。

```text
id
unit
depends_on
owned_paths
input_contracts
output_contracts
invariants
selected_method_world
rejected_method_worlds
targeted_checks
blocking_checks
repair_budget
rollback_scope
claim_boundary
```

最低 slice:

| Unit | Required slices |
| --- | --- |
| C0 | coverage partition、summary projection、completion predicate、regression |
| C1 | bounded walk、bounded read、source session、projection reuse |
| C2 | digest framing、stable tags、portable identity、FNV migration、property tests |
| C3 | semantic revisions、nested schemas、launcher integrity、migration fixtures |
| C4 | pack compiler、overlay evaluator、private freshness、support tier |
| C5 | shared Markdown IR、wording nodes、route tokens、proposition consistency |
| C6 | README entry、real example、docs authority、claim/version parity |
| C7 | process supervisor、host receipts、CI pins、fuzz wiring、bundle verification |
| C8 | corpus split、metrics/intervals、claim matrix、final completion |

scope expansion は roadmap の不変条件を満たすために必要な場合だけ行い、reason と dependency を先に ledger へ追加します。

### 9. A2: Rust Method-World Gate

各 nontrivial Rust slice は edit 前に少なくとも次を比較します。

1. safe idiomatic Rust
2. newtype / typestate / sealed trait
3. compact IR / interning / bounded arena
4. iterator / streaming / bounded buffer
5. verification-oriented pure kernel
6. unsafe capsule
7. parallel / async / OS-specific backend

不変条件を満たす最小 complexity を選びます。次を強制します。

- `unsafe` は safe alternative で不変条件を保てない場合だけ選び、local safety contract と Miri 対象を付けます。
- persistent/wire bytes は decode/validate 後に型へします。
- concurrency は publication/cancellation semantics を持ちます。
- performance、SIMD、parallel、allocation 改善は paired measurement なしに claim しません。
- prose renderer、CLI glue、docs-only path を低レイヤ化しません。

### 10. A3: Continuous Slice Loop

dependency-ready slice を ID 順に一つ選び、次を実行します。

1. input contract、owned path、直前の source binding を再読します。
2. Labyrinth coding artifact で premise、TranslationLoss、LLMCodingLoss、countermodel、gate を確認します。
3. 最小の coherent diff を実装します。
4. formatter、crate check、targeted test、schema/fixture test を実行します。
5. diff critique を実行し、claim と test の不足を確認します。
6. pass なら ledger receipt を保存して次へ進みます。
7. fail なら A4 へ移り、ユーザー回答を待ちません。

slice ごとの commit、hidden stash、destructive reset、unrelated change の revert は禁止です。

### 11. A4: Automatic Repair And Backflow

repair budget:

- local repair: slice ごとに最大3回
- owner backflow: owner ごとに最大2回
- global repair: 全 execution で最大24回
- 同じ command/error fingerprint の無変更 retry: 1回まで

| Failure class | 自動処理 |
| --- | --- |
| `LocalRepairable` | 同じ slice を修正して targeted checks を再実行 |
| `ContractRegression` | contract owner slice へ backflow |
| `CrossUnitRegression` | 最小 affected unit set を再度 pending にする |
| `SourceDrift` | session を破棄し、同じ worktree から一度だけ再baseline |
| `EnvironmentBlocked` | command/OS error/unexecuted scope を記録し、独立 slice を継続 |
| `AuthorityRequired` | operation を実行せず affected slice を blocked にし、他を継続 |
| `UserOverlap` | user change を保持して三者整合を試み、安全でなければ affected slice を blocked にする |
| `PrivacyBoundary` | protocol-owned leak candidate を public/tracked output から除去し、user-owned source は変更せず対象sliceをblockし、privacy regression後に他を継続 |
| `UnsupportedDesignDecision` |既存 contract に最も保守的な選択を採用できなければ affected slice を blocked にする |
| `NonRepairable` | affected dependency cone を blocked にし、独立 slice と final diagnostics を継続 |

budget exhaustion を pass にしません。blocked slice に依存する slice は `blocked_dependency` にしますが、独立 slice、docs reconciliation、最終 diagnostic checks は続行します。

### 12. A5: Unit Gates

各 unit の最後に Roadmap v10 の unit gate と次を確認します。

| Unit | Blocking evidence |
| --- | --- |
| C0 | fixture/primary coverage separation、Unknown completion negative test |
| C1 | bounded hostile corpus、single-session counter、source drift |
| C2 | insertion/order/run invariance、known-answer digest |
| C3 | nested schema negative tests、revision/launcher parity |
| C4 | executable overlay effect、unsupported/stale/privacy negatives |
| C5 | Markdown dead zones、bilingual token matrix、two-span conflict |
| C6 | real example snapshot、README/docs/CLI/plugin parity |
| C7 | timeout/output flood/kill tests、host binding、CI/fuzz wiring |
| C8 | holdout report、uncertainty、final completion matrix |

unit gate failure は A4 へ backflow します。targeted test の pass だけで unit を pass にしません。

### 13. A6: Integration Gate

C0-C8 の feasible slice を処理した後、同じ source binding に対して最低限次を実行します。

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo +1.88.0 check --workspace --all-targets --locked
cargo run --quiet -p xtask -- completion --format json
git diff --check
```

追加 blocking checks:

- schema/semantic revision contract suite
- hostile Markdown、bounded I/O、delta property、private overlay privacy suite
- deterministic query double-run と source-session digest equality
- public absolute path / private marker / credential scan
- RepoSeiri self-audit summary、governance、linter、patches
- unrelated repository に対する Windows/Linux plugin smoke
- missing binary、manifest mismatch、schema mismatch、semantic revision mismatch、non-zero propagation
- README/docs/CLI/schema/plugin/version parity
- fuzz smoke と、利用可能な場合の Miri/Kani check
- source-bound Windows/Linux host receipt

環境 policy で実行できない check は `EnvironmentBlocked` です。skip や推測 pass にはしません。

### 14. A7: Terminal Decision

判定順:

1. 全 source/test が実装され、required local check が同一 source で pass: `ready_for_git`
2. source/test は実装されたが host/tool policy により required evidence が blocked: `implemented_with_blocked_evidence`
3. semantic failure、privacy failure、dependency block、repair budget exhaustion が残る: `incomplete`

`EVIDENCE_COMPLETE` は Roadmap v10 の required local/host/calibration evidence が全て同一 source binding に結合した場合だけ付与します。

### 15. 非対話継続規則

- 実行開始後、repairable failure、テスト失敗、missing optional tool、設計候補の選択で clarification を要求しません。
- user update は進捗通知であり承認待ちではありません。
- irreversible policy、license、support promise、security commitment、visibility、release、GitHub mutation を推測しません。必要な slice は blocked にして残りを完走します。
- process termination、host shutdown、tool host failure はこの protocol の外部境界です。resume 時は A8 を使います。
- final state が `incomplete` でも、実装済みの安全な diff と evidence を保持し、失敗を隠しません。

### 16. A8: Resume And Drift

resume 時は protocol/roadmap/template digest、base HEAD、Cargo.lock、owned-path digest、source-session digest を確認します。

- 一致: 最初の nonterminal slice から再開
- user-owned path のみ変更:保持して dependency を再評価
- protocol-owned path が外部変更: overlap repair を一度行い、不可なら affected slice を blocked
- roadmap/template変更: old executionを`superseded`にして新 executionを作成
- HEAD移動: commit差分を読み、安全に統合できる場合だけ継続

reset、force checkout、hidden stash、user diff の削除は行いません。

### 17. Final Report

最終応答は次の順にします。

1. terminal state と source binding
2. C0-C8 / slice completion table
3. implemented facts
4. selected/rejected Rust method worlds と unsafe status
5. verification performed
6. blocked checks と environment evidence
7. remaining semantic/empirical uncertainty
8. 未実行の Git、GitHub、release、visibility、plugin install 操作
9. 次に必要な明示 authority

途中 failure があっても、この report まで一つの実行として進みます。

### 18. Git Handoff

標準 trigger は Git 操作を行いません。`ready_for_git` 後の commit 分割、push、PR、merge は別の明示指示で行います。同じ user message がそれらの authority と範囲を明示した場合だけ、verification 後に実行できます。

### 19. Claim Boundary

R10-SIP-v1 は Codex instruction/skill layer の実行契約です。host-level absolute interception、filesystem transaction、process survival、GitHub permission、release approval を提供しません。

`ready_for_git` は同じ local source に対する required local verification を示します。`EVIDENCE_COMPLETE` も repository の人気、信頼、安全性、品質、法的適合性、将来保守、一般性能の証明ではありません。

---

## English

### 1. Purpose

R10-SIP-v1 is the Codex execution contract that decomposes Roadmap v10 C0-C8 from one explicit instruction and proceeds through implementation, local verification, automatic repair, backflow, integration verification, and the final report without waiting for interactive answers.

"Do not stop" means that ordinary compile/test/lint failures, local design gaps, and environment-dependent checks do not pause for a user reply. It does not mean converting failures to passes, retrying forever, publishing private data, destroying user changes, or performing unauthorized operations. A slice that cannot continue moves to a typed blocked state; the executor processes all remaining safe slices and final verification and always reaches a terminal report.

### 2. Trigger

Standard trigger:

```text
Implement C0-C8 as one batch under R10-SIP-v1.
```

The following form has the same meaning:

```text
Execute R10-SIP-v1.
```

The trigger authorizes repository mutation and local verification for C0-C8. It does not authorize commit, push, merge, release, publication, visibility changes, plugin reinstallation, or Codex restart.

### 3. Authority Envelope

| Authority | Default | Scope |
| --- | --- | --- |
| AnalysisAuthority | true | Read repository, tests, docs, and local tool state |
| MutationAuthority | true | Roadmap v10 source, tests, fixtures, schemas, and public docs |
| VerificationAuthority | true | Local build, test, lint, audit, fuzz smoke, and benchmark smoke |
| LedgerAuthority | true | Ignored `target/r10-sip/` metadata |
| CommitAuthority | false | Stage / commit |
| PushAuthority | false | Remote push |
| MergeAuthority | false | PR merge |
| ReleaseAuthority | false | Tag, release, and publication |
| VisibilityAuthority | false | Public/private changes |
| InstallAuthority | false | Plugin installation / cache update |
| RestartAuthority | false | Codex, app, or process restart |

No other authority is inferred from MutationAuthority. An authority changes only when a higher-level explicit instruction grants it in the same turn.

### 4. Read Order

At execution time, read:

1. `AGENTS.md`
2. `docs/design/roadmap-v10-closure-and-product-integrity.md`
3. `docs/design/r10-sip-v1-protocol.md`
4. `docs/design/r10-sip-v1-template.json`
5. the current `ContractManifest`, public schemas, and completion harness
6. the current worktree and user-owned changes

Older roadmaps and protocols are history and comparison material. Roadmap v10 and R10-SIP-v1 take precedence for overlapping responsibilities.

### 5. Execution Ledger

Store the ledger at `target/r10-sip/<execution-id>/state.json`. `target/` is outside Git tracking.

Allowed:

- Protocol/roadmap/template digests
- Base HEAD, branch, upstream, worktree identity, and Cargo.lock digest
- Unit/slice ID, dependency, owned-path digest, state, and attempt count
- Command ID, argv digest, exit class, duration, and bounded-output digest
- Source-session digest, contract-revision set, and verification receipt
- Failure class, repair/backflow decision, and remaining residual

Forbidden:

- Source, diff, README, issue, or security-report bodies
- Private-analysis filenames or bodies
- Private-calibration bodies, exact priors, or private digests
- Host absolute paths, usernames, credentials, tokens, or environment secrets

### 6. State Machine

Execution state:

```text
armed
  -> baselined
  -> expanded
  -> executing
  -> integrating
  -> closure_verifying
  -> ready_for_git
  -> implemented_with_blocked_evidence
  -> incomplete
```

Slice state:

```text
pending
  -> in_progress
  -> passed
  -> repairing -> in_progress
  -> backflow_pending -> in_progress
  -> blocked_environment
  -> blocked_authority
  -> blocked_privacy
  -> blocked_conflict
  -> blocked_dependency
  -> superseded
```

At most one slice may be `in_progress`. Every terminal state is recorded in the ledger and final report.

### 7. A0: Arm And Baseline

1. Record root, HEAD, branch, upstream, dirty state, toolchain, Cargo.lock, and installed plugin version.
2. Classify existing changes as `user_owned`, `protocol_owned`, or `overlap_unknown`; never revert, stash, or overwrite user-owned diffs.
3. Capture baseline format, targeted tests, workspace check, and self-audit where possible, distinguishing existing failures from new failures.
4. Fix roadmap/protocol/template digests and the authority envelope in the ledger.
5. Capture private-data-marker and public-absolute-path scan baselines without storing private-data bodies in the ledger.

A baseline failure does not pause execution. Record it and proceed to A1 so attribution remains possible.

### 8. A1: Slice Expansion

Decompose every unit into slices with one owner and one primary contract. Every slice carries:

```text
id
unit
depends_on
owned_paths
input_contracts
output_contracts
invariants
selected_method_world
rejected_method_worlds
targeted_checks
blocking_checks
repair_budget
rollback_scope
claim_boundary
```

Minimum slices:

| Unit | Required slices |
| --- | --- |
| C0 | Coverage partition, summary projection, completion predicate, regression |
| C1 | Bounded walk, bounded read, source session, and projection reuse |
| C2 | Digest framing, stable tags, portable identity, FNV migration, property tests |
| C3 | Semantic revisions, nested schemas, launcher integrity, migration fixtures |
| C4 | Pack compiler, overlay evaluator, private freshness, support tier |
| C5 | Shared Markdown IR, wording nodes, route tokens, proposition consistency |
| C6 | README entry, real example, docs authority, claim/version parity |
| C7 | Process supervisor, host receipts, CI pins, fuzz wiring, bundle verification |
| C8 | Corpus split, metrics/intervals, claim matrix, final completion |

Expand scope only when required by a roadmap invariant, recording the reason and dependency first.

### 9. A2: Rust Method-World Gate

Before editing each nontrivial Rust slice, compare at least:

1. Safe idiomatic Rust
2. Newtype / typestate / sealed trait
3. Compact IR / interning / bounded arena
4. Iterator / streaming / bounded buffer
5. Verification-oriented pure kernel
6. Unsafe capsule
7. Parallel / async / OS-specific backend

Select the lowest complexity that preserves the invariant:

- Select `unsafe` only when a safe alternative cannot preserve the invariant; add a local safety contract and a Miri target.
- Decode and validate persistent/wire bytes before typing them.
- Give concurrency explicit publication/cancellation semantics.
- Do not claim performance, SIMD, parallel, or allocation improvements without paired measurements.
- Do not make prose renderers, CLI glue, or docs-only paths low-level.

### 10. A3: Continuous Slice Loop

Select one dependency-ready slice in ID order:

1. Re-read its input contract, owned paths, and latest source binding.
2. Use a Labyrinth coding artifact to inspect premises, TranslationLoss, LLMCodingLoss, countermodels, and the gate.
3. Implement the smallest coherent diff.
4. Run formatting, crate checks, targeted tests, and schema/fixture tests.
5. Critique the diff for missing claims and tests.
6. On pass, store the ledger receipt and continue.
7. On failure, enter A4 without waiting for a user answer.

Per-slice commits, hidden stashes, destructive resets, and reverting unrelated changes are forbidden.

### 11. A4: Automatic Repair And Backflow

Repair budgets:

- Up to 3 local repairs per slice
- Up to 2 owner backflows per owner
- Up to 24 global repairs per execution
- At most 1 unchanged retry for the same command/error fingerprint

| Failure class | Automatic handling |
| --- | --- |
| `LocalRepairable` | Repair the same slice and rerun targeted checks |
| `ContractRegression` | Backflow to the contract-owner slice |
| `CrossUnitRegression` | Return the minimum affected unit set to pending |
| `SourceDrift` | Discard the session and rebaseline once from the same worktree |
| `EnvironmentBlocked` | Record command/OS error/unexecuted scope and continue independent slices |
| `AuthorityRequired` | Do not run the operation; block the affected slice and continue |
| `UserOverlap` | Preserve user changes, attempt reconciliation, and block the affected slice if unsafe |
| `PrivacyBoundary` | Remove a protocol-owned leak candidate from public/tracked output; preserve user-owned source, block the affected slice, run privacy regression, and continue |
| `UnsupportedDesignDecision` | Use the most conservative existing-contract choice or block the affected slice |
| `NonRepairable` | Block the affected dependency cone and continue independent slices and diagnostics |

Budget exhaustion never becomes pass. A slice depending on a blocked slice becomes `blocked_dependency`, while independent slices, documentation reconciliation, and final diagnostic checks continue.

### 12. A5: Unit Gates

At the end of each unit, apply its Roadmap v10 unit gate plus:

| Unit | Blocking evidence |
| --- | --- |
| C0 | Fixture/primary coverage separation and Unknown-completion negative test |
| C1 | Bounded hostile corpus, single-session counter, and source drift |
| C2 | Insertion/order/run invariance and known-answer digests |
| C3 | Nested-schema negative tests and revision/launcher parity |
| C4 | Executable-overlay effect and unsupported/stale/privacy negatives |
| C5 | Markdown dead zones, bilingual token matrix, and two-span conflict |
| C6 | Real-example snapshot and README/docs/CLI/plugin parity |
| C7 | Timeout/output-flood/kill tests, host binding, and CI/fuzz wiring |
| C8 | Holdout report, uncertainty, and final completion matrix |

A failed unit gate backflows through A4. A targeted-test pass alone never passes the unit.

### 13. A6: Integration Gate

After all feasible C0-C8 slices, run at least the following against one source binding:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo +1.88.0 check --workspace --all-targets --locked
cargo run --quiet -p xtask -- completion --format json
git diff --check
```

Additional blocking checks:

- Schema/semantic-revision contract suite
- Hostile Markdown, bounded-I/O, delta-property, and private-overlay privacy suites
- Deterministic query double-run and source-session digest equality
- Public-absolute-path, private-marker, and credential scans
- RepoSeiri self-audit summary, governance, linter, and patches
- Windows/Linux plugin smoke against an unrelated repository
- Missing-binary, manifest-mismatch, schema-mismatch, semantic-revision-mismatch, and non-zero propagation
- README/docs/CLI/schema/plugin/version parity
- Fuzz smoke and Miri/Kani where available
- Source-bound Windows/Linux host receipts

A check blocked by environment policy is `EnvironmentBlocked`; it is never skipped or guessed as pass.

### 14. A7: Terminal Decision

Decision order:

1. All source/tests implemented and required local checks pass against one source: `ready_for_git`
2. Source/tests implemented but required evidence is blocked by host/tool policy: `implemented_with_blocked_evidence`
3. Semantic failure, privacy failure, dependency block, or repair-budget exhaustion remains: `incomplete`

Grant `EVIDENCE_COMPLETE` only when all required local, host, and calibration evidence from Roadmap v10 binds to the same source.

### 15. Noninteractive Continuation Rules

- After execution begins, do not request clarification for repairable failures, test failures, missing optional tools, or design-candidate selection.
- User updates are progress reports, not approval waits.
- Never infer irreversible policy, licensing, support promises, security commitments, visibility, release, or GitHub mutation. Block the affected slice and complete the rest.
- Process termination, host shutdown, and tool-host failure are external to this protocol. Resume through A8.
- Even with final state `incomplete`, retain safe implemented diffs and evidence and do not hide failures.

### 16. A8: Resume And Drift

On resume, compare protocol/roadmap/template digest, base HEAD, Cargo.lock, owned-path digests, and source-session digest:

- Match: resume at the first nonterminal slice.
- Only user-owned paths changed: preserve them and reevaluate dependencies.
- Protocol-owned paths changed externally: attempt overlap repair once, then block affected slices if unsafe.
- Roadmap/template changed: mark the old execution `superseded` and create a new execution.
- HEAD moved: read commit differences and continue only when integration is safe.

Never reset, force checkout, hide a stash, or delete a user diff.

### 17. Final Report

Render:

1. Terminal state and source binding
2. C0-C8 / slice completion table
3. Implemented facts
4. Selected/rejected Rust method worlds and unsafe status
5. Verification performed
6. Blocked checks and environment evidence
7. Remaining semantic/empirical uncertainty
8. Unperformed Git, GitHub, release, visibility, and plugin-install operations
9. The next explicit authority required

The execution proceeds to this report as one run even when intermediate failures occur.

### 18. Git Handoff

The standard trigger performs no Git operation. Commit splitting, push, PR, and merge happen after `ready_for_git` under a separate explicit instruction. They may run in the same user turn only when that message explicitly grants and scopes those authorities.

### 19. Claim Boundary

R10-SIP-v1 is an execution contract in the Codex instruction/skill layer. It does not provide host-level absolute interception, a filesystem transaction, process survival, GitHub permission, or release approval.

`ready_for_git` means required local verification passed against the same local source. Even `EVIDENCE_COMPLETE` is not proof of repository popularity, trust, security, quality, legal fitness, future maintenance, or general performance.
