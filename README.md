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

実装ロードマップと一括実装ブロックは [Roadmap And Implementation Blocks](docs/design/roadmap-and-implementation-blocks.md) に置きます。

### Block A の使い方

Block A は、Rust workspace、core IR、file scanner、Markdown route scanner、JSON / Markdown report、`seiri audit` を提供します。

```powershell
cargo test --workspace
cargo run -p seiri-cli -- audit --path fixtures/readme-route-repo --format json
cargo run -p seiri-cli -- audit --path fixtures/missing-readme-repo --format markdown
```

Block A は GitHub API、Codex adapter、PR 作成、patch generation、profile scoring、100,000 件 data ingest をまだ実行しません。

### Block B の使い方

Block B は Pattern Registry、Common Baseline、baseline finding generation、baseline report を追加します。`seiri audit` の JSON / Markdown 出力には、`pattern_matches` と `baseline` が含まれます。

```powershell
cargo test --workspace
cargo run -p seiri-cli -- audit --path fixtures/readme-route-repo --format json
cargo run -p seiri-cli -- audit --path fixtures/missing-readme-repo --format markdown
```

Block B は profile scoring、auto fix、remote metadata、GitHub API、Codex adapter をまだ実行しません。

### Block C の使い方

Block C は Profile Branching、profile rules、recommendation order、score view を追加します。`seiri audit` は `--profile common|library|cli|infra|docs|tutorial|research|template` を受け取り、目的別に不足 route の優先順位を変えます。

```powershell
cargo test --workspace
cargo run -p seiri-cli -- audit --path fixtures/docs-routed-repo --profile cli --format json
cargo run -p seiri-cli -- audit --path fixtures/docs-routed-repo --profile infra --format markdown
```

Block C の score view は、観測された baseline pattern に対する決定的な priority view です。人気、信頼、セキュリティ、品質の保証ではありません。Block C は safe patch、Codex PR、GitHub API、auto fix、100,000 件 data ingest をまだ実行しません。

### Block D の使い方

Block D は Safe / Guarded / Manual gate、dry-run patch plan、safe routing patch operation を追加します。`seiri plan` は実ファイルを書き換えず、safe な README routing だけを operation として出し、guarded / manual な項目は blocked item として表示します。

```powershell
cargo test --workspace
cargo run -p seiri-cli -- plan --path fixtures/safe-plan-repo --profile common --format json
cargo run -p seiri-cli -- plan --path fixtures/missing-readme-repo --profile library --format markdown
```

Block D は GitHub write、automatic policy decision、file mutation、Codex PR、auto fix apply、100,000 件 data ingest をまだ実行しません。

### Block E の使い方

Block E は Data Calibration の受け皿を追加します。`seiri calibrate` は benchmark dataset または JSONL repo records を読み込み、known pattern stats、pending pattern candidates、weight suggestions を出します。出力は review 用の候補であり、未検証 rule の自動採用、truth claim、popularity / trust / security / quality guarantee は行いません。

```powershell
cargo test --workspace
cargo run -p seiri-cli -- calibrate --input fixtures/calibration-dataset.json --format json
cargo run -p seiri-cli -- calibrate --input fixtures/calibration-records.jsonl --format markdown
```

Block E は Codex PR、GitHub write、runtime registry mutation、automatic rule adoption、automatic policy decision、auto fix apply をまだ実行しません。

### Block F の使い方

Block F は Codex Integration を追加します。`seiri codex` は Rust core の audit、dry-run plan、review context、PR draft body をまとめて出します。repo 内 plugin source は `plugins/reposeiri` にあり、Codex 側はこの Rust CLI 出力を使う薄い adapter として扱います。

```powershell
cargo test --workspace
cargo run -p seiri-cli -- codex --path fixtures/safe-plan-repo --profile common --format json
cargo run -p seiri-cli -- codex --path fixtures/safe-plan-repo --profile common --view pr-body --format markdown
```

Block F は PR body と review context を生成しますが、GitHub write、branch 作成、commit、push、PR 作成、file mutation、core logic の plugin 側再実装は実行しません。

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

The implementation roadmap and batch implementation blocks live in [Roadmap And Implementation Blocks](docs/design/roadmap-and-implementation-blocks.md).

### Block A Usage

Block A provides the Rust workspace, core IR, file scanner, Markdown route scanner, JSON / Markdown report, and `seiri audit`.

