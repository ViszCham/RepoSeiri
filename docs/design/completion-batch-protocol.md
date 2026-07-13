# RepoSeiri Completion Batch Protocol v1

## 日本語

### 1. 目的

RCBP-v1は、Roadmap v6のCF0-CF7を一つのユーザー指示で開始し、内部では小さなsliceへ分解して実装、検証、repair、integrationを進めるCodex実行契約です。

RCBP-v1はfilesystem transactionではありません。通常modeでは同じworktreeへ順次変更を置きますが、途中commitを作らず、最後に一つの統合diffとして引き渡します。worktreeへ最後まで変更を見せないisolated-worktree modeは、別のGit/worktree権限が明示された場合だけ使用できます。

### 2. Trigger

標準triggerは次です。

```text
RCBP-v1でCF0-CF7を一括実装してください。
```

次の長い形式も同じ意味です。

```text
RCBP-v1でCF0-CF7を一括実装してください。
内部sliceへの分解、実装、repair、local verificationまで許可します。
commit、push、merge、release、plugin再インストール、再起動は行わないでください。
```

Codexはtriggerを受けたら、Roadmap v6、この文書、`rcbp-v1-template.json`、`AGENTS.md`の順にauthorityと実行契約を確認します。

### 3. Authority Envelope

triggerが既定で付与する権限:

| Authority | Default | Scope |
| --- | --- | --- |
| MutationAuthority | true | CF0-CF7のsource、test、fixture、public docs |
| TestAuthority | true | local build、test、lint、audit、fuzz corpus |
| ExecutionLedgerAuthority | true | ignored `target/rcbp/` metadata |
| CommitAuthority | false | Git commit |
| PushAuthority | false | remote push |
| MergeAuthority | false | PR merge |
| ReleaseAuthority | false | tag、GitHub Release、publication |
| PluginInstallAuthority | false | marketplace/cache reinstall |
| RestartAuthority | false | Codex/app/process restart |
| VisibilityAuthority | false | public/private変更 |

上位のユーザー指示が個別authorityを明示した場合だけ、その値を変更できます。MutationAuthorityからGit、release、restart権限を推測しません。

### 4. Execution Ledger

ledgerは`target/rcbp/<execution-id>/state.json`へ保存します。`target/`はGit管理外です。

ledgerに保存できるもの:

- protocol version、roadmap digest、base HEAD、開始時worktree summary
- block/slice ID、state、owned path、input/output contract ID
- 実行command名、exit code、duration、test count、artifact digest
- failure class、repair count、residual、final state

ledgerに保存しないもの:

- source file本文、diff本文、README本文
- private analysis本文またはfilename
- private calibrationのraw body、exact prior、local source path
- credential、token、environment secret
- GitHub issue、security report、user contentの本文

### 5. State Machine

execution state:

```text
armed -> baselined -> sliced -> executing -> integrating -> verifying
  -> ready_for_git
  -> incomplete
```

slice state:

```text
pending -> in_progress -> passed
                       -> repairing -> in_progress
                       -> blocked
                       -> superseded
```

同時に`in_progress`にできるsliceは一つです。`ready_for_git`は実装とverificationが終わったことを示しますが、Git操作を許可しません。

### 6. R0: Arm

1. repository root、current branch、HEAD、upstream、worktreeを確認する。
2. Rust toolchain、Cargo.lock、plugin source version、baseline test数を記録する。
3. Roadmap v6とmachine templateのdigestを記録する。
4. 既存変更をuser-owned、RCBP-owned、overlap-unknownへ分類する。
5. unresolved maintainer decisionがあればedit前に`incomplete`で停止する。

### 7. R1: Slice Expansion

各CF blockを、単一ownerと単一主要契約を持つsliceへ分解します。sliceは次を持ちます。

```text
id
depends_on
owned_paths
input_contracts
output_contracts
targeted_checks
blocking_checks
rollback_scope
claim_boundary
```

最低分割はRoadmap v6のCFx.yを使います。実装中に追加sliceが必要な場合は、scopeを広げる前にledgerへreasonとdependencyを追加します。

### 8. R2: Slice Loop

