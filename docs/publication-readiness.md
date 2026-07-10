# Publication Readiness

## 日本語

この文書は、RepoSeiri を公開リポジトリとして扱うための checklist です。公開操作、repository visibility の変更、法務判断、security outcome の保証を自動化するものではありません。

RepoSeiri は個人使用目的の Rust coding practice repository です。公開状態でも、「個人使用・コーディング練習リポジトリであること」「何をする CLI / Codex plugin か」「何を保証しないか」を明確にします。

### 公開リポジトリとして満たしたい状態

| 項目 | 確認内容 |
| --- | --- |
| README | 初見で、個人使用目的の Rust コーディング練習リポであり、repository organization 用 CLI / Codex plugin prototype だと分かる。 |
| 実行方法 | `cargo test --workspace` と `seiri-cli codex` の最初の route がある。 |
| license | root `LICENSE`、`LICENSE-MIT`、`LICENSE-APACHE` があり、workspace license と矛盾しない。 |
| security | 未修正の脆弱性を public issue に書かない route がある。 |
| support | 固定 SLA や解決保証を約束しない support route がある。 |
| contribution | 外部 contribution の採用、review、merge を保証しない contribution route がある。 |
| lifecycle | maintenance、compatibility、release 判断を確認できる route があり、保守保証としては表現しない。 |
| ownership | `.github/CODEOWNERS` で個人リポとしての所有境界を示す。 |
| automation | CI と Dependabot があり、ただし品質保証として表現しない。 |
| fixtures | `fixtures/` が実 policy や real user data ではなく、テスト入力だと分かる。 |
| claims | 人気、信頼、安全性、品質、法務適合、公開可否を保証する表現がない。 |

### GitHub description / topics

GitHub description は次を使います。

```text
Personal-use Rust coding practice repo for a RepoSeiri CLI/Codex plugin prototype that reviews repository organization routes.
```

GitHub topics は次を使います。

```text
rust, coding-practice, codex-plugin, repository-audit, cli, personal-project
```

### 公開状態チェック

```powershell
gh repo view ViszCham/RepoSeiri --json visibility,isPrivate
rg -n -i "(token|secret|password|api[_-]?key|private[_-]?key|credential|github_pat_|ghp_|BEGIN .*PRIVATE KEY)" --glob "!target/**" --glob "!Cargo.lock"
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo +1.76.0 check --workspace --all-targets --locked
cargo run --quiet -p seiri-cli -- audit --path . --profile library --format markdown
cargo run --quiet -p seiri-cli -- codex --path . --profile library --format markdown
cargo run --quiet -p seiri-cli -- codex --path . --profile library --schema native-v2 --format json
cargo run --quiet -p seiri-cli -- codex --path . --profile library --view linter --format markdown
git diff --check
```

`rg` の結果に、文書内の注意書きではなく実際の secret 候補が出た場合は、公開状態を維持する前に secret rotation と履歴対応を別作業として扱います。

### まだ約束しないこと

- crates.io や plugin marketplace への公開。
- 外部利用者への support SLA。
- security program の完全性。
- RepoSeiri score を品質の根拠として扱うこと。
- GitHub visibility の暗黙変更。

### visibility と公開状態の最終確認

1. GitHub repository description と topics が、個人使用・コーディング練習リポであることを誤解なく示しているか確認します。
2. README の日本語前半と英語後半が同じ内容、同じ判断、同じ注意点になっているか確認します。
3. `SECURITY.md`、`SUPPORT.md`、`CONTRIBUTING.md` が過剰な約束をしていないか確認します。
4. RepoSeiri audit の guarded / manual decision を、人間が判断するべきものとして残しているか確認します。
5. 今後の visibility 変更は、repository owner の明示タスクとして実行します。

---

## English

This document is a checklist for treating RepoSeiri as a public repository. It does not automate publication, change repository visibility by itself, make legal judgments, or guarantee security outcomes.

RepoSeiri is a personal-use Rust coding practice repository. In its public state, it should stay clear about being personal-use coding practice work, what CLI / Codex plugin it implements, and what it does not guarantee.

### Desired Public Repository State

| Item | Check |
| --- | --- |
| README | A first-time reader can tell that this is a personal-use Rust coding practice repo and a repository organization CLI / Codex plugin prototype. |
| Running it | The first route includes `cargo test --workspace` and `seiri-cli codex`. |
| License | Root `LICENSE`, `LICENSE-MIT`, and `LICENSE-APACHE` exist and do not conflict with the workspace license. |
| Security | There is a route that keeps unfixed vulnerability details out of public issues. |
| Support | The support route does not promise a fixed SLA or guaranteed resolution. |
| Contribution | The contribution route does not guarantee external contribution acceptance, review, or merge. |
| Lifecycle | There is a route for maintenance, compatibility, and release decisions, and it is not described as a maintenance guarantee. |
| Ownership | `.github/CODEOWNERS` shows the ownership boundary for a personal repository. |
| Automation | CI and Dependabot exist, but they are not described as quality guarantees. |
| Fixtures | `fixtures/` is clearly test input, not real policy or real user data. |
| Claims | There are no claims that guarantee popularity, trust, safety, quality, legal fitness, or publication readiness. |

### GitHub Description / Topics

Use this GitHub description.

```text
Personal-use Rust coding practice repo for a RepoSeiri CLI/Codex plugin prototype that reviews repository organization routes.
```

Use these GitHub topics.

```text
rust, coding-practice, codex-plugin, repository-audit, cli, personal-project
```

### Public-State Checks

```powershell
gh repo view ViszCham/RepoSeiri --json visibility,isPrivate
rg -n -i "(token|secret|password|api[_-]?key|private[_-]?key|credential|github_pat_|ghp_|BEGIN .*PRIVATE KEY)" --glob "!target/**" --glob "!Cargo.lock"
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo +1.76.0 check --workspace --all-targets --locked
cargo run --quiet -p seiri-cli -- audit --path . --profile library --format markdown
cargo run --quiet -p seiri-cli -- codex --path . --profile library --format markdown
cargo run --quiet -p seiri-cli -- codex --path . --profile library --schema native-v2 --format json
cargo run --quiet -p seiri-cli -- codex --path . --profile library --view linter --format markdown
git diff --check
```

If `rg` reports a real secret candidate instead of documentation text, handle secret rotation and history response as separate work before keeping the repository public.

### Not Promised Yet

- Publication to crates.io or a plugin marketplace.
- A support SLA for external users.
- Completeness of a security program.
- Treating RepoSeiri scores as evidence of quality.
- Implicit GitHub visibility changes.

### Final Visibility And Public-State Check

1. Check that the GitHub repository description and topics clearly present the repository as personal-use coding practice work.
2. Check that the Japanese first half and English second half of the README contain the same content, decisions, and cautions.
3. Check that `SECURITY.md`, `SUPPORT.md`, and `CONTRIBUTING.md` do not over-promise.
4. Keep RepoSeiri guarded / manual decisions as items for human judgment.
5. Treat any future visibility change as an explicit repository-owner task.
