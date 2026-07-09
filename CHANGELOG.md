# Changelog

## 日本語

RepoSeiri の利用者向け変更履歴です。更新判断、移行確認、互換性確認の入口として使います。詳細なリリース手順と互換性境界は [Release Process](docs/release.md) に置きます。

この changelog は、人気、信頼、安全性、品質、法務適合の保証ではありません。RepoSeiri の状態を読むための release route です。

### Unreleased

#### Added

- Repository release route の root entry として `CHANGELOG.md` を追加しました。
- `docs/release.md` を追加し、versioning、pre-release checks、release notes、manual release、compatibility boundary を分けました。
- `docs/README.md` と `docs/design/README.md` を追加し、docs topology と design docs の subindex を分けました。
- `.gitignore`、`.gitattributes`、`docs/hygiene.md`、`docs/self-audit.md` を追加し、repository hygiene と self-audit loop の route を分けました。
- R0 から R4 までに追加した repository health route を、README から参照できる形に整理しました。

#### Changed

- README は release 詳細を抱え込まず、`CHANGELOG.md` への入口を持つ route hub として維持します。
- README の docs route は docs topology に向け、詳細設計への導線は docs 側へ逃がします。
- README の hygiene route は `docs/hygiene.md` に向け、self-audit の詳細手順は `docs/self-audit.md` に逃がします。
- リリース作業は、CI、Dependabot、RepoSeiri audit の結果を確認してから行う手動判断として扱います。

#### Security

- Security に関わる変更は、`SECURITY.md` の報告経路と合わせて記録します。
- 脆弱性修正、依存関係更新、公開タイミングが絡む変更は、release notes だけで完結させません。

### Release note policy

- `Unreleased` に未公開の変更を積み、release 時に version と日付を付けた節へ移します。
- 利用者に影響する変更、互換性に関わる変更、移行が必要な変更を優先して書きます。
- 内部実装だけの変更でも、CLI 出力、Codex adapter、report schema、plugin behavior に影響する場合は記録します。
- 公開済み release の誤りを修正する場合は、該当節に correction を追記し、必要なら新しい patch release に逃がします。

---

## English

This is the user-facing change history for RepoSeiri. Use it as the entry point for update decisions, migration review, and compatibility review. Detailed release procedure and compatibility boundaries live in [Release Process](docs/release.md).

This changelog does not guarantee popularity, trust, safety, quality, or legal fitness. It is the release route for reading the state of RepoSeiri.

### Unreleased

#### Added

- Added `CHANGELOG.md` as the root entry for the repository release route.
- Added `docs/release.md` to separate versioning, pre-release checks, release notes, manual release, and compatibility boundaries.
- Added `docs/README.md` and `docs/design/README.md` to separate docs topology from the design docs subindex.
- Added `.gitignore`, `.gitattributes`, `docs/hygiene.md`, and `docs/self-audit.md` to separate repository hygiene from the self-audit loop.
- Organized the repository health routes added from R0 through R4 so the README can route to them.

#### Changed

- The README stays a route hub with an entry to `CHANGELOG.md` instead of carrying release details.
- The README docs route points to docs topology, while detailed design routing moves into docs.
- The README hygiene route points to `docs/hygiene.md`, while detailed self-audit procedure moves into `docs/self-audit.md`.
- Release work is treated as a manual maintainer decision after checking CI, Dependabot, and RepoSeiri audit output.

#### Security

- Security-related changes are recorded together with the reporting route in `SECURITY.md`.
- Vulnerability fixes, dependency updates, and disclosure timing are not handled by release notes alone.

### Release note policy

- Collect unreleased changes under `Unreleased`, then move them into a versioned and dated section during release.
- Prioritize user-facing changes, compatibility changes, and migration-relevant changes.
- Record internal implementation changes when they affect CLI output, Codex adapters, report schemas, or plugin behavior.
- When correcting a published release, add a correction to the relevant section and move risk into a new patch release when needed.
