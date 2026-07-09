# RepoSeiri

## 日本語

RepoSeiri は、GitHub リポジトリを監査し、整理し、改善提案へつなげる Rust 実装の Codex プラグイン / アプリとして設計します。

### 固定前提

- プロダクト名とリポジトリ名は `RepoSeiri` です。
- 実装言語は Rust です。
- 人間が読む主要ドキュメントは、前半を日本語、後半を英語にします。
- 日本語部分と英語部分には、同じ内容、同じ判断、同じ注意事項を書くものとします。
- 片方の言語だけに仕様、制約、手順、警告を追加しません。

### 初期スコープ

このリポジトリは、GitHub リポジトリ品質分析のための評価器、レポート生成、改善提案、Codex 連携を段階的に実装する作業場所です。初期段階では、README ルーティング、docs トポロジー、community health、security posture、CI、release、repository hygiene を repo type ごとに評価する設計を前提にします。

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

### Initial Scope

This repository is the working place for gradually implementing evaluators, report generation, improvement proposals, and Codex integration for GitHub repository quality analysis. The initial design assumes repo-type-aware evaluation of README routing, docs topology, community health, security posture, CI, release, and repository hygiene.

### Verification Boundary

The existing benchmark aggregate is treated as input for initial weighting and prioritization. It is not treated as a complete 10,000-repository root-tree crawl or statistical proof.
