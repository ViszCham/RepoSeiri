# RepoSeiri

[![CI](https://github.com/ViszCham/RepoSeiri/actions/workflows/ci.yml/badge.svg)](https://github.com/ViszCham/RepoSeiri/actions/workflows/ci.yml)

## 日本語

RepoSeiri は、リポジトリの入口、文書、GitHub 設定、ローカル Git 構造を bounded local evidence から調べる Rust 製 CLI / Codex plugin です。

- 一度の source session から route、typed evidence、文書間整合、review priority を出します。
- 変更候補は existing-target-only の dry-run patch plan として出します。
- 標準監査はファイルを書かず、network や GitHub 操作を開始せず、policy を発明しません。

### Quickstart

Rust 1.88 以上が必要です。source checkout から次を実行します。

```powershell
git clone https://github.com/ViszCham/RepoSeiri.git
Set-Location RepoSeiri
cargo test --workspace --locked
cargo run --locked --quiet -p seiri-cli -- codex --path . --profile library --query summary --format markdown
```

最後のコマンドは RepoSeiri 自身を監査します。別のリポジトリを調べる場合は `--path` をその root へ変更します。

### 実出力例

次は tracked fixture `fixtures/readme-route-repo` に対する `summary` の全出力です。`tests/product_surface.rs` が同じ fixture から再生成し、README に掲載する値との drift を検出します。

```powershell
cargo run --locked --quiet -p seiri-cli -- codex --path fixtures/readme-route-repo --scope subtree --profile common --query summary --format markdown
```

```text
# RepoSeiri Codex Query

- Schema: `seiri.codex.v2`
- Repository: `.`
- Query: `summary`

- Entries: `13`
- Evidence facts: `78`
- Route assessments: `14`
- Content slots: `63`
- Findings: `0`
- Documents: `8` selected / `8` candidates; primary `8` / `8`
- Document budget skips: `0`; byte budget skips: `0`
- Coverage: `20` complete / `0` partial / `1` not requested; limit exceeded `0`
- Markdown coverage: `Complete`; conflict coverage: `Complete`
- Observations: `29` present / `46` absent / `1` unknown (`0` unacknowledged) / `0` conflict
- Patch operations: `1`
- Patch holds: `3`

- Boundary: Codex queries are bounded projections of canonical local analysis. They do not write files, execute commands, call GitHub, adopt policy, or guarantee popularity, trust, security, quality, or publication readiness.
```

`Findings: 0` は、表示された coverage と budget の範囲で finding がなかったことだけを表します。リポジトリ全体の品質、安全性、信頼性を証明しません。

### 主要な使い方

| 目的 | コマンド |
| --- | --- |
| 人間向け監査 | `cargo run --locked --quiet -p seiri-cli -- audit --path . --profile common --format markdown` |
| dry-run patch plan | `cargo run --locked --quiet -p seiri-cli -- plan --path . --profile common --format markdown` |
| Codex query | `cargo run --locked --quiet -p seiri-cli -- codex --path . --profile common --query summary --format markdown` |
| wording lint | `cargo run --locked --quiet -p seiri-cli -- lint-wording --path . --profile common --format markdown` |
| pattern registry | `cargo run --locked --quiet -p seiri-cli -- patterns --format markdown` |
| public calibration dataset | `cargo run --locked --quiet -p seiri-cli -- calibrate --input fixtures/calibration-dataset.json --format markdown` |
| public synthetic holdout | `cargo run --locked --quiet -p xtask -- calibration-holdout --format json` |
| machine contract | `cargo run --locked --quiet -p seiri-cli -- contract --format json` |
| completion evidence | `cargo run --locked --quiet -p xtask -- completion --format json` |

Codex query は次の10種類です。

`summary`, `routes`, `evidence`, `documents`, `governance`, `patches`, `linter`, `actions`, `remote`, `pr-body`

profile は `common`, `library`, `cli`, `infra`, `product`, `runtime`, `docs`, `tutorial`, `ml`, `research`, `template` です。通常は repository root で `--scope repository` を使います。

holdout report は route、wording、consistency、profile、planner のprecision、recall、false positive/negative、coverage、Wilson 95% interval、実行時間を出します。現在のtracked corpusは各task 4 holdout caseの低N回帰用なので、最低20 caseを満たさず`insufficient_sample`です。一般性能の校正結果ではありません。

### 出力の読み方

- `Verified` は、存在確認済みの repository-local target と対応する構造 evidence が一致した route state です。一般的な正しさの保証ではありません。
- `Structured` は構造 evidence があり、README route が明示されていない状態です。
- `Routed` は README に入口がある状態です。target の存在までは示しません。
- `Weak`, `Overloaded`, `Stale`, `Conflicting` は review が必要な route 状態です。
- `Absent`, `Unknown`, `UnsafeToInvent` は、それぞれ非観測、観測不足、人間の policy 判断が先に必要な状態を分けます。
- `Safe`, `Guarded`, `Manual` は dry-run operation の権限境界です。planner 自身は書き込みません。

### Rust 実装の焦点

- bounded filesystem traversal、bounded UTF-8 source read、byte-accurate source span
- framed SHA-256 identity、source-session binding、portable repository-relative evidence
- code fence、inline code、HTML comment、raw code を可視 prose から分離する Markdown event IR
- `Present`, `Absent`, `Unknown`, `Conflict`, `Disabled` を混同しない typed state
- private calibration body、exact prior、host absolute path を public artifact に出さない境界

低レイヤ設計、semantic revision、completion 条件は [Design Documentation](docs/design/README.md) にあります。これらは人気、信頼、安全性、品質、法的適合性、production readiness の保証ではありません。

### Codex plugin

plugin source は `plugins/reposeiri` にあります。launcher は `REPOSEIRI_BIN`、bundle-local binary、`PATH` の順に native runtime を解決し、contract、semantic revision、bundle manifest、binary SHA-256、同梱schema SHA-256を検証します。

plugin は Rust core の10 queryを使う薄い adapter です。query output は review data であり、file write、command execution、branch、commit、push、PR、merge の権限を付与しません。

### 文書と方針

| 読みたいもの | 入口 |
| --- | --- |
| 文書地図 | [Documentation Topology](docs/README.md) |
| 設計と Roadmap v10 | [Design Documentation](docs/design/README.md) |
| schema migration | [Migration v3](docs/migration-v3.md) |
| release | [Release Process](docs/release.md) |
| lifecycle | [Lifecycle Boundary](docs/lifecycle.md) |
| self-audit | [Self-Audit Loop](docs/self-audit.md) |
| security report | [SECURITY.md](SECURITY.md) |
| support | [SUPPORT.md](SUPPORT.md) |
| contribution | [CONTRIBUTING.md](CONTRIBUTING.md) |
| governance | [GOVERNANCE.md](GOVERNANCE.md) |
| change history | [CHANGELOG.md](CHANGELOG.md) |

RepoSeiri v1.0.0 は個人開発・Rust coding practice として公開しています。固定 SLA、release cadence、compatibility duration、外部 contribution 採用を約束しません。

---

## English

RepoSeiri is a Rust CLI and Codex plugin that inspects repository entry points, documents, GitHub configuration, and local Git structure from bounded local evidence.

- One source session produces routes, typed evidence, document consistency, and review priorities.
- Change candidates are emitted as an existing-target-only dry-run patch plan.
- Standard audits do not write files, initiate network or GitHub operations, or invent policy.

### Quickstart

Rust 1.88 or newer is required. Run the following from a source checkout.

```powershell
git clone https://github.com/ViszCham/RepoSeiri.git
Set-Location RepoSeiri
cargo test --workspace --locked
cargo run --locked --quiet -p seiri-cli -- codex --path . --profile library --query summary --format markdown
```

The final command audits RepoSeiri itself. To inspect another repository, change `--path` to its root.

### Real Output Example

The following is the complete `summary` output for the tracked `fixtures/readme-route-repo` fixture. `tests/product_surface.rs` regenerates it from the same fixture and detects drift from the values published in this README.

```powershell
cargo run --locked --quiet -p seiri-cli -- codex --path fixtures/readme-route-repo --scope subtree --profile common --query summary --format markdown
```

```text
# RepoSeiri Codex Query

- Schema: `seiri.codex.v2`
- Repository: `.`
- Query: `summary`

- Entries: `13`
- Evidence facts: `78`
- Route assessments: `14`
- Content slots: `63`
- Findings: `0`
- Documents: `8` selected / `8` candidates; primary `8` / `8`
- Document budget skips: `0`; byte budget skips: `0`
- Coverage: `20` complete / `0` partial / `1` not requested; limit exceeded `0`
- Markdown coverage: `Complete`; conflict coverage: `Complete`
- Observations: `29` present / `46` absent / `1` unknown (`0` unacknowledged) / `0` conflict
- Patch operations: `1`
- Patch holds: `3`

- Boundary: Codex queries are bounded projections of canonical local analysis. They do not write files, execute commands, call GitHub, adopt policy, or guarantee popularity, trust, security, quality, or publication readiness.
```

`Findings: 0` means only that no finding was emitted within the displayed coverage and budgets. It does not prove repository-wide quality, safety, or trustworthiness.

### Main Uses

| Purpose | Command |
| --- | --- |
| Human-readable audit | `cargo run --locked --quiet -p seiri-cli -- audit --path . --profile common --format markdown` |
| Dry-run patch plan | `cargo run --locked --quiet -p seiri-cli -- plan --path . --profile common --format markdown` |
| Codex query | `cargo run --locked --quiet -p seiri-cli -- codex --path . --profile common --query summary --format markdown` |
| Wording lint | `cargo run --locked --quiet -p seiri-cli -- lint-wording --path . --profile common --format markdown` |
| Pattern registry | `cargo run --locked --quiet -p seiri-cli -- patterns --format markdown` |
| Public calibration dataset | `cargo run --locked --quiet -p seiri-cli -- calibrate --input fixtures/calibration-dataset.json --format markdown` |
| Public synthetic holdout | `cargo run --locked --quiet -p xtask -- calibration-holdout --format json` |
| Machine contract | `cargo run --locked --quiet -p seiri-cli -- contract --format json` |
| Completion evidence | `cargo run --locked --quiet -p xtask -- completion --format json` |

The ten Codex query kinds are:

`summary`, `routes`, `evidence`, `documents`, `governance`, `patches`, `linter`, `actions`, `remote`, `pr-body`

Profiles are `common`, `library`, `cli`, `infra`, `product`, `runtime`, `docs`, `tutorial`, `ml`, `research`, and `template`. Normally, use `--scope repository` from the repository root.

The holdout report emits precision, recall, false positives/negatives, coverage, a Wilson 95% interval, and runtime for routes, wording, consistency, profiles, and planning. The tracked corpus currently has four holdout cases per task, below the minimum of 20, so it remains `insufficient_sample`. It is regression data, not general performance calibration.

### Reading Output

- `Verified` is a route state where an existence-checked repository-local target agrees with matching structural evidence. It is not a general correctness guarantee.
- `Structured` means structural evidence exists without an explicit README route.
- `Routed` means the README contains an entry point. It does not establish that the target exists.
- `Weak`, `Overloaded`, `Stale`, and `Conflicting` are route states that require review.
- `Absent`, `Unknown`, and `UnsafeToInvent` separate non-observation, insufficient observation, and cases where a human policy decision must come first.
- `Safe`, `Guarded`, and `Manual` are authority boundaries for dry-run operations. The planner itself does not write.

### Rust Implementation Focus

- Bounded filesystem traversal, bounded UTF-8 source reads, and byte-accurate source spans
- Framed SHA-256 identities, source-session binding, and portable repository-relative evidence
- A Markdown event IR that separates code fences, inline code, HTML comments, and raw code from visible prose
- Typed `Present`, `Absent`, `Unknown`, `Conflict`, and `Disabled` states
- Boundaries that keep private calibration bodies, exact priors, and host absolute paths out of public artifacts

Low-level design, semantic revisions, and completion conditions are in [Design Documentation](docs/design/README.md). They are not guarantees of popularity, trust, security, quality, legal fitness, or production readiness.

### Codex Plugin

Plugin source lives in `plugins/reposeiri`. The launcher resolves the native runtime in the order `REPOSEIRI_BIN`, bundle-local binary, then `PATH`, and validates the contract, semantic revisions, bundle manifest, binary SHA-256, and bundled-schema SHA-256 values.

The plugin is a thin adapter over the ten Rust-core queries. Query output is review data and does not grant authority to write files, execute commands, create branches, commit, push, open PRs, or merge.

### Documentation And Policy

| Topic | Entry |
| --- | --- |
| Documentation map | [Documentation Topology](docs/README.md) |
| Design and Roadmap v10 | [Design Documentation](docs/design/README.md) |
| Schema migration | [Migration v3](docs/migration-v3.md) |
| Release | [Release Process](docs/release.md) |
| Lifecycle | [Lifecycle Boundary](docs/lifecycle.md) |
| Self-audit | [Self-Audit Loop](docs/self-audit.md) |
| Security reporting | [SECURITY.md](SECURITY.md) |
| Support | [SUPPORT.md](SUPPORT.md) |
| Contributions | [CONTRIBUTING.md](CONTRIBUTING.md) |
| Governance | [GOVERNANCE.md](GOVERNANCE.md) |
| Change history | [CHANGELOG.md](CHANGELOG.md) |

RepoSeiri v1.0.0 is public as personal development and Rust coding practice. It does not promise a fixed SLA, release cadence, compatibility duration, or acceptance of external contributions.
