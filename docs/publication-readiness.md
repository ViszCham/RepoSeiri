# Publication Readiness

## 日本語

この文書は、RepoSeiri を private のまま公開直前まで整理するための checklist です。公開操作そのもの、repository visibility の変更、法務判断、security outcome の保証は扱いません。

RepoSeiri は個人使用目的の Rust coding practice repository です。公開する場合も、まず「個人使用・コーディング練習リポジトリであること」「何をする CLI / Codex plugin か」「何を保証しないか」を明確にしてから visibility を変えます。

### 公開前に満たしたい状態

| 項目 | 確認内容 |
| --- | --- |
| README | 初見で、個人使用目的の Rust コーディング練習リポであり、repository organization 用 CLI / Codex plugin prototype だと分かる。 |
| 実行方法 | `cargo test --workspace` と `seiri-cli codex` の最初の route がある。 |
| license | root `LICENSE`、`LICENSE-MIT`、`LICENSE-APACHE` があり、workspace license と矛盾しない。 |
| security | 未修正の脆弱性を public issue に書かない route がある。 |
| support | 固定 SLA や解決保証を約束しない support route がある。 |
| contribution | 外部 contribution の採用、review、merge を保証しない contribution route がある。 |
| ownership | `.github/CODEOWNERS` で個人リポとしての所有境界を示す。 |
| automation | CI と Dependabot があり、ただし品質保証として表現しない。 |
| fixtures | `fixtures/` が実 policy や real user data ではなく、テスト入力だと分かる。 |
| claims | 人気、信頼、安全性、品質、法務適合、公開可否を保証する表現がない。 |

### 公開前チェック

```powershell
gh repo view ViszCham/RepoSeiri --json visibility,isPrivate
rg -n -i "(token|secret|password|api[_-]?key|private[_-]?key|credential|github_pat_|ghp_|BEGIN .*PRIVATE KEY)" --glob "!target/**" --glob "!Cargo.lock"
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run --quiet -p seiri-cli -- audit --path . --profile library --format markdown
cargo run --quiet -p seiri-cli -- codex --path . --profile library --format markdown
git diff --check
```

`rg` の結果に、文書内の注意書きではなく実際の secret 候補が出た場合は、公開判断の前に secret rotation と履歴対応を別作業として扱います。

### まだ約束しないこと

- crates.io や plugin marketplace への公開。
- 外部利用者への support SLA。
- security program の完全性。
- RepoSeiri score による品質保証。
- GitHub visibility の自動変更。

### visibility を変える前の最終確認

1. GitHub repository description と topics が、個人使用・コーディング練習リポであることを誤解なく示しているか確認します。
2. README の日本語前半と英語後半が同じ内容、同じ判断、同じ注意点になっているか確認します。
3. `SECURITY.md`、`SUPPORT.md`、`CONTRIBUTING.md` が過剰な約束をしていないか確認します。
4. RepoSeiri audit の guarded / manual decision を、人間が判断するべきものとして残しているか確認します。
5. visibility 変更は、別の明示タスクとして実行します。

---

## English

This document is a checklist for organizing RepoSeiri up to the point just before publication while it remains private. It does not perform publication, change repository visibility, make legal judgments, or guarantee security outcomes.

RepoSeiri is a personal-use Rust coding practice repository. If it is made public later, it should first be clear that this is a personal-use coding practice repository, what CLI / Codex plugin it implements, and what it does not guarantee.

### Desired Pre-Publication State

| Item | Check |
| --- | --- |
| README | A first-time reader can tell that this is a personal-use Rust coding practice repo and a repository organization CLI / Codex plugin prototype. |
| Running it | The first route includes `cargo test --workspace` and `seiri-cli codex`. |
| License | Root `LICENSE`, `LICENSE-MIT`, and `LICENSE-APACHE` exist and do not conflict with the workspace license. |
| Security | There is a route that keeps unfixed vulnerability details out of public issues. |
| Support | The support route does not promise a fixed SLA or guaranteed resolution. |
| Contribution | The contribution route does not guarantee external contribution acceptance, review, or merge. |
| Ownership | `.github/CODEOWNERS` shows the ownership boundary for a personal repository. |
| Automation | CI and Dependabot exist, but they are not described as quality guarantees. |
| Fixtures | `fixtures/` is clearly test input, not real policy or real user data. |
| Claims | There are no claims that guarantee popularity, trust, safety, quality, legal fitness, or publication readiness. |

### Pre-Publication Checks

```powershell
gh repo view ViszCham/RepoSeiri --json visibility,isPrivate
rg -n -i "(token|secret|password|api[_-]?key|private[_-]?key|credential|github_pat_|ghp_|BEGIN .*PRIVATE KEY)" --glob "!target/**" --glob "!Cargo.lock"
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run --quiet -p seiri-cli -- audit --path . --profile library --format markdown
cargo run --quiet -p seiri-cli -- codex --path . --profile library --format markdown
git diff --check
```

If `rg` reports a real secret candidate instead of documentation text, handle secret rotation and history response as separate work before any publication decision.

### Not Promised Yet

- Publication to crates.io or a plugin marketplace.
- A support SLA for external users.
- Completeness of a security program.
- Quality guarantees from RepoSeiri scores.
- Automatic GitHub visibility changes.

### Final Check Before Changing Visibility

1. Check that the GitHub repository description and topics clearly present the repository as personal-use coding practice work.
2. Check that the Japanese first half and English second half of the README contain the same content, decisions, and cautions.
3. Check that `SECURITY.md`, `SUPPORT.md`, and `CONTRIBUTING.md` do not over-promise.
4. Keep RepoSeiri guarded / manual decisions as items for human judgment.
5. Treat visibility change as a separate explicit task.