1. sliceのinput contractとowned pathを再読する。
2. Labyrinth coding artifactでpremise、loss、gate、counterexampleを確認する。
3. 最小diffを実装する。
4. targeted fmt、crate test、wire/fixture checkを実行する。
5. diff critiqueを行う。
6. passならledgerを更新し、次の依存可能sliceへ進む。

各sliceでcommitを作りません。targeted checkのpassを全体completionへ昇格しません。

### 9. R3: Failure And Backflow

| Class | Handling |
| --- | --- |
| LocalRepairable | 同じsliceで修正しtargeted checkを再実行 |
| ContractRegression | 契約owner sliceへbackflow |
| CrossBlockRegression | affected groupを再度incompleteに戻す |
| UserOverlap | user変更を保持し、統合できなければ停止 |
| AuthorityRequired | 対象operationを実行せずuserへ確認 |
| EnvironmentBlocked | 未検証として記録しcompletionを禁止 |
| PrivacyBoundary | 直ちにaffected outputを停止しleak sourceを除去 |

blocking checkのskip、expected outputの書き換えだけによるtest pass、schema versionを変えないbreaking wire変更は禁止します。

### 10. R4: Group Gates

| Group | Blocks | Gate |
| --- | --- | --- |
| Runtime | CF0-CF1 | schema、exit、standalone plugin |
| Semantics | CF2-CF4 | coverage、route、co-occurrence、profile |
| Hardening | CF5-CF6 | fuzz、resource、privacy、calibration、pack |
| Completion | CF7 | xtask、CI、bundle、docs、final schema |

group gateが失敗した場合、次groupへ進む前にowner sliceへbackflowします。独立作業を続けても、failed groupをpass扱いにはしません。

### 11. R5: Final Integration Gate

最低限、同じworktree stateに対して次を実行します。

```powershell
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo +1.88.0 check --workspace --all-targets --locked
cargo run --quiet -p xtask -- completion --format json
git diff --check
```

completion harnessがCF7以前に存在しない間は、Roadmap v6のcheckを個別実行し、ledger上ではcompletion harnessを`pending`のままにします。
required host evidenceを統合するCIでは`--host-evidence target/host-evidence`を追加します。Windows/Linuxの一方でも欠ける場合、completion harnessは`incomplete`を返します。

追加blocking check:

- removed-wire negative test
- privacy/public-boundary guard
- deterministic property corpusとrelease fuzz corpus
- RepoSeiri self-audit invariant
- unrelated repositoryでのWindows / Linux plugin smoke
- missing binary、schema mismatch、non-zero propagation
- docs、CLI help、schema、plugin skillのversion/boundary parity

### 12. R6: Final Report

final reportは次の順で表示します。

1. final state: `ready_for_git`または`incomplete`
2. base HEADとcurrent worktree identity
3. block/slice completion table
4. public schema、CLI、plugin変更
5. verification commandと結果
6. self-audit before/after invariants
7. LOCとtest countのbefore/after
8. unresolved residualとmanual decision
9. plugin reinstall、restart、Git、releaseの未実行状態

blocking checkが一つでもfail、blocked、pendingなら`ready_for_git`を出しません。

### 13. R7: Git Handoff

`ready_for_git`後もGit操作は実行しません。Git integrationを行う場合は、ユーザーが別の指示でcommit、push、PR、mergeの範囲を明示します。

一つのcommitへまとめる場合でも、final reportとcommit messageはCF block別に変更を説明します。RCBP slice IDをpublic APIやproduction type名へ入れません。

### 14. Resume And Drift

resume時はbase HEAD、roadmap digest、template digest、owned path hashを再確認します。

- 一致: pending sliceから再開する。
- user-owned pathだけが変化:変更を保持して再評価する。
- RCBP-owned pathが外部変更: overlapとして再統合する。
- roadmap/templateが変化:旧executionをsupersededにし、新しいexecution IDを作る。
- HEADが移動: commit差分を確認し、安全に再baseできなければ停止する。

destructive reset、unrelated changeのrevert、hidden stashは使用しません。

### 15. Completion Boundary

