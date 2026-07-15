# RepoSeiri

[![CI](https://github.com/ViszCham/RepoSeiri/actions/workflows/ci.yml/badge.svg)](https://github.com/ViszCham/RepoSeiri/actions/workflows/ci.yml)

## 日本語

RepoSeiri は、GitHub リポジトリの構造と導線を bounded local evidence から解析する Rust 製 CLI / Codex plugin です。標準監査はローカルで完結し、repository files を書き換えず、確認済み事実と提案を分離して出力します。

### 3行要約

- root files、README links、docs、Git-local metadata、GitHub configuration を bounded scope で観測します。
- 一度の canonical analysis から route、typed evidence、文書間整合、review priority、dry-run patch plan を生成します。
- 結果を10種類の Codex query で取り出し、確認済み事実、提案、人間による判断、evidence boundary外の事項を分離します。

### 何をするものか

- リポジトリ内の root files、README links、docs、Git metadata、GitHub workflow、issue templates などを観測します。
- 観測した typed evidence から一度だけ canonical analysis を構築し、route assessment、content slot、profile score、review priority を出します。
- 自動適用ではなく、`Safe`、`Guarded`、`Manual` の境界を分けた dry-run patch plan を出します。
- `summary`、`routes`、`evidence`、`documents`、`governance`、`patches`、`linter`、`actions`、`remote`、`pr-body`を bounded Codex query として生成します。

### 設計上の特徴

- **Rust-first low-level boundaries:** bounded filesystem traversal、Git-local metadata、UTF-8、byte span、決定的ID、compact maskを型で扱い、workspaceのcrate rootでは`unsafe`を禁止します。
- **Evidence-closed decisions:** routeの存在、contentの存在、coverage、`Unknown`を分離し、部分的な観測を「存在しない」へ昇格させません。
- **Stable machine-readable contracts:** `seiri.analysis.v2`、`seiri.patch-plan.v2`、`seiri.codex.v2`を公開し、v1 aliasや暗黙変換を受け付けません。
- **Bounded authority:** 標準監査はremote accessを開始せず、plannerは既存のrepository-local targetに対するdry-runだけを生成します。
- **Standalone Codex adapter:** bundle-local binary、runtime manifest、SHA-256を持つplugin bundleを生成し、Windows/Linux CIでlauncher smokeとcompletion gateを実行します。

### 現在の位置づけ

- 個人開発と Rust coding practice を起点に継続している公開リポジトリです。
- 現在の v1.0.0 source contract は、repository organization を支援する CLI、v2 schema、standalone Codex plugin adapter を固定しています。
- Rust crate 側に監査、profile、pattern registry、calibration、patch planning の主要ロジックを置きます。
- Codex plugin は薄い adapter とし、Rust CLI の結果を Codex の作業文脈へ渡します。
- 人間向けの主要ドキュメントは、日本語を前半、英語を後半に置き、同じ内容、同じ判断、同じ注意点を保ちます。

### Quickstart

必要環境は Rust 1.88 以上です。

```powershell
cargo test --workspace
cargo run --quiet -p seiri-cli -- codex --path . --profile library --query summary --format markdown
```

まずこの2つを実行します。1行目で workspace の基本動作を確認し、2行目で RepoSeiri 自身を対象にした Codex 向け整理案を確認します。

**Example Output**

RepoSeiri 自身を対象にした `governance` と `patches` の短縮例です。件数はリポジトリ状態で変わります。

```text
Schema: seiri.codex.v2
Query: governance

Evidence-Backed Claims
- The audit observed the `docs` route and found its repository-local target present.
- The audit observed the `automation` route and found its repository-local target present.

Query: patches
Dry-run operations: <count>
Held items: <count>
Writes files: false
```

### 主要コマンド

| 目的 | コマンド |
| --- | --- |
| 監査レポート | `cargo run --quiet -p seiri-cli -- audit --path . --profile library --format markdown` |
| dry-run patch plan | `cargo run --quiet -p seiri-cli -- plan --path . --profile library --format markdown` |
| Codex summary | `cargo run --quiet -p seiri-cli -- codex --path . --profile library --query summary --format markdown` |
| route assessment | `cargo run --quiet -p seiri-cli -- codex --path . --profile library --query routes --format json` |
| typed evidence JSON | `cargo run --quiet -p seiri-cli -- codex --path . --profile library --query evidence --format json` |
| documents | `cargo run --quiet -p seiri-cli -- codex --path . --profile library --query documents --format json` |
| governance | `cargo run --quiet -p seiri-cli -- codex --path . --profile library --query governance --format json` |
| patch query | `cargo run --quiet -p seiri-cli -- codex --path . --profile library --query patches --format markdown` |
| wording linter | `cargo run --quiet -p seiri-cli -- codex --path . --profile library --query linter --format markdown` |
| typed action suggestions | `cargo run --quiet -p seiri-cli -- codex --path . --profile library --query actions --format json` |
| remote terminal state | `cargo run --quiet -p seiri-cli -- codex --path . --profile library --query remote --format markdown` |
| Codex PR body draft | `cargo run --quiet -p seiri-cli -- codex --path . --profile library --query pr-body --format markdown` |
| pattern registry | `cargo run --quiet -p seiri-cli -- patterns --format markdown` |
| calibration ingest | `cargo run --quiet -p seiri-cli -- calibrate --input fixtures/calibration-dataset.json --format markdown` |
| contract確認 | `cargo run --quiet -p seiri-cli -- contract --format json` |
| completion gate | `cargo run --quiet -p xtask -- completion --format json` |

ローカルcompletionはrequired host evidenceが不足していれば`incomplete`を返します。CIはWindows/Linux bundle smokeとchecksum manifestを収集し、`--host-evidence`付きで最終gateを実行します。

### 結果の読み方

- `Verified` は、存在確認済みの repository-local target と対応する構造的証拠が揃っている状態です。
- `Structured` は、構造的証拠はあるが README route が明示されていない状態です。
- `Routed` は、README 内に入口がある状態です。これだけでは repository-local target の存在を示しません。
- `Weak`、`Overloaded`、`Stale`、`Conflicting` は、入口が薄い、多すぎる、古い、または意図が曖昧な状態です。
- `Absent` と `UnsafeToInvent` は、RepoSeiri が自動生成すべきではない、または人間の方針決定が先に必要な状態です。
- observed claim は、evidence が支える肯定文を先に表示し、そのclaimに関連するboundaryだけを続けます。

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
- plugin は `seiri.codex.v2` の10種類のqueryを使います。標準監査はremote accessを開始しません。launcherは`REPOSEIRI_BIN`、bundle-local binary、`PATH`の順でstandalone runtimeを解決し、repository policyを推測で作らず、Rust CLIのtyped observationとdry-run planを作業文脈へ渡します。

### 公開リポジトリとしての境界

- RepoSeiri は個人開発・Rust coding practice として継続します。実行可能な CLI / plugin であることと、固定 SLA、release cadence、compatibility duration を約束することは別です。
- README は「何のリポジトリか」「どう動かすか」「どこを読むか」だけを持ち、詳細設計は docs に逃がします。
- `SECURITY.md`、`SUPPORT.md`、`CONTRIBUTING.md` は案内用です。固定 SLA、外部 contribution 採用、security outcome を約束しません。
- `fixtures/` はテスト入力です。実際の policy、license、support route として扱いません。

### 検証境界

変更後は次を確認します。

```powershell
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo +1.88.0 check --workspace --all-targets --locked
cargo audit
cargo run --quiet -p seiri-cli -- audit --path . --profile library --format markdown
cargo run --quiet -p seiri-cli -- codex --path . --profile library --query summary --format markdown
cargo run --quiet -p seiri-cli -- codex --path . --profile library --query evidence --format json
cargo run --quiet -p xtask -- completion --format json
git diff --check
```

RepoSeiri の claim、score、route state は、現在の bounded local evidence に対する観測とレビュー補助です。partial coverage は `Unknown` のまま保持します。人気、信頼、安全性、品質、法務適合、保守、production readiness、publication readiness を保証しません。

---

## English

RepoSeiri is a Rust CLI and Codex plugin that analyzes GitHub repository structure and navigation from bounded local evidence. Standard audits stay local, do not write repository files, and emit verified facts separately from suggestions.

### Three-Line Summary

- It observes root files, README links, docs, Git-local metadata, and GitHub configuration within a bounded scope.
- One canonical analysis produces routes, typed evidence, document consistency, review priorities, and dry-run patch plans.
- Ten Codex queries keep verified facts, suggestions, maintainer decisions, and outcomes outside the evidence boundary separate.

### What It Does

- Observes root files, README links, docs, Git metadata, GitHub workflows, issue templates, and similar repository signals.
- Builds canonical analysis once from observed typed evidence, then emits route assessments, content slots, profile scores, and review priorities.
- Produces a dry-run patch plan that separates `Safe`, `Guarded`, and `Manual` boundaries instead of applying changes automatically.
- Generates `summary`, `routes`, `evidence`, `documents`, `governance`, `patches`, `linter`, `actions`, `remote`, and `pr-body` as bounded Codex queries.

### Design Highlights

- **Rust-first low-level boundaries:** Types represent bounded filesystem traversal, Git-local metadata, UTF-8, byte spans, deterministic IDs, and compact masks. Workspace crate roots forbid `unsafe`.
- **Evidence-closed decisions:** Route presence, content presence, coverage, and `Unknown` remain separate. Partial observation is not promoted to absence.
- **Stable machine-readable contracts:** RepoSeiri exposes `seiri.analysis.v2`, `seiri.patch-plan.v2`, and `seiri.codex.v2`, while rejecting v1 aliases and silent conversions.
- **Bounded authority:** Standard audits do not initiate remote access, and the planner emits only dry-run proposals for existing repository-local targets.
- **Standalone Codex adapter:** Plugin bundles include a bundle-local binary, runtime manifest, and SHA-256 value. Windows/Linux CI runs launcher smoke and the completion gate.

### Current Status

- This public repository continues as personal development and Rust coding practice.
- The current v1.0.0 source contract freezes the repository-organization CLI, v2 schemas, and standalone Codex plugin adapter.
- The Rust crates own the core audit, profile, pattern registry, calibration, and patch planning logic.
- The Codex plugin is a thin adapter that passes Rust CLI output into the Codex working context.
- Major human-facing documents keep Japanese in the first half and English in the second half, with the same content, decisions, and cautions.

### Quickstart

Rust 1.88 or newer is required.

```powershell
cargo test --workspace
cargo run --quiet -p seiri-cli -- codex --path . --profile library --query summary --format markdown
```

Run these two commands first. The first checks the workspace baseline, and the second inspects RepoSeiri itself through the Codex-oriented organization context.

**Example Output**

This is abbreviated output from the `governance` and `patches` queries against RepoSeiri itself. Counts change with repository state.

```text
Schema: seiri.codex.v2
Query: governance

Evidence-Backed Claims
- The audit observed the `docs` route and found its repository-local target present.
- The audit observed the `automation` route and found its repository-local target present.

Query: patches
Dry-run operations: <count>
Held items: <count>
Writes files: false
```

### Main Commands

| Purpose | Command |
| --- | --- |
| Audit report | `cargo run --quiet -p seiri-cli -- audit --path . --profile library --format markdown` |
| Dry-run patch plan | `cargo run --quiet -p seiri-cli -- plan --path . --profile library --format markdown` |
| Codex summary | `cargo run --quiet -p seiri-cli -- codex --path . --profile library --query summary --format markdown` |
| Route assessments | `cargo run --quiet -p seiri-cli -- codex --path . --profile library --query routes --format json` |
| Typed evidence JSON | `cargo run --quiet -p seiri-cli -- codex --path . --profile library --query evidence --format json` |
| Documents | `cargo run --quiet -p seiri-cli -- codex --path . --profile library --query documents --format json` |
| Governance | `cargo run --quiet -p seiri-cli -- codex --path . --profile library --query governance --format json` |
| Patch query | `cargo run --quiet -p seiri-cli -- codex --path . --profile library --query patches --format markdown` |
| Wording linter | `cargo run --quiet -p seiri-cli -- codex --path . --profile library --query linter --format markdown` |
| Typed action suggestions | `cargo run --quiet -p seiri-cli -- codex --path . --profile library --query actions --format json` |
| Remote terminal state | `cargo run --quiet -p seiri-cli -- codex --path . --profile library --query remote --format markdown` |
| Codex PR body draft | `cargo run --quiet -p seiri-cli -- codex --path . --profile library --query pr-body --format markdown` |
| Pattern registry | `cargo run --quiet -p seiri-cli -- patterns --format markdown` |
| Calibration ingest | `cargo run --quiet -p seiri-cli -- calibrate --input fixtures/calibration-dataset.json --format markdown` |
| Contract manifest | `cargo run --quiet -p seiri-cli -- contract --format json` |
| Completion gate | `cargo run --quiet -p xtask -- completion --format json` |

Local completion returns `incomplete` when required host evidence is missing. CI collects Windows/Linux bundle-smoke and checksum manifests, then runs the final gate with `--host-evidence`.

### Reading Results

- `Verified` means an existence-checked repository-local target and matching structural evidence agree.
- `Structured` means structural evidence exists but the README route is not explicit.
- `Routed` means the README contains an entry point. It does not by itself indicate that a repository-local target exists.
- `Weak`, `Overloaded`, `Stale`, and `Conflicting` mean the entry point is thin, too broad, old, or ambiguous.
- `Absent` and `UnsafeToInvent` mean RepoSeiri should not create the route automatically, or that a human policy decision must come first.
- Observed claims state the evidence-backed positive proposition first, followed only by boundaries relevant to that claim.

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
- The plugin uses the ten `seiri.codex.v2` queries. Standard audits do not initiate remote access. Its launcher resolves the standalone runtime in the order `REPOSEIRI_BIN`, bundle-local binary, then `PATH`. It does not invent repository policy and passes typed observations and the dry-run plan into the working context.

### Public Repository Boundary

- RepoSeiri continues as personal development and Rust coding practice. Being an executable CLI / plugin is separate from promising a fixed SLA, release cadence, or compatibility duration.
- The README owns only what the repository is, how to run it, and where to read next. Detailed design moves to docs.
- `SECURITY.md`, `SUPPORT.md`, and `CONTRIBUTING.md` are routing documents. They do not promise a fixed SLA, external contribution acceptance, or security outcomes.
- `fixtures/` contains test inputs. They are not treated as the real project policy, license, or support route.

### Verification Boundary

After changes, check the following.

```powershell
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo +1.88.0 check --workspace --all-targets --locked
cargo audit
cargo run --quiet -p seiri-cli -- audit --path . --profile library --format markdown
cargo run --quiet -p seiri-cli -- codex --path . --profile library --query summary --format markdown
cargo run --quiet -p seiri-cli -- codex --path . --profile library --query evidence --format json
cargo run --quiet -p xtask -- completion --format json
git diff --check
```

RepoSeiri claims, scores, and route states are observations and review aids for the current bounded local evidence. Partial coverage remains `Unknown`. They do not guarantee popularity, trust, security, quality, legal fitness, maintenance, production readiness, or publication readiness.
