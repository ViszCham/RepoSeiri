# Self-Audit Loop

## 日本語

RepoSeiri の self-audit loop は、RepoSeiri 自身を RepoSeiri で読み直すための固定 route です。local check、CI check、Codex review context、manual review を分けます。

この loop は自己承認ではありません。CI、RepoSeiri score、Codex draft、patch plan は review aid であり、release、security、ownership、legal、quality の最終判断を自動化しません。

### Local loop

```powershell
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo +1.76.0 check --workspace --all-targets --locked
cargo run --quiet -p seiri-cli -- audit --path . --profile library --format markdown
cargo run --quiet -p seiri-cli -- plan --path . --profile library --format markdown
cargo run --quiet -p seiri-cli -- codex --path . --profile library --format markdown
cargo run --quiet -p seiri-cli -- codex --path . --profile library --schema native-v2 --format json
cargo run --quiet -p seiri-cli -- codex --path . --profile library --view linter --format markdown
git diff --check
```

### CI loop

CI は次を実行します。

| Step | Output | Use |
| --- | --- | --- |
| format / test / clippy / Rust 1.76 check | job result | Rust workspace の基本的な regression と MSRV compile を見る。 |
| `seiri audit` | `audit.md` artifact | route state、profile、missing route priority を見る。 |
| `seiri plan` | `plan.md` artifact | safe / guarded / manual gate を見る。 |
| `seiri codex` | `codex.md` artifact | Codex review context と PR draft surface を見る。 |
| `seiri codex --schema native-v2` | `codex-native-v2.json` artifact | typed evidence、route assessment、claim、patch proposal を見る。 |
| `seiri codex --view linter` | `codex-linter.md` artifact | evidence-scoped wording findings を見る。 |

### Review loop

1. README route map に `overloaded`、`stale`、`conflicting` が出た場合は、README から docs topology へ逃がすか、target link を修正します。
2. `UnsafeToInvent`、`Manual`、security、ownership、license は自動修正しません。
3. `Guarded` draft は maintainer が内容を確認してから file 化します。
4. score が上がっても品質保証とは書きません。score が下がった場合は、どの route evidence が消えたかを先に確認します。

---

## English

The RepoSeiri self-audit loop is the fixed route for reading RepoSeiri with RepoSeiri itself. It separates local checks, CI checks, Codex review context, and manual review.

This loop is not self-approval. CI, RepoSeiri scores, Codex drafts, and patch plans are review aids; they do not automate release, security, ownership, legal, or quality decisions.

### Local loop

```powershell
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo +1.76.0 check --workspace --all-targets --locked
cargo run --quiet -p seiri-cli -- audit --path . --profile library --format markdown
cargo run --quiet -p seiri-cli -- plan --path . --profile library --format markdown
cargo run --quiet -p seiri-cli -- codex --path . --profile library --format markdown
cargo run --quiet -p seiri-cli -- codex --path . --profile library --schema native-v2 --format json
cargo run --quiet -p seiri-cli -- codex --path . --profile library --view linter --format markdown
git diff --check
```

### CI loop

CI runs the following.

| Step | Output | Use |
| --- | --- | --- |
| format / test / clippy / Rust 1.76 check | job result | Review basic Rust workspace regression and MSRV compilation. |
| `seiri audit` | `audit.md` artifact | Review route state, profile, and missing route priority. |
| `seiri plan` | `plan.md` artifact | Review safe / guarded / manual gates. |
| `seiri codex` | `codex.md` artifact | Review Codex review context and PR draft surface. |
| `seiri codex --schema native-v2` | `codex-native-v2.json` artifact | Review typed evidence, route assessments, claims, and patch proposals. |
| `seiri codex --view linter` | `codex-linter.md` artifact | Review evidence-scoped wording findings. |

### Review loop

1. If the README route map emits `overloaded`, `stale`, or `conflicting`, move material from README into docs topology or fix the target link.
2. Do not auto-fix `UnsafeToInvent`, `Manual`, security, ownership, or license decisions.
3. File a `Guarded` draft only after a maintainer reviews the content.
4. Do not describe a higher score as a quality guarantee. If a score drops, first inspect which route evidence disappeared.