RCBP-v1はCodex skill/instruction layerの実行契約です。host-level transaction、absolute interception、process isolation、GitHub権限、release承認を提供しません。

RCBP-v1の成功は、指定されたverificationが通った統合worktreeを示します。人気、信頼、安全性、品質、法的妥当性、保守性、一般性能を証明しません。

---

## English

### 1. Purpose

RCBP-v1 is the Codex execution contract that starts Roadmap v6 CF0-CF7 from one user instruction while internally decomposing the work into small implementation, verification, repair, and integration slices.

RCBP-v1 is not a filesystem transaction. Normal mode places changes into the same worktree incrementally, creates no intermediate commits, and hands off one integrated diff at the end. Isolated-worktree mode, which hides changes from the primary worktree until integration, is available only with separate explicit Git/worktree authority.

### 2. Trigger

The standard trigger is:

```text
Implement CF0-CF7 as one batch under RCBP-v1.
```

The following expanded form has the same meaning:

```text
Implement CF0-CF7 as one batch under RCBP-v1.
You may decompose the work into internal slices, implement, repair, and run local verification.
Do not commit, push, merge, release, reinstall the plugin, or restart Codex.
```

After receiving the trigger, Codex reads Roadmap v6, this document, `rcbp-v1-template.json`, and `AGENTS.md` in that order to confirm authority and execution contracts.

### 3. Authority Envelope

Authorities granted by the trigger by default:

| Authority | Default | Scope |
| --- | --- | --- |
| MutationAuthority | true | CF0-CF7 source, tests, fixtures, and public docs |
| TestAuthority | true | Local build, test, lint, audit, and fuzz corpus |
| ExecutionLedgerAuthority | true | Ignored `target/rcbp/` metadata |
| CommitAuthority | false | Git commit |
| PushAuthority | false | Remote push |
| MergeAuthority | false | PR merge |
| ReleaseAuthority | false | Tags, GitHub Releases, and publication |
| PluginInstallAuthority | false | Marketplace/cache reinstall |
| RestartAuthority | false | Codex, app, or process restart |
| VisibilityAuthority | false | Public/private changes |

Only a higher-priority explicit user instruction can change an individual authority. Never infer Git, release, or restart authority from MutationAuthority.

### 4. Execution Ledger

Store the ledger at `target/rcbp/<execution-id>/state.json`. `target/` is outside Git tracking.

The ledger may store:

- Protocol version, roadmap digest, base HEAD, and starting worktree summary
- Block/slice IDs, states, owned paths, and input/output contract IDs
- Command names, exit codes, durations, test counts, and artifact digests
- Failure class, repair count, residuals, and final state

The ledger must not store:

- Source-file, diff, or README bodies
- Private-analysis bodies or filenames
- Raw private-calibration bodies, exact priors, or local source paths
- Credentials, tokens, or environment secrets
- GitHub issue, security report, or user-content bodies

### 5. State Machine

Execution state:

```text
armed -> baselined -> sliced -> executing -> integrating -> verifying
  -> ready_for_git
  -> incomplete
```

Slice state:

```text
pending -> in_progress -> passed
                       -> repairing -> in_progress
                       -> blocked
                       -> superseded
```

At most one slice may be `in_progress`. `ready_for_git` means implementation and verification finished; it does not authorize Git operations.

### 6. R0: Arm

1. Check repository root, current branch, HEAD, upstream, and worktree.
2. Record Rust toolchains, Cargo.lock, plugin-source version, and baseline test count.
3. Record Roadmap v6 and machine-template digests.
4. Classify existing changes as user-owned, RCBP-owned, or overlap-unknown.
5. Stop as `incomplete` before editing when a maintainer decision remains unresolved.

### 7. R1: Slice Expansion

Decompose every CF block into slices with one owner and one primary contract. Every slice carries:

```text
id
depends_on
owned_paths
input_contracts
output_contracts
targeted_checks
blocking_checks
rollback_scope
claim_boundary
```

Use Roadmap v6 CFx.y as the minimum decomposition. If implementation requires another slice, record its reason and dependency before expanding scope.

### 8. R2: Slice Loop

