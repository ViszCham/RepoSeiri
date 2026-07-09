# CONTRIBUTING

## 日本語

RepoSeiri の contribution route は、Rust core、Codex adapter、repository route 文書、fixtures、calibration data を安全に変更するための入口です。この文書は作業基準を定めますが、外部 contribution の採用、review、merge を保証するものではありません。

### 先に確認すること

- 大きな設計変更、public policy、security、license、ownership、release、automation の方針変更は、PR の前に issue で確認してください。
- 未修正のセキュリティ脆弱性は public issue や public PR に書かず、[SECURITY.md](SECURITY.md) の route を使ってください。
- RepoSeiri の出力は review artifact です。人気、信頼、安全性、品質、法務適合の保証として表現しないでください。
- 人間向けの主要文書を変更する場合は、日本語前半と英語後半に同じ内容、同じ判断、同じ注意点を入れてください。

### 作業単位

- Rust core の挙動変更、docs route の追加、fixture 追加、calibration 更新を混ぜすぎないでください。
- 1つの PR では、目的、対象 route、検証方法が説明できる範囲に絞ってください。
- 既存のユーザー変更や未コミット変更を巻き戻さないでください。
- 自動生成や大規模置換を行う場合は、差分の意図と検証方法をPRに書いてください。

### ローカル検証

変更後は、該当範囲に応じて次を実行してください。

```powershell
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run --quiet -p seiri-cli -- audit --path . --profile library --format markdown
cargo run --quiet -p seiri-cli -- codex --path . --profile library --format markdown
git diff --check
```

docs だけの変更でも、README route、support、contributing、issue forms、PR template の変更では `seiri audit` を実行し、route state が意図どおり変わったか確認してください。

### PR に含める情報

- 変更の目的と対象 route。
- 実装したファイルと、意図的に触っていない範囲。
- 実行した検証 command と結果。
- RepoSeiri audit で変わった route state、profile score、guarded/manual decision。
- 既知の未解決点、maintainer decision が必要な点。

### コーディング基準

- Rust では既存 crate 境界と型を優先し、policy 推測を plugin 側に重ねないでください。
- 低レイヤで安定して読める file/path/evidence は Rust core 側に置き、Codex plugin は adapter に留めてください。
- safe patch は dry-run と gate を優先し、manual decision を自動適用しないでください。
- fixture は実ポリシーの代替ではありません。fixture で通ることを repository policy の証明として扱わないでください。

---

## English

The RepoSeiri contribution route is the entry point for safely changing the Rust core, Codex adapter, repository route documents, fixtures, and calibration data. This document sets working standards, but it does not guarantee acceptance, review, or merge of external contributions.

### Check First

- For large design changes or changes to public policy, security, license, ownership, release, or automation decisions, open an issue before a PR.
- Do not describe an unfixed security vulnerability in a public issue or public PR; use the route in [SECURITY.md](SECURITY.md).
- RepoSeiri output is a review artifact. Do not present it as a guarantee of popularity, trust, safety, quality, or legal fitness.
- When changing major human-facing documents, keep Japanese in the first half and English in the second half with the same content, decisions, and cautions.

### Change Scope

- Avoid mixing Rust core behavior changes, docs route additions, fixture additions, and calibration updates in one broad change.
- Keep each PR narrow enough to explain the goal, affected route, and verification method.
- Do not revert existing user changes or unrelated uncommitted work.
- For generated output or large mechanical replacements, explain the diff intent and verification method in the PR.

### Local Verification

After a change, run the checks that match the scope.

```powershell
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run --quiet -p seiri-cli -- audit --path . --profile library --format markdown
cargo run --quiet -p seiri-cli -- codex --path . --profile library --format markdown
git diff --check
```

Even for docs-only changes, run `seiri audit` when the change touches README routing, support, contributing, issue forms, or the PR template, and confirm that route states changed as intended.

### PR Content

- The purpose of the change and the affected route.
- Files implemented and intentionally untouched areas.
- Verification commands and results.
- Route state, profile score, and guarded/manual decision changes from RepoSeiri audit.
- Known unresolved points and maintainer decisions still required.

### Coding Standards

- In Rust, prefer existing crate boundaries and types, and do not add policy inference to the plugin side.
- Keep stable low-level file/path/evidence reading in the Rust core; keep the Codex plugin as an adapter.
- Prefer dry-run safe patches and gates, and do not auto-apply manual decisions.
- Fixtures are not substitutes for real policy. Passing fixtures is not proof of repository policy.
