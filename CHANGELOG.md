# Changelog

## 日本語

RepoSeiri の利用者向け変更履歴です。内部実装の全履歴ではなく、CLI、schema、plugin、運用境界に影響する変更を記録します。

この changelog は品質、安全性、互換性、保守期間を保証しません。release は maintainer の手動判断です。

### Unreleased

- workspace/plugin sourceを1.0.0へ更新し、analysis、patch plan、Codexをv2-only wireへ移行しました。migration noteと`seiri.error.v1` typed exitを追加しました。
- standalone launcherを`REPOSEIRI_BIN`、bundle-local binary、`PATH`の明示順へ変更し、Windows/Linux bundleとSHA-256 runtime manifestを追加しました。
- role予約付きDocumentIndex、document scope class、logical bilingual route、degraded/unknown co-occurrence、10-profile purpose affinityを追加しました。
- hostile-input corpus、cargo-fuzz target、private overlay v2 metadata、source-boundな`seiri.completion.v3` xtask gateを追加しました。
- RCBP-v1のGit/release/plugin/restart/visibility権限分離は維持します。
- RustSec advisoryに対応して`gix` ecosystemと`time`を更新し、MSRVをRust 1.88へ変更しました。CIとrelease checkに`cargo audit`を追加しました。
- observed claimをevidence-backedな肯定文から表示し、claim-local boundaryをroute、state、strength、meaningとの関連範囲へ限定しました。v2 wireとCodex query集合は維持します。
- Roadmap v10でprimary coverage、bounded source session、framed SHA-256 identity、closed semantic revision、executable pattern extension、visible-prose Markdown event、文書proposition conflictを追加しました。
- READMEをsource checkout、fixture snapshot、主要query、実装境界が先に読める製品入口へ更新し、plugin skill、migration、release metadataと同期しました。
- private calibrationの比較をowner-supplied opaque revisionへ限定し、local supportをsample countと明示interval付きで表示するよう変更しました。
- completion subprocessにtimeout、bounded output capture、kill/reap、typed failureを追加しました。CI action/toolchainを固定し、fuzz smokeとsource-boundなbinary/schema/command host receiptを追加しました。
- route、wording、consistency、profile、plannerのpublic synthetic train/holdout reportを追加しました。completion claimをimplemented、locally verified、host verified、calibrated、manual policyへ分離し、低Nは`insufficient_sample`、環境制約は`implemented_with_blocked_evidence`として保持します。

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

- Updated workspace/plugin source to 1.0.0 and migrated analysis, patch-plan, and Codex output to v2-only wires. Added the migration note and `seiri.error.v1` typed exits.
- Replaced the launcher with explicit `REPOSEIRI_BIN`, bundle-local binary, then `PATH` resolution. Added Windows/Linux bundles and SHA-256 runtime manifests.
- Added role-reserved DocumentIndex selection, document scope classes, logical bilingual routes, degraded/unknown co-occurrence, and ten-profile purpose affinity.
- Added a hostile-input corpus, cargo-fuzz targets, private-overlay v2 metadata, and a source-bound `seiri.completion.v3` xtask gate.
- Preserved the RCBP-v1 separation for Git, release, plugin, restart, and visibility authority.
- Updated the `gix` ecosystem and `time` for RustSec advisories, raised the MSRV to Rust 1.88, and added `cargo audit` to CI and release checks.
- Rendered observed claims from evidence-backed positive statements and limited claim-local boundaries to route, state, strength, and meaning relevance. The v2 wires and Codex query set remain unchanged.
- Roadmap v10 added primary coverage, bounded source sessions, framed SHA-256 identities, closed semantic revisions, executable pattern extensions, visible-prose Markdown events, and document-proposition conflicts.
- Updated the README into a product entry point that leads with source checkout, a fixture snapshot, main queries, and implementation boundaries, synchronized with the plugin skill, migration guide, and release metadata.
- Limited private-calibration comparison to owner-supplied opaque revisions and changed local support output to include sample counts and explicitly named intervals.
- Added timeouts, bounded output capture, kill/reap, and typed failures to completion subprocesses. Pinned CI actions and toolchains, then added fuzz smoke and source-bound binary/schema/command host receipts.
- Added a public synthetic train/holdout report for routes, wording, consistency, profiles, and planning. Completion now separates implemented, locally verified, host verified, calibrated, and manual-policy claims; low-N remains `insufficient_sample`, and environment constraints remain `implemented_with_blocked_evidence`.

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