1. Re-read the slice input contract and owned paths.
2. Use a Labyrinth coding artifact to review premises, loss, gates, and counterexamples.
3. Implement the smallest coherent diff.
4. Run targeted formatting, crate tests, and wire/fixture checks.
5. Critique the diff.
6. On pass, update the ledger and move to the next dependency-ready slice.

Do not create commits for individual slices. Do not promote a targeted-check pass to global completion.

### 9. R3: Failure And Backflow

| Class | Handling |
| --- | --- |
| LocalRepairable | Repair in the same slice and rerun targeted checks |
| ContractRegression | Backflow to the contract-owner slice |
| CrossBlockRegression | Return the affected group to incomplete |
| UserOverlap | Preserve user changes and stop if integration is unsafe |
| AuthorityRequired | Do not run the operation; ask the user |
| EnvironmentBlocked | Record unverified state and forbid completion |
| PrivacyBoundary | Stop affected output and remove the leak source immediately |

Do not skip blocking checks, pass tests only by rewriting expected output, or make a breaking wire change without changing the schema version.

### 10. R4: Group Gates

| Group | Blocks | Gate |
| --- | --- | --- |
| Runtime | CF0-CF1 | Schema, exits, and standalone plugin |
| Semantics | CF2-CF4 | Coverage, routes, co-occurrence, and profiles |
| Hardening | CF5-CF6 | Fuzzing, resources, privacy, calibration, and packs |
| Completion | CF7 | xtask, CI, bundles, docs, and final schemas |

If a group gate fails, backflow to the owner slice before entering the next group. Independent work may continue, but the failed group never becomes passed.

### 11. R5: Final Integration Gate

Run at least the following against the same worktree state:

```powershell
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo +1.88.0 check --workspace --all-targets --locked
cargo run --quiet -p xtask -- completion --format json
git diff --check
```

Before the CF7 completion harness exists, run the Roadmap v6 checks separately and keep the completion-harness item `pending` in the ledger.
CI adds `--host-evidence target/host-evidence` when integrating required-host evidence. The completion harness returns `incomplete` when either Windows or Linux evidence is missing.

Additional blocking checks:

- Removed-wire negative tests
- Privacy/public-boundary guard
- Deterministic property corpus and release fuzz corpus
- RepoSeiri self-audit invariants
- Windows / Linux plugin smoke against an unrelated repository
- Missing-binary, schema-mismatch, and non-zero propagation checks
- Version/boundary parity across docs, CLI help, schemas, and plugin skill

### 12. R6: Final Report

Render the final report in this order:

1. Final state: `ready_for_git` or `incomplete`
2. Base HEAD and current worktree identity
3. Block/slice completion table
4. Public schema, CLI, and plugin changes
5. Verification commands and results
6. Before/after self-audit invariants
7. Before/after LOC and test count
8. Unresolved residuals and manual decisions
9. Unperformed plugin reinstall, restart, Git, and release operations

Do not emit `ready_for_git` when any blocking check is failed, blocked, or pending.

### 13. R7: Git Handoff

Do not perform Git operations after `ready_for_git`. A separate user instruction must explicitly scope commit, push, PR, and merge operations.

Even when producing one commit, explain the final report and commit message by CF block. Do not put RCBP slice IDs into public APIs or production type names.

### 14. Resume And Drift

On resume, recheck the base HEAD, roadmap digest, template digest, and owned-path hashes.

- Match: Resume from the pending slice.
- Only user-owned paths changed: Preserve the changes and reevaluate.
- RCBP-owned paths changed externally: Reconcile as overlap.
- Roadmap/template changed: Mark the old execution superseded and create a new execution ID.
- HEAD moved: Inspect commit differences and stop if safe rebasing is unavailable.

Do not use destructive reset, revert unrelated changes, or use a hidden stash.

### 15. Completion Boundary

RCBP-v1 is an execution contract in the Codex skill/instruction layer. It does not provide host-level transactions, absolute interception, process isolation, GitHub authority, or release approval.

RCBP-v1 success means that an integrated worktree passed the specified verification. It does not prove popularity, trust, security, quality, legal fitness, maintenance, or general performance.
