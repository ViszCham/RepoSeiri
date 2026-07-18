# Release Process

## 日本語

RepoSeiri の release docs は、利用者が更新リスクを判断するための手順と境界を示します。正の変更履歴は [CHANGELOG.md](../CHANGELOG.md) です。この文書は release 作業の補助であり、release の自動実行、package publication、GitHub release 作成を行いません。

### Authority

- release 判断は maintainer の手動判断です。
- Codex、RepoSeiri audit、CI は review aid です。release 可否の最終判断を自動化しません。
- security disclosure、legal 判断、ownership 判断は automation ではなく maintainer が扱います。

### Versioning

- tag は `vMAJOR.MINOR.PATCH` 形式を使います。
- `1.0.0` source contractは、public CLI behavior、v2 report schema、standalone plugin adapter、compatibility policyを固定します。
- 現行wireは`seiri.analysis.v2`、`seiri.patch-plan.v2`、`seiri.codex.v2`です。v1 compatibility shimはありません。
- 互換性に影響する変更は、release notes に migration note を置きます。

### Pre-release checks

release 前に次を確認します。

```powershell
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo +1.88.0 check --workspace --all-targets --locked
cargo audit
cargo check --manifest-path fuzz/Cargo.toml --bins --locked
cargo run --quiet -p seiri-cli -- audit --path . --profile library --format markdown
cargo run --quiet -p seiri-cli -- codex --path . --profile library --query summary --format markdown
cargo run --quiet -p seiri-cli -- codex --path . --profile library --query evidence --format json
cargo run --locked --quiet -p xtask -- calibration-holdout --format json
cargo run --locked --quiet -p xtask -- completion --format json
git diff --check
```

fuzz smokeはCIに固定した`nightly-2026-07-01`と`cargo-fuzz 0.13.2`でMarkdown、bounded reader、executable pack、schema decoder、deltaを各64 run実行します。ローカルにこのtoolchainまたはcargo-fuzzがなければcompletionは`environment_blocked`を保持し、他のrequired local checkがpassでもstateを`implemented_with_blocked_evidence`にします。

`ready_for_git`は同じsourceに対するrequired local checkだけの状態です。Windows/Linux双方のsource-bound bundle receiptがなければ`host_verified`はunsatisfiedのままで、CIの最終jobが`--host-evidence target/host-evidence`で再評価します。tracked synthetic holdoutは各task 4 caseで最低20 caseを満たさないため、現在の`calibrated` claimはunsatisfiedであり、`evidence_complete`へ昇格しません。

CI の結果、Dependabot の未処理更新、security issue、manual policy decision が残る場合は、release を止めるか release notes に明示します。

### Changelog update

- `CHANGELOG.md` の `Unreleased` を release version の節へ移します。
- version、date、Added、Changed、Fixed、Removed、Security、Migration を必要な範囲で書きます。
- 実装詳細より、利用者の判断に必要な変更を優先します。
- RepoSeiri score や route state を、人気、信頼、安全性、品質の保証として書きません。

### Manual release

- release branch、tag、GitHub Release は maintainer が明示的に作ります。
- release note は `CHANGELOG.md` の該当節を元にします。
- Windows/Linux plugin bundleはCIのbundle matrixが生成し、`reposeiri.runtime-manifest.v3`へtarget、tool version、repository-relative binary path、binary SHA-256、同梱schema SHA-256、source digest、Cargo.lock digest、contract、semantic revision、実行command setを記録します。
- host receiptは実行したcommand setと同じsource bindingを持つ場合だけcompletion evidenceとして受理します。host absolute pathはpublic receiptへ記録しません。
- tag、GitHub Release、package publicationは自動実行しません。

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
- The `1.0.0` source contract freezes public CLI behavior, v2 report schemas, the standalone plugin adapter, and the compatibility policy.
- Active wires are `seiri.analysis.v2`, `seiri.patch-plan.v2`, and `seiri.codex.v2`. There is no v1 compatibility shim.
- Changes that affect compatibility need migration notes in the release notes.

### Pre-release checks

Before release, check the following.

```powershell
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo +1.88.0 check --workspace --all-targets --locked
cargo audit
cargo check --manifest-path fuzz/Cargo.toml --bins --locked
cargo run --quiet -p seiri-cli -- audit --path . --profile library --format markdown
cargo run --quiet -p seiri-cli -- codex --path . --profile library --query summary --format markdown
cargo run --quiet -p seiri-cli -- codex --path . --profile library --query evidence --format json
cargo run --locked --quiet -p xtask -- calibration-holdout --format json
cargo run --locked --quiet -p xtask -- completion --format json
git diff --check
```

Fuzz smoke uses the CI-pinned `nightly-2026-07-01` and `cargo-fuzz 0.13.2` to run 64 cases each for Markdown, the bounded reader, executable packs, schema decoding, and delta. If the local toolchain or cargo-fuzz is missing, completion records `environment_blocked` and uses `implemented_with_blocked_evidence` even when the other required local checks pass.

`ready_for_git` describes only required local checks bound to the same source. Without both source-bound Windows and Linux bundle receipts, `host_verified` remains unsatisfied; the final CI job reevaluates with `--host-evidence target/host-evidence`. The tracked synthetic holdout currently has four cases per task, below the minimum of 20, so `calibrated` remains unsatisfied and `evidence_complete` is not granted.

If CI results, unresolved Dependabot updates, security issues, or manual policy decisions remain, stop the release or disclose the boundary in the release notes.

### Changelog update

- Move `Unreleased` in `CHANGELOG.md` into a release-version section.
- Include version, date, and the relevant Added, Changed, Fixed, Removed, Security, and Migration entries.
- Prioritize changes users need for decisions over implementation detail.
- Do not describe RepoSeiri scores or route states as guarantees of popularity, trust, safety, or quality.

### Manual release

- Maintainers explicitly create release branches, tags, and GitHub Releases.
- Release notes are based on the matching section of `CHANGELOG.md`.
- The CI bundle matrix generates Windows/Linux plugin bundles and records the target, tool version, repository-relative binary path, binary SHA-256, bundled-schema SHA-256 values, source digest, Cargo.lock digest, contract, semantic revisions, and executed command set in `reposeiri.runtime-manifest.v3`.
- A host receipt is accepted as completion evidence only when it binds its executed command set to the same source. Public receipts do not record host absolute paths.
- Tags, GitHub Releases, and package publication are not run automatically.

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
