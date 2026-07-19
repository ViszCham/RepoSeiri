# R11-SCCP-v1: Semantic Compression Completion Protocol

## 日本語

### 1. Triggerと権限

```text
R11-SCCP-v1でK0-K12を一括実装してください。
```

このtriggerはK0-K12のrepository mutationとlocal verificationを許可します。commit、push、merge、release、publication、visibility change、plugin install、restartは許可しません。

### 2. 順次実行

1. HEAD、worktree、contract、source版RepoSeiri self-audit、既存failureをbaseline化します。
2. K0からK12をdependency順に一つずつ実行します。同時に`in_progress`にできるblockは一つです。
3. 各blockをdependency-closedなsliceへ分け、最小のcoherent diff、focused test、semantic differential、privacy scan、Labyrinth critiqueを実行します。
4. compile、test、lint failureは自動修復して再試行します。failureをpassへ変換せず、blocking checkをskipしません。
5. K2、K7、K10、K12の後にworkspace group gateを実行します。
6. user-owned changeはrevert、stash、resetしません。安全に統合できないblockだけをtyped blocked stateにします。
7. timeout processは終了し、範囲を狭めて再試行します。未完processを残しません。
8. 最後にblock状態、source digest、verification receipt、host/calibration境界、後続commit slice案を報告します。

全required local checkが同じsourceで通ったterminal stateを`ready_for_git`と呼びます。これはGit操作権限ではありません。

### 3. 停止条件

通常failureでは停止しません。private data leak、権限不足、破壊的操作の曖昧さ、user changeとの解消不能な衝突、同じhard blockerが3回連続した場合だけaffected dependency coneを停止します。独立blockと最終diagnosticは継続します。

### 4. Privacyとclaim境界

ledgerは`target/r11-sccp/<execution-id>/`に置けますが、source body、diff body、private analysis名/本文、exact prior、private digest、host absolute path、credentialを保存しません。local test、host receipt、calibration、manual policyは別claimです。

---

## English

### 1. Trigger And Authority

```text
Implement K0-K12 as one batch under R11-SCCP-v1.
```

The trigger authorizes repository mutation and local verification for K0-K12. It does not authorize commit, push, merge, release, publication, visibility changes, plugin installation, or restart.

### 2. Sequential Execution

1. Baseline HEAD, the worktree, contracts, source-built RepoSeiri self-audit, and existing failures.
2. Execute K0 through K12 in dependency order with at most one block `in_progress`.
3. Split each block into dependency-closed slices and run a minimal coherent diff, focused tests, semantic differentials, privacy scans, and Labyrinth critique.
4. Repair compile, test, and lint failures automatically. Never convert a failure into a pass or skip a blocking check.
5. Run workspace group gates after K2, K7, K10, and K12.
6. Never revert, stash, or reset user-owned changes. Mark only an irreconcilable affected block as typed blocked.
7. Terminate timed-out processes, retry a narrower scope, and leave no unfinished process.
8. Report block states, source digest, verification receipts, host/calibration boundaries, and a later commit-slice proposal.

The terminal state for all required local checks passing against the same source is `ready_for_git`. It is not Git-operation authority.

### 3. Stop Conditions

Ordinary failures do not stop execution. Stop only the affected dependency cone for a private-data leak, missing authority, destructive ambiguity, an irreconcilable user-change conflict, or the same hard blocker repeated three times. Continue independent blocks and final diagnostics.

### 4. Privacy And Claim Boundary

The ledger may live under `target/r11-sccp/<execution-id>/`, but it stores no source body, diff body, private-analysis name/body, exact prior, private digest, host absolute path, or credential. Local tests, host receipts, calibration, and manual policy remain separate claims.
