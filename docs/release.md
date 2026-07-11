# Release Process

## 日本語

RepoSeiri の release docs は、利用者が更新リスクを判断するための手順と境界を示します。正の変更履歴は [CHANGELOG.md](../CHANGELOG.md) です。この文書は release 作業の補助であり、release の自動実行、package publication、GitHub release 作成を行いません。

### Authority

- release 判断は maintainer の手動判断です。
- Codex、RepoSeiri audit、CI は review aid です。release 可否の最終判断を自動化しません。
- security disclosure、legal 判断、ownership 判断は automation ではなく maintainer が扱います。

### Versioning

- tag は `vMAJOR.MINOR.PATCH` 形式を使います。
- `0.y.z` は、CLI schema、crate API、Codex plugin adapter、release cadence がまだ安定宣言前であることを示します。
- `1.0.0` へ進むには、public CLI behavior、report schema、plugin adapter contract、compatibility policy を明示します。
- 互換性に影響する変更は、release notes に migration note を置きます。

### Pre-release checks

release 前に次を確認します。

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

CI の結果、Dependabot の未処理更新、security issue、manual policy decision が残る場合は、release を止めるか release notes に明示します。

### Changelog update

- `CHANGELOG.md` の `Unreleased` を release version の節へ移します。
- version、date、Added、Changed、Fixed、Removed、Security、Migration を必要な範囲で書きます。
- 実装詳細より、利用者の判断に必要な変更を優先します。
- RepoSeiri score や route state を、人気、信頼、安全性、品質の保証として書きません。

### Manual release

- release branch、tag、GitHub Release は maintainer が明示的に作ります。
- release note は `CHANGELOG.md` の該当節を元にします。
- binary、archive、plugin bundle などの artifact を添付する場合は、生成コマンド、対象 commit、checksum を記録します。
- 現在、release workflow と package publication は未実装です。

### Compatibility boundary

- CLI option、JSON schema、Markdown report、Codex review context、plugin adapter behavior は compatibility review の対象です。
- 互換性を壊す変更は、可能な限り migration note、deprecation note、または major version 境界で扱います。
- fixture や internal crate の変更でも、public output に出る場合は release note 対象です。

### Correction and rollback

- 誤った release note、tag、artifact が出た場合は、削除だけで済ませず correction を記録します。
- 可能なら forward fix を優先し、必要なら patch release を作ります。
- security risk が関わる場合は `SECURITY.md` の経路を優先します。

### Automation boundary

- release route は release を自動化しません。
- CI は verification evidence、Dependabot は dependency update evidence、RepoSeiri は route review evidence として扱います。
- release automation、signed artifact、package publication は、別ブロックで方針決定してから追加します。

---

## English

RepoSeiri release docs define the procedure and boundaries users need for update-risk review. The canonical change history is [CHANGELOG.md](../CHANGELOG.md). This document supports release work; it does not run releases, publish packages, or create GitHub releases.

### Authority

- Release decisions are manual maintainer decisions.
- Codex, RepoSeiri audit, and CI are review aids. They do not automate the final release decision.
- Security disclosure, legal judgment, and ownership judgment remain with maintainers instead of automation.

### Versioning

- Tags use the `vMAJOR.MINOR.PATCH` form.
- `0.y.z` means the CLI schema, crate API, Codex plugin adapter, and release cadence have not yet been declared stable.
- Moving to `1.0.0` requires explicit public CLI behavior, report schema, plugin adapter contract, and compatibility policy.
- Changes that affect compatibility need migration notes in the release notes.

### Pre-release checks

Before release, check the following.

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

If CI results, unresolved Dependabot updates, security issues, or manual policy decisions remain, stop the release or disclose the boundary in the release notes.

### Changelog update

- Move `Unreleased` in `CHANGELOG.md` into a release-version section.
- Include version, date, and the relevant Added, Changed, Fixed, Removed, Security, and Migration entries.
- Prioritize changes users need for decisions over implementation detail.
- Do not describe RepoSeiri scores or route states as guarantees of popularity, trust, safety, or quality.

### Manual release

- Maintainers explicitly create release branches, tags, and GitHub Releases.
- Release notes are based on the matching section of `CHANGELOG.md`.
- If attaching binaries, archives, plugin bundles, or other artifacts, record the generation command, target commit, and checksum.
- Release workflows and package publication are not currently implemented.

### Compatibility boundary

- CLI options, JSON schemas, Markdown reports, Codex review contexts, and plugin adapter behavior are compatibility-review targets.
- Breaking changes should use migration notes, deprecation notes, or major-version boundaries where practical.
- Fixture and internal crate changes are release-note candidates when they affect public output.

### Correction and rollback

- If an incorrect release note, tag, or artifact is published, record a correction instead of relying only on deletion.
- Prefer forward fixes when possible and create a patch release when needed.
- When security risk is involved, prioritize the route in `SECURITY.md`.

### Automation boundary

- The release route does not automate releases.
- CI is verification evidence, Dependabot is dependency-update evidence, and RepoSeiri is route-review evidence.
- Release automation, signed artifacts, and package publication are added only after their policy is decided in a later block.