```powershell
cargo test --workspace
cargo run -p seiri-cli -- audit --path fixtures/readme-route-repo --format json
cargo run -p seiri-cli -- audit --path fixtures/missing-readme-repo --format markdown
```

Block A does not yet run GitHub API access, Codex adapter actions, PR creation, patch generation, profile scoring, or 100,000-repository data ingest.

### Block B Usage

Block B adds the Pattern Registry, Common Baseline, baseline finding generation, and baseline report. The JSON / Markdown output from `seiri audit` includes `pattern_matches` and `baseline`.

```powershell
cargo test --workspace
cargo run -p seiri-cli -- audit --path fixtures/readme-route-repo --format json
cargo run -p seiri-cli -- audit --path fixtures/missing-readme-repo --format markdown
```

Block B does not yet run profile scoring, auto fix, remote metadata, GitHub API access, or Codex adapter actions.

### Block C Usage

Block C adds Profile Branching, profile rules, recommendation order, and score view. `seiri audit` accepts `--profile common|library|cli|infra|docs|tutorial|research|template` and changes the priority order of missing routes by repository purpose.

```powershell
cargo test --workspace
cargo run -p seiri-cli -- audit --path fixtures/docs-routed-repo --profile cli --format json
cargo run -p seiri-cli -- audit --path fixtures/docs-routed-repo --profile infra --format markdown
```

The Block C score view is a deterministic priority view over observed baseline patterns. It is not a guarantee of popularity, trust, security, or quality. Block C does not yet run safe patches, Codex PRs, GitHub API access, auto fix, or 100,000-repository data ingest.

### Block D Usage

Block D adds the Safe / Guarded / Manual gate, dry-run patch plan, and safe routing patch operations. `seiri plan` does not write files; it emits only safe README routing operations and keeps guarded / manual items as blocked items.

```powershell
cargo test --workspace
cargo run -p seiri-cli -- plan --path fixtures/safe-plan-repo --profile common --format json
cargo run -p seiri-cli -- plan --path fixtures/missing-readme-repo --profile library --format markdown
```

Block D does not yet run GitHub writes, automatic policy decisions, file mutation, Codex PRs, auto fix apply, or 100,000-repository data ingest.

### Block E Usage

Block E adds the Data Calibration intake surface. `seiri calibrate` reads a benchmark dataset or JSONL repository records and emits known pattern stats, pending pattern candidates, and weight suggestions. The output is a candidate review artifact; it does not automatically adopt unverified rules, make truth claims, or guarantee popularity, trust, security, or quality.

```powershell
cargo test --workspace
cargo run -p seiri-cli -- calibrate --input fixtures/calibration-dataset.json --format json
cargo run -p seiri-cli -- calibrate --input fixtures/calibration-records.jsonl --format markdown
```

Block E does not yet run Codex PRs, GitHub writes, runtime registry mutation, automatic rule adoption, automatic policy decisions, or auto fix apply.

### Block F Usage

Block F adds Codex Integration. `seiri codex` emits the Rust-core audit, dry-run plan, review context, and PR draft body together. The repo-local plugin source lives in `plugins/reposeiri`, and the Codex side treats the Rust CLI output as a thin adapter surface.

```powershell
cargo test --workspace
cargo run -p seiri-cli -- codex --path fixtures/safe-plan-repo --profile common --format json
cargo run -p seiri-cli -- codex --path fixtures/safe-plan-repo --profile common --view pr-body --format markdown
```

Block F generates PR bodies and review context, but it does not run GitHub writes, create branches, commit, push, create PRs, mutate files, or reimplement core logic inside the plugin.

### Implementation Policy

The low-level-first scope includes file traversal, reading Git objects and tree information, Markdown / YAML / workflow parsing, scoring intermediate representations, diff generation, and the core data structures for report generation. Prioritize areas where Rust ownership, types, explicit error handling, streaming, zero-copy, or low-copy design provide practical benefit.

For GitHub API authentication, Codex host integration, user-environment-dependent operations, and external service boundaries, use appropriate high-level APIs when they improve safety, maintainability, and compatibility. Low-level implementation is not an end in itself; use it when it improves performance, verifiability, portability, dependency reduction, or clarity of failure boundaries.

### Verification Boundary

The existing benchmark aggregate is treated as input for initial weighting and prioritization. It is not treated as a complete 10,000-repository root-tree crawl or statistical proof.
