# RepoSeiri

[![CI](https://github.com/ViszCham/RepoSeiri/actions/workflows/ci.yml/badge.svg)](https://github.com/ViszCham/RepoSeiri/actions/workflows/ci.yml)

## 日本語

RepoSeiri は、個人使用と Rust コーディング練習を目的に作っている公開リポジトリです。題材として、GitHub リポジトリの README、docs、license、security、support、CI などの導線を読み取り、Codex で使いやすい整理案を出す CLI / Codex plugin の試作を実装しています。

このリポジトリは公開されても読み手が目的を誤解しにくい形へ整えていますが、外部利用を前提にした製品ではありません。RepoSeiri の出力は個人利用のためのレビュー補助であり、人気、信頼、安全性、品質、法務適合、公開可否を保証しません。

### 3行要約

- RepoSeiri は、個人使用と Rust コーディング練習のための公開リポジトリです。
- GitHub リポジトリの bounded local evidence を読み、route・content・dry-run patch plan を出す CLI / Codex plugin prototype です。
- 出力は review aid であり、品質、安全性、信頼性、公開可否を保証しません。

### 何をするものか

- リポジトリ内の root files、README links、docs、GitHub workflow、issue templates などを観測します。
- 観測した typed evidence から route assessment、content slot、profile score、review priority を出します。
- 自動適用ではなく、`Safe`、`Guarded`、`Manual` の境界を分けた dry-run patch plan を出します。
- Codex で必要な summary、evidence、documents、governance、patch、PR body を query 単位で生成します。

### 現在の位置づけ

- 個人使用目的の Rust coding practice repository です。
- 実装対象は repository organization を支援する CLI / Codex plugin prototype です。
- Rust crate 側に監査、profile、pattern registry、calibration、patch planning の主要ロジックを置きます。
- Codex plugin は薄い adapter とし、Rust CLI の結果を Codex の作業文脈へ渡します。
- 人間向けの主要ドキュメントは、日本語を前半、英語を後半に置き、同じ内容、同じ判断、同じ注意点を保ちます。

### Quickstart

必要環境は Rust 1.76 以上です。

```powershell
cargo test --workspace
cargo run --quiet -p seiri-cli -- codex --path . --profile library --query summary --format markdown
```

まずこの2つを実行します。1行目で workspace の基本動作を確認し、2行目で RepoSeiri 自身を対象にした Codex 向け整理案を確認します。

**Example Output**

RepoSeiri 自身を対象にした Codex summary の短縮例です。件数はリポジトリ状態で変わり、外部評価や安全性を保証しません。

```text
Schema: seiri.codex.v1
Query: summary
Evidence facts: <count>
Route assessments: 14
Patch operations / holds: <count> / <count>
```

### 主要コマンド

| 目的 | コマンド |
| --- | --- |
| 監査レポート | `cargo run --quiet -p seiri-cli -- audit --path . --profile library --format markdown` |
| dry-run patch plan | `cargo run --quiet -p seiri-cli -- plan --path . --profile library --format markdown` |
| Codex summary | `cargo run --quiet -p seiri-cli -- codex --path . --profile library --query summary --format markdown` |
| typed evidence JSON | `cargo run --quiet -p seiri-cli -- codex --path . --profile library --query evidence --format json` |
| documents / governance | `cargo run --quiet -p seiri-cli -- codex --path . --profile library --query governance --format json` |
| patch query | `cargo run --quiet -p seiri-cli -- codex --path . --profile library --query patches --format markdown` |
| wording linter | `cargo run --quiet -p seiri-cli -- codex --path . --profile library --query linter --format markdown` |
| Codex PR body draft | `cargo run --quiet -p seiri-cli -- codex --path . --profile library --query pr-body --format markdown` |
| pattern registry | `cargo run --quiet -p seiri-cli -- patterns --format markdown` |
| calibration ingest | `cargo run --quiet -p seiri-cli -- calibrate --input fixtures/calibration-dataset.json --format markdown` |

### 結果の読み方

- `Verified` は、存在確認済みの repository-local target と対応する構造的証拠が揃っている状態です。
- `Structured` は、構造的証拠はあるが README route が明示されていない状態です。
- `Routed` は、README 内に入口がある状態です。これだけでは repository-local target の存在を示しません。
- `Weak`、`Overloaded`、`Stale`、`Conflicting` は、入口が薄い、多すぎる、古い、または意図が曖昧な状態です。
- `Absent` と `UnsafeToInvent` は、RepoSeiri が自動生成すべきではない、または人間の方針決定が先に必要な状態です。

### リポジトリの入口

| 読みたいもの | 入口 |
| --- | --- |
| project identity / README | [RepoSeiri README](README.md) |
| docs 全体の地図 | [Documentation Topology](docs/README.md) |
| license | [LICENSE](LICENSE) |
| security report | [SECURITY.md](SECURITY.md) |
| support route | [SUPPORT.md](SUPPORT.md) |
| contribution route | [CONTRIBUTING.md](CONTRIBUTING.md) |
| governance boundary | [GOVERNANCE.md](GOVERNANCE.md) |
| release history | [CHANGELOG.md](CHANGELOG.md) |
| lifecycle / maintenance boundary | [Lifecycle / Maintenance Boundary](docs/release.md) |
| issue / PR intake | [.github/ISSUE_TEMPLATE](.github/ISSUE_TEMPLATE) |
| ownership boundary | [.github/CODEOWNERS](.github/CODEOWNERS) |
| CI automation | [.github/workflows/ci.yml](.github/workflows/ci.yml) |
| hygiene / self-audit | [Repository Hygiene](docs/hygiene.md) |

公開状態の checklist、設計docs、release手順などの詳細は docs topology から辿ります。

### Codex plugin route

- Plugin root: `plugins/reposeiri`
- Skill file: [RepoSeiri Skill](plugins/reposeiri/skills/reposeiri/SKILL.md)
- Codex 側では `seiri codex` の出力を優先して使います。
- plugin は `seiri.codex.v1` の query を使い、repository policy を推測で作らず、Rust CLI の typed observation と dry-run plan を作業文脈へ渡します。

### 公開リポジトリとしての境界

- このリポジトリは公開リポジトリとして読めるように整理します。公開後も、個人使用・コーディング練習目的であることを維持します。
- README は「何のリポジトリか」「どう動かすか」「どこを読むか」だけを持ち、詳細設計は docs に逃がします。
- `SECURITY.md`、`SUPPORT.md`、`CONTRIBUTING.md` は案内用です。固定 SLA、外部 contribution 採用、security outcome を約束しません。
- `fixtures/` はテスト入力です。実際の policy、license、support route として扱いません。

### 検証境界

変更後は次を確認します。

```powershell
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo +1.76.0 check --workspace --all-targets --locked
cargo run --quiet -p seiri-cli -- audit --path . --profile library --format markdown
cargo run --quiet -p seiri-cli -- codex --path . --profile library --query summary --format markdown
cargo run --quiet -p seiri-cli -- codex --path . --profile library --query evidence --format json
git diff --check
```

RepoSeiri のスコアや route state は、現在のリポジトリ状態に対するレビュー補助です。外部評価、実運用の安全性、法務判断、人気獲得を保証しません。

---

## English

RepoSeiri is a public repository for personal use and Rust coding practice. Its subject is a CLI / Codex plugin prototype that reads GitHub repository routes such as README, docs, license, security, support, and CI, then produces organization suggestions that are useful in Codex.

This repository is organized so public readers can understand its purpose with less ambiguity, but it is not a product intended for external use. RepoSeiri output is a review aid for personal use. It does not guarantee popularity, trust, safety, quality, legal fitness, or publication readiness.

### Three-Line Summary

- RepoSeiri is a public repository for personal use and Rust coding practice.
- It is a CLI / Codex plugin prototype that reads bounded local evidence and emits routes, content observations, and dry-run patch plans.
- Its output is a review aid, not a guarantee of quality, safety, trust, or publication readiness.

### What It Does

- Observes root files, README links, docs, GitHub workflows, issue templates, and similar repository signals.
- Emits route assessments, content slots, profile scores, and review priorities from typed evidence.
- Produces a dry-run patch plan that separates `Safe`, `Guarded`, and `Manual` boundaries instead of applying changes automatically.
- Generates summary, evidence, documents, governance, patches, and draft PR bodies as bounded Codex queries.

### Current Status

- This is a personal-use Rust coding practice repository.
- The implementation target is a CLI / Codex plugin prototype for repository organization.
- The Rust crates own the core audit, profile, pattern registry, calibration, and patch planning logic.
- The Codex plugin is a thin adapter that passes Rust CLI output into the Codex working context.
- Major human-facing documents keep Japanese in the first half and English in the second half, with the same content, decisions, and cautions.

### Quickstart

Rust 1.76 or newer is required.

```powershell
cargo test --workspace
cargo run --quiet -p seiri-cli -- codex --path . --profile library --query summary --format markdown
```

Run these two commands first. The first checks the workspace baseline, and the second inspects RepoSeiri itself through the Codex-oriented organization context.

**Example Output**

This is an abbreviated Codex summary for RepoSeiri itself. Counts change with repository state and do not guarantee external evaluation or safety.

```text
Schema: seiri.codex.v1
Query: summary
Evidence facts: <count>
Route assessments: 14
Patch operations / holds: <count> / <count>
```

### Main Commands

| Purpose | Command |
| --- | --- |
| Audit report | `cargo run --quiet -p seiri-cli -- audit --path . --profile library --format markdown` |
| Dry-run patch plan | `cargo run --quiet -p seiri-cli -- plan --path . --profile library --format markdown` |
| Codex summary | `cargo run --quiet -p seiri-cli -- codex --path . --profile library --query summary --format markdown` |
| Typed evidence JSON | `cargo run --quiet -p seiri-cli -- codex --path . --profile library --query evidence --format json` |
| Documents / governance | `cargo run --quiet -p seiri-cli -- codex --path . --profile library --query governance --format json` |
| Patch query | `cargo run --quiet -p seiri-cli -- codex --path . --profile library --query patches --format markdown` |
| Wording linter | `cargo run --quiet -p seiri-cli -- codex --path . --profile library --query linter --format markdown` |
| Codex PR body draft | `cargo run --quiet -p seiri-cli -- codex --path . --profile library --query pr-body --format markdown` |
| Pattern registry | `cargo run --quiet -p seiri-cli -- patterns --format markdown` |
| Calibration ingest | `cargo run --quiet -p seiri-cli -- calibrate --input fixtures/calibration-dataset.json --format markdown` |

### Reading Results

- `Verified` means an existence-checked repository-local target and matching structural evidence agree.
- `Structured` means structural evidence exists but the README route is not explicit.
- `Routed` means the README contains an entry point. It does not by itself indicate that a repository-local target exists.
- `Weak`, `Overloaded`, `Stale`, and `Conflicting` mean the entry point is thin, too broad, old, or ambiguous.
- `Absent` and `UnsafeToInvent` mean RepoSeiri should not create the route automatically, or that a human policy decision must come first.

### Repository Entry Points

| Topic | Entry |
| --- | --- |
| Project identity / README | [RepoSeiri README](README.md) |
| Documentation map | [Documentation Topology](docs/README.md) |
| License | [LICENSE](LICENSE) |
| Security reporting | [SECURITY.md](SECURITY.md) |
| Support route | [SUPPORT.md](SUPPORT.md) |
| Contribution route | [CONTRIBUTING.md](CONTRIBUTING.md) |
| Governance boundary | [GOVERNANCE.md](GOVERNANCE.md) |
| Release history | [CHANGELOG.md](CHANGELOG.md) |
| Lifecycle / maintenance boundary | [Lifecycle / Maintenance Boundary](docs/release.md) |
| Issue / PR intake | [.github/ISSUE_TEMPLATE](.github/ISSUE_TEMPLATE) |
| Ownership boundary | [.github/CODEOWNERS](.github/CODEOWNERS) |
| CI automation | [.github/workflows/ci.yml](.github/workflows/ci.yml) |
| Hygiene / self-audit | [Repository Hygiene](docs/hygiene.md) |

Detailed publication-state checks, design docs, and release procedures are routed through the docs topology.

### Codex Plugin Route

- Plugin root: `plugins/reposeiri`
- Skill file: [RepoSeiri Skill](plugins/reposeiri/skills/reposeiri/SKILL.md)
- In Codex, prefer the output from `seiri codex`.
- The plugin uses `seiri.codex.v1` queries. It does not invent repository policy and passes typed observations and the dry-run plan into the working context.

### Public Repository Boundary

- This repository is organized to be readable as a public repository. After publication, it remains scoped as personal-use coding practice work.
- The README owns only what the repository is, how to run it, and where to read next. Detailed design moves to docs.
- `SECURITY.md`, `SUPPORT.md`, and `CONTRIBUTING.md` are routing documents. They do not promise a fixed SLA, external contribution acceptance, or security outcomes.
- `fixtures/` contains test inputs. They are not treated as the real project policy, license, or support route.

### Verification Boundary

After changes, check the following.

```powershell
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo +1.76.0 check --workspace --all-targets --locked
cargo run --quiet -p seiri-cli -- audit --path . --profile library --format markdown
cargo run --quiet -p seiri-cli -- codex --path . --profile library --query summary --format markdown
cargo run --quiet -p seiri-cli -- codex --path . --profile library --query evidence --format json
git diff --check
```

RepoSeiri scores and route states are review aids for the current repository state. They do not guarantee external evaluation, production safety, legal judgment, or popularity.
