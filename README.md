# RepoSeiri

[![CI](https://github.com/ViszCham/RepoSeiri/actions/workflows/ci.yml/badge.svg)](https://github.com/ViszCham/RepoSeiri/actions/workflows/ci.yml)

## 日本語

RepoSeiri は Rust 実装の Codex plugin / CLI です。GitHub リポジトリを低レイヤの証拠から読み取り、Repository Trust Graph、profile branch confidence、missing route priority、safe patch plan、Codex review context を生成します。

### 固定前提

- プロダクト名とリポジトリ名は `RepoSeiri` です。
- 実装言語は Rust です。監査、profile、pattern registry、calibration、patch planning の主要ロジックは Rust crate 側で持ちます。
- Codex plugin は薄い adapter として扱い、Rust CLI の結果を Codex の作業文脈へ渡します。
- 人間向けの主要ドキュメントは、日本語を前半、英語を後半に置き、同じ内容、同じ判断、同じ注意点を保ちます。
- README は詳細説明を抱え込まず、最初に読むための route hub として維持します。
- RepoSeiri の出力はレビュー用の決定論的な整理案です。人気、信頼、安全性、品質、法務適合を保証するものではありません。

### Quickstart

```powershell
cargo test --workspace
cargo run --quiet -p seiri-cli -- codex --path . --profile library --format markdown
```

この2行を、RepoSeiri 自身を確認する最初の route とします。詳細を読む前に、test が通るか、Codex 向けの route review がどう出るかを確認します。

### 主要コマンド

| 目的 | コマンド |
| --- | --- |
| 監査レポート | `cargo run --quiet -p seiri-cli -- audit --path . --profile library --format markdown` |
| dry-run patch plan | `cargo run --quiet -p seiri-cli -- plan --path . --profile library --format markdown` |
| Codex review context | `cargo run --quiet -p seiri-cli -- codex --path . --profile library --format markdown` |
| Codex PR body draft | `cargo run --quiet -p seiri-cli -- codex --path . --profile library --view pr-body --format markdown` |
| pattern registry | `cargo run --quiet -p seiri-cli -- patterns --format markdown` |
| calibration ingest | `cargo run --quiet -p seiri-cli -- calibrate --input fixtures/calibration-dataset.json --format markdown` |

### Profile

`--profile` は `common`、`library`、`cli`、`infra`、`product`、`runtime`、`docs`、`tutorial`、`ml`、`research`、`template` を受け付けます。RepoSeiri 自身には、Rust library workspace と Codex plugin の両方を持つため、まず `library` を使います。

### 結果の読み方

- `Verified` は、root file などの構造的証拠と README route が揃っている状態です。
- `Structured` は、構造的証拠はあるが README route が明示されていない状態です。
- `Routed` は、README 内に入口がある状態です。
- `Weak`、`Overloaded`、`Stale`、`Conflicting` は、入口が薄い、多すぎる、古い、または意図が曖昧な状態です。
- `Absent` と `UnsafeToInvent` は、RepoSeiri が自動生成すべきではない、または人間の方針決定が先に必要な状態です。

### Codex plugin route

- Plugin root: `plugins/reposeiri`
- Skill file: [RepoSeiri Skill](plugins/reposeiri/skills/reposeiri/SKILL.md)
- Codex 側では `seiri codex` の出力を優先して使います。
- plugin は repository policy を推測で作らず、Rust CLI が出した gate と safe patch plan を作業文脈へ渡します。

### Repository routes

| Route | 現在の入口 |
| --- | --- |
| Documentation topology | [Documentation Topology](docs/README.md) |
| Roadmap and implementation blocks | [Roadmap and Implementation Blocks](docs/design/roadmap-and-implementation-blocks.md) |
| License | [LICENSE](LICENSE) |
| Security | [SECURITY.md](SECURITY.md) |
| Release | [CHANGELOG.md](CHANGELOG.md) |
| Support | [SUPPORT.md](SUPPORT.md) |
| Contribution | [CONTRIBUTING.md](CONTRIBUTING.md) |
| Issue / PR intake | [.github/ISSUE_TEMPLATE](.github/ISSUE_TEMPLATE) |
| Hygiene / self-audit | [Repository Hygiene](docs/hygiene.md) |

詳細設計は docs topology から design docs へ分岐します。Automation は CI workflow と Dependabot config を置きます。Release root route は `CHANGELOG.md` とし、手順は `docs/release.md` に分けます。Hygiene は `docs/hygiene.md` を入口とし、self-audit loop は `docs/self-audit.md` に分けます。未作成ファイルへのリンクは置きません。

### README route 方針

- Quickstart は、最初に実行する route として1箇所に集約します。
- README では全コマンドを長く説明せず、主要コマンド表と docs topology への入口だけを置きます。
- root policy は `LICENSE` と `SECURITY.md` を正とし、README はそこへ送ります。
- Release は `CHANGELOG.md` を正とし、詳細手順は release docs に逃がします。
- Automation は badge と CI workflow への入口に留め、Dependabot config は root evidence として扱います。
- 未決定の運用 route は README で作ったことにせず、後続ブロックで root file と一緒に追加します。

### 検証境界

変更後は次を確認します。

```powershell
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run --quiet -p seiri-cli -- audit --path . --profile library --format markdown
git diff --check
```

RepoSeiri のスコアや route state は、現在のリポジトリ状態に対するレビュー補助です。外部評価、実運用の安全性、法務判断、人気獲得を保証しません。

---

## English

RepoSeiri is a Rust Codex plugin / CLI. It reads GitHub repositories from low-level evidence and produces a Repository Trust Graph, profile branch confidence, missing route priority, safe patch plan, and Codex review context.

### Fixed Premises

- The product and repository name is `RepoSeiri`.
- The implementation language is Rust. The core audit, profile, pattern registry, calibration, and patch planning logic belongs in Rust crates.
- The Codex plugin is a thin adapter that passes Rust CLI output into the Codex working context.
- Major human-facing documents keep Japanese in the first half and English in the second half, with the same content, decisions, and cautions.
- The README stays as a first-read route hub instead of absorbing detailed design material.
- RepoSeiri output is a deterministic review aid. It does not guarantee popularity, trust, safety, quality, or legal fitness.

### Quickstart

```powershell
cargo test --workspace
cargo run --quiet -p seiri-cli -- codex --path . --profile library --format markdown
```

These two lines are the first route for checking RepoSeiri itself. Before reading the details, confirm that tests pass and inspect the Codex-oriented route review.

### Main Commands

| Purpose | Command |
| --- | --- |
| Audit report | `cargo run --quiet -p seiri-cli -- audit --path . --profile library --format markdown` |
| Dry-run patch plan | `cargo run --quiet -p seiri-cli -- plan --path . --profile library --format markdown` |
| Codex review context | `cargo run --quiet -p seiri-cli -- codex --path . --profile library --format markdown` |
| Codex PR body draft | `cargo run --quiet -p seiri-cli -- codex --path . --profile library --view pr-body --format markdown` |
| Pattern registry | `cargo run --quiet -p seiri-cli -- patterns --format markdown` |
| Calibration ingest | `cargo run --quiet -p seiri-cli -- calibrate --input fixtures/calibration-dataset.json --format markdown` |

### Profile

`--profile` accepts `common`, `library`, `cli`, `infra`, `product`, `runtime`, `docs`, `tutorial`, `ml`, `research`, and `template`. For RepoSeiri itself, start with `library` because the repository is both a Rust library workspace and a Codex plugin.

### Reading Results

- `Verified` means structural evidence such as a root file and README routing agree.
- `Structured` means structural evidence exists but the README route is not explicit.
- `Routed` means the README contains an entry point.
- `Weak`, `Overloaded`, `Stale`, and `Conflicting` mean the entry point is thin, too broad, old, or ambiguous.
- `Absent` and `UnsafeToInvent` mean RepoSeiri should not create the route automatically, or that a human policy decision must come first.

### Codex plugin route

- Plugin root: `plugins/reposeiri`
- Skill file: [RepoSeiri Skill](plugins/reposeiri/skills/reposeiri/SKILL.md)
- In Codex, prefer the output from `seiri codex`.
- The plugin should not invent repository policy. It passes the Rust CLI gates and safe patch plan into the working context.

### Repository routes

| Route | Current entry |
| --- | --- |
| Documentation topology | [Documentation Topology](docs/README.md) |
| Roadmap and implementation blocks | [Roadmap and Implementation Blocks](docs/design/roadmap-and-implementation-blocks.md) |
| License | [LICENSE](LICENSE) |
| Security | [SECURITY.md](SECURITY.md) |
| Release | [CHANGELOG.md](CHANGELOG.md) |
| Support | [SUPPORT.md](SUPPORT.md) |
| Contribution | [CONTRIBUTING.md](CONTRIBUTING.md) |
| Issue / PR intake | [.github/ISSUE_TEMPLATE](.github/ISSUE_TEMPLATE) |
| Hygiene / self-audit | [Repository Hygiene](docs/hygiene.md) |

Detailed design branches from docs topology into design docs. Automation has a CI workflow and Dependabot config. The Release root route is `CHANGELOG.md`, and the procedure lives in `docs/release.md`. Hygiene uses `docs/hygiene.md` as the entry, and the self-audit loop lives in `docs/self-audit.md`. The README does not link to files that do not exist.

### README route policy

- Quickstart is consolidated into one first-run route.
- The README does not explain every command at length. It keeps a main command table and an entry to docs topology.
- Root policy is owned by `LICENSE` and `SECURITY.md`; the README routes readers there.
- Release is owned by `CHANGELOG.md`; detailed procedure moves to release docs.
- Automation stays limited to the badge and CI workflow entry; the Dependabot config is treated as root evidence.
- Undecided operational routes are not treated as created by the README. They are added later together with root files.

### Verification Boundary

After changes, check the following.

```powershell
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run --quiet -p seiri-cli -- audit --path . --profile library --format markdown
git diff --check
```

RepoSeiri scores and route states are review aids for the current repository state. They do not guarantee external evaluation, production safety, legal judgment, or popularity.
