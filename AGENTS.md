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
