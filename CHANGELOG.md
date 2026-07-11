# Changelog

## 日本語

RepoSeiri の利用者向け変更履歴です。内部実装の全履歴ではなく、CLI、schema、plugin、運用境界に影響する変更を記録します。

この changelog は品質、安全性、互換性、保守期間を保証しません。release は maintainer の手動判断です。

### Unreleased

- 現在、未記録の利用者向け変更はありません。

### 0.2.0 - 2026-07-11

#### Breaking

- analysis、patch plan、Codex query をそれぞれ一つの canonical schema に統合しました。
- Codex CLI は `codex --query <kind>` に統一し、schema selector、view selector、暗黙 fallback を削除しました。
- planner は `seiri plan` と `plan_patches` の一経路に統一しました。
- 旧 field alias、旧 enum value、欠落 resource trace の暗黙補完を削除し、該当 wire input を strict にしました。
- workspace と plugin の version を 0.2.0 へ更新しました。

#### Added

- scanner event から直接構築する typed `EvidenceKernel` と deterministic document/evidence ID を追加しました。
- non-serializable `RepositoryAnalysis` と explicit borrowed audit wire を追加しました。
- typed route assessment、content slot、scope、freshness、structured GitHub semantics、claim provenance を canonical report に統合しました。
- source text と private calibration value を除外する portable audit delta を追加しました。
- existing-target-only、dry-run、stale-bound の patch planner を追加しました。
- `summary`、`routes`、`evidence`、`documents`、`governance`、`patches`、`linter`、`actions`、`remote`、`pr-body` の10 Codex query を追加しました。
- calibration/evidence schema validation と strict calibration resource trace を追加しました。
- Roadmap v5 と behavior-named regression suite を追加しました。

#### Changed

- nested policy file を root policy として数えない path predicate を pattern detection に追加しました。
- route delta は README routing、root structure、inherited evidence、local target の typed signal loss/gainを比較します。
- README、docs topology、self-audit、publication checklist、CI artifact、plugin skill を canonical command に同期しました。
- report は旧 DTO を復元せず、scope、freshness、GitHub-local semantics、content claim を canonical data から表示します。

#### Removed

- 重複 evidence ledger、versioned route-content representation、複数 planner generation、複数 Codex schema/view を削除しました。
- 過去の実装 roadmap を削除し、Roadmap v5 を現行の正にしました。
- 世代番号と block 番号を責務名にした test file 名を削除しました。

#### Security And Privacy

- private calibration は明示的 local provider のまま保持し、public JSON、Markdown、Debug、Codex query へ raw value や local path を出しません。
- standard audit は network を開始せず、planner は file を書きません。
- GitHub の branch、commit、push、PR、merge は Rust core から分離した明示操作です。

### 0.1.0 - 2026-07-11

- bounded filesystem/Markdown scanning、repository routes、pattern/profile analysis、calibration、dry-run planning、Codex adapter の初期 prototype を公開しました。
- README、docs、license、security、support、contribution、CI、release、hygiene の repository routes を整備しました。

---

## English

This is the user-facing RepoSeiri change history. It records changes that affect the CLI, schemas, plugin, and operating boundaries rather than every internal implementation detail.

This changelog does not guarantee quality, safety, compatibility duration, or maintenance duration. Releases remain manual maintainer decisions.

### Unreleased

- There are currently no unrecorded user-facing changes.

### 0.2.0 - 2026-07-11

#### Breaking

- Consolidated analysis, patch plans, and Codex queries into one canonical schema each.
- Consolidated the Codex CLI on `codex --query <kind>` and removed schema selectors, view selectors, and implicit fallback.
- Consolidated planning on one `seiri plan` command and one `plan_patches` function.
- Removed old field aliases, old enum values, and implicit repair of missing resource traces, making the relevant wire inputs strict.
- Updated workspace and plugin versions to 0.2.0.

#### Added

- Added a typed `EvidenceKernel` built directly from scanner events with deterministic document and evidence IDs.
- Added a non-serializable `RepositoryAnalysis` owner and an explicit borrowed audit wire.
- Integrated typed route assessments, content slots, scope, freshness, structured GitHub semantics, and claim provenance into the canonical report.
- Added portable audit delta that excludes source text and private calibration values.
- Added an existing-target-only, dry-run, stale-bound patch planner.
- Added ten Codex queries: `summary`, `routes`, `evidence`, `documents`, `governance`, `patches`, `linter`, `actions`, `remote`, and `pr-body`.
- Added calibration/evidence schema validation and required calibration resource traces.
- Added Roadmap v5 and a behavior-named regression suite.

#### Changed

- Added path predicates so nested policy files do not count as root policy.
- Route delta now compares typed losses and gains in README routing, root structure, inherited evidence, and local targets.
- Synchronized README, docs topology, self-audit, publication checks, CI artifacts, and the plugin skill with canonical commands.
- Reports display scope, freshness, GitHub-local semantics, and content claims from canonical data without restoring removed DTOs.

#### Removed

- Removed the duplicate evidence ledger, versioned route-content representation, multiple planner generations, and multiple Codex schemas/views.
- Removed prior implementation roadmaps and made Roadmap v5 authoritative.
- Removed generation- and block-number-based test filenames.

#### Security And Privacy

- Private calibration remains behind an explicit local provider. Public JSON, Markdown, Debug, and Codex queries do not expose raw values or local private paths.
- Standard audit does not start network access, and the planner does not write files.
- GitHub branch, commit, push, PR, and merge operations remain explicit operations outside the Rust core.

### 0.1.0 - 2026-07-11

- Published the initial prototype for bounded filesystem/Markdown scanning, repository routes, pattern/profile analysis, calibration, dry-run planning, and the Codex adapter.
- Added repository routes for README, docs, license, security, support, contributions, CI, release, and hygiene.
