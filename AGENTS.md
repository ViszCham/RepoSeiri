# RepoSeiri Agent Instructions

## 日本語

このリポジトリでは、RepoSeiri の固定前提を保って作業します。

### 固定前提

- 名前は `RepoSeiri` とします。
- 実装言語は Rust とします。
- 人間が読む主要ドキュメントは、前半を日本語、後半を英語にします。
- 日本語部分と英語部分は、同じ意味、同じ判断、同じ制約、同じ警告を持つように更新します。
- 片方の言語だけに新しい仕様や作業手順を追加しないでください。
- 実装可能な部分は、なるべく低レイヤで実装します。
- RepoSeiri の中核設計は、Repository Trust Graph、Trust Path Planner、Safe Repair Engine として扱います。
- 全体共通の observable evidence を先に実装し、その上に目的別 profile を重ねます。
- README は routing hub とし、詳細は docs、support、security、contributing、release、governance へ逃がします。

### 作業方針

- 小さく検証できる変更を優先します。
- ファイル走査、Git オブジェクトや tree 情報の読み取り、Markdown / YAML / workflow 解析、スコアリング中間表現、差分生成、レポート生成の核は Rust の低レイヤ寄りの実装を優先します。
- GitHub API 認証、Codex ホスト連携、ユーザー環境に依存する操作、外部サービス境界では、安全性、保守性、互換性を優先して適切な高レイヤ API を使います。
- 低レイヤ化は目的ではなく、性能、検証性、移植性、依存削減、失敗境界の明確化に効く場合に採用します。
- 実装が追加されたら、該当する Rust の検証コマンドを実行します。
- benchmark aggregate の数値は、初期設計の重み付けとして扱い、実測済みの完全証明として扱いません。
- ユーザーや別作業の未関係な変更を戻さないでください。

### RCBP-v1 一括完成実装

- ユーザーが `RCBP-v1でCF0-CF7を一括実装してください`、または同じ意味の指示を出した場合、[Roadmap v6](docs/design/roadmap-v6-completion.md)、[RCBP-v1](docs/design/completion-batch-protocol.md)、[機械可読 template](docs/design/rcbp-v1-template.json) をこの順で読みます。
- trigger は CF0-CF7 の source、test、fixture、公開文書を変更する `MutationAuthority` と、必要なlocal verificationを実行する `TestAuthority` を付与します。
- trigger だけでは commit、push、merge、release、plugin再インストール、Codex再起動、repository visibility変更の権限を付与しません。これらは個別の明示指示を必要とします。
- 実装前にbase HEAD、worktree、roadmap digest、既存変更の所有境界を確認し、内部sliceへ分解します。同時に進行中にするsliceは一つだけです。
- execution ledgerは `target/rcbp/<execution-id>/state.json` に置き、source本文、private analysis本文、credential、private calibration値を保存しません。
- sliceごとにtargeted verificationを行い、block間の失敗は契約ownerへbackflowします。blocking checkをskipした状態を完成と呼びません。
- 最終状態は `ready_for_git` または `incomplete` です。`ready_for_git` はGit操作の許可ではありません。

---

## English

Work in this repository while preserving the fixed premises for RepoSeiri.

### Fixed Premises

- The name is `RepoSeiri`.
- The implementation language is Rust.
- Major human-facing documents use Japanese in the first half and English in the second half.
- Update the Japanese half and the English half so they carry the same meaning, decisions, constraints, and warnings.
- Do not add new specifications or work steps to only one language.
- Implement feasible parts at as low a layer as practical.
- Treat the core RepoSeiri design as Repository Trust Graph, Trust Path Planner, and Safe Repair Engine.
- Implement common observable evidence first, then layer purpose-specific profiles on top.
- README is a routing hub; details move to docs, support, security, contributing, release, and governance surfaces.

### Work Policy

- Prefer small changes that can be verified.
- Prefer low-level Rust implementations for the core of file traversal, reading Git objects and tree information, Markdown / YAML / workflow parsing, scoring intermediate representations, diff generation, and report generation.
- For GitHub API authentication, Codex host integration, user-environment-dependent operations, and external service boundaries, use appropriate high-level APIs when they improve safety, maintainability, and compatibility.
- Low-level implementation is not an end in itself; use it when it improves performance, verifiability, portability, dependency reduction, or clarity of failure boundaries.
- After implementation is added, run the relevant Rust verification commands.
- Treat benchmark aggregate numbers as initial design weights, not as measured complete proof.
- Do not revert unrelated changes made by the user or by other work.

### RCBP-v1 Completion Batch

- When the user says `Implement CF0-CF7 as one batch under RCBP-v1`, `RCBP-v1でCF0-CF7を一括実装してください`, or gives an equivalent instruction, read [Roadmap v6](docs/design/roadmap-v6-completion.md), [RCBP-v1](docs/design/completion-batch-protocol.md), and the [machine-readable template](docs/design/rcbp-v1-template.json) in that order.
- The trigger grants `MutationAuthority` for CF0-CF7 source, tests, fixtures, and public documentation, plus `TestAuthority` for the required local verification.
- The trigger alone does not grant authority to commit, push, merge, release, reinstall the plugin, restart Codex, or change repository visibility. Each requires a separate explicit instruction.
- Before editing, record the base HEAD, worktree state, roadmap digest, and ownership boundary for existing changes, then expand the blocks into internal slices. Keep at most one slice in progress.
- Store the execution ledger at `target/rcbp/<execution-id>/state.json`. Do not store source bodies, private-analysis bodies, credentials, or private-calibration values in it.
- Run targeted verification for every slice and backflow cross-block failures to the contract owner. Never call a run complete after skipping a blocking check.
- The only final states are `ready_for_git` and `incomplete`. `ready_for_git` does not authorize Git operations.
