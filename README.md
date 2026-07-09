# RepoSeiri

## 日本語

RepoSeiri は、GitHub リポジトリを監査し、整理し、改善提案へつなげる Rust 実装の Codex プラグイン / アプリとして設計します。

### 固定前提

- プロダクト名とリポジトリ名は `RepoSeiri` です。
- 実装言語は Rust です。
- 人間が読む主要ドキュメントは、前半を日本語、後半を英語にします。
- 日本語部分と英語部分には、同じ内容、同じ判断、同じ注意事項を書くものとします。
- 片方の言語だけに仕様、制約、手順、警告を追加しません。
- 実装可能な部分は、なるべく低レイヤで実装します。
- RepoSeiri の中核設計は、Repository Trust Graph、Trust Path Planner、Safe Repair Engine とします。
- 全体共通の observable evidence を先に実装し、その上に目的別 profile を重ねます。
- README は routing hub とし、詳細は docs、support、security、contributing、release、governance へ逃がします。

### 初期スコープ

このリポジトリは、GitHub リポジトリ品質分析のための評価器、レポート生成、改善提案、Codex 連携を段階的に実装する作業場所です。初期段階では、README ルーティング、docs トポロジー、community health、security posture、CI、release、repository hygiene を repo type ごとに評価する設計を前提にします。

### 設計ドキュメント

RepoSeiri の詳細設計は [Repository Trust Graph Design](docs/design/repository-trust-graph.md) に置きます。README は設計本文を抱え込まず、詳細設計への導線だけを持ちます。

### 実装方針

低レイヤ優先の対象は、ファイル走査、Git オブジェクトや tree 情報の読み取り、Markdown / YAML / workflow 解析、スコアリング中間表現、差分生成、レポート生成の核となるデータ構造です。Rust の所有権、型、明示的なエラー処理、ストリーム処理、ゼロコピーまたは低コピー設計が効く部分を優先します。

GitHub API 認証、Codex ホスト連携、ユーザー環境に依存する操作、外部サービス境界は、安全性、保守性、互換性を優先して適切な高レイヤ API を使います。低レイヤ化は目的ではなく、性能、検証性、移植性、依存削減、失敗境界の明確化に効く場合に採用します。

### 検証境界

既存の benchmark aggregate は、初期の重み付けと優先順位付けの材料として扱います。完全な 10,000 リポジトリの root-tree crawl や統計的証明としては扱いません。

---

## English

RepoSeiri is designed as a Rust-based Codex plugin / app that audits GitHub repositories, organizes findings, and turns them into improvement proposals.

### Fixed Premises

- The product name and repository name are `RepoSeiri`.
- The implementation language is Rust.
- Major human-facing documents use Japanese in the first half and English in the second half.
- The Japanese half and the English half must contain the same content, decisions, and cautions.
- Do not add specifications, constraints, steps, or warnings to only one language.
- Implement feasible parts at as low a layer as practical.
- The core RepoSeiri design is Repository Trust Graph, Trust Path Planner, and Safe Repair Engine.
- Implement common observable evidence first, then layer purpose-specific profiles on top.
- README is a routing hub; details move to docs, support, security, contributing, release, and governance surfaces.

### Initial Scope

This repository is the working place for gradually implementing evaluators, report generation, improvement proposals, and Codex integration for GitHub repository quality analysis. The initial design assumes repo-type-aware evaluation of README routing, docs topology, community health, security posture, CI, release, and repository hygiene.

### Design Document

The detailed RepoSeiri design lives in [Repository Trust Graph Design](docs/design/repository-trust-graph.md). README keeps the route to the detailed design instead of carrying the full design body.

### Implementation Policy

The low-level-first scope includes file traversal, reading Git objects and tree information, Markdown / YAML / workflow parsing, scoring intermediate representations, diff generation, and the core data structures for report generation. Prioritize areas where Rust ownership, types, explicit error handling, streaming, zero-copy, or low-copy design provide practical benefit.

For GitHub API authentication, Codex host integration, user-environment-dependent operations, and external service boundaries, use appropriate high-level APIs when they improve safety, maintainability, and compatibility. Low-level implementation is not an end in itself; use it when it improves performance, verifiability, portability, dependency reduction, or clarity of failure boundaries.

### Verification Boundary

The existing benchmark aggregate is treated as input for initial weighting and prioritization. It is not treated as a complete 10,000-repository root-tree crawl or statistical proof.
