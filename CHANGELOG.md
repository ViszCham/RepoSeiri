# Changelog

## 日本語

RepoSeiri の利用者向け変更履歴です。更新判断、移行確認、互換性確認の入口として使います。詳細なリリース手順と互換性境界は [Release Process](docs/release.md) に置きます。

この changelog は、人気、信頼、安全性、品質、法務適合の保証ではありません。RepoSeiri の状態を読むための release route です。

### Unreleased

#### Added

- Evidence Kernel v3 の Q12-Q19 実装順序と完了条件を低レイヤ設計roadmapへ固定しました。
- Q12 Semantic Firewall として、固定集計値をrepository observationから分離する `AggregateRepositoryEstimate` と互換読み込み検査を追加しました。
- Q13 Compact Evidence Kernelとして、`EvidenceId`、`EvidenceDraft`、`EvidenceFact`、`EvidenceKernel`、typed originを追加しました。
- Q14 RouteAssessment v3として、route presence、README routing、target reachability、conflict、freshnessの直交したtyped assessmentを追加しました。
- Q15 Scanner / Document Eventsとして、typed repository walker、bounded Markdown scanner、canonical `DocumentEvent`、soft diagnosticを追加しました。
- Q16 Pattern / Profile v3として、detector、欠落判断境界、profile、negative fixtureをregistry単位に分離し、profile scoreをrepository evidenceとstatic registry weightだけから計算する型境界を追加しました。
- Q17 Streaming Calibrationとして、全recordやrepository ID setを保持しないbounded JSONL集計経路、typed pattern slot、固定長co-occurrence matrix、deterministic replay digest、structural resource traceを追加しました。
- Q18 Patch Proposal IRとして、base digest、encoding、EOL、byte span、unresolved policy slotに結びつくtyped text editと、apply前のReady/Hold/Reject preflightを追加しました。
- Q19 Renderer / Codex v4として、単一review kernelから生成するv1互換view、native v2、typed query、full linter context、program+argv commandを追加しました。
- Repository release route の root entry として `CHANGELOG.md` を追加しました。
- `docs/release.md` を追加し、versioning、pre-release checks、release notes、manual release、compatibility boundary を分けました。
- `docs/README.md` と `docs/design/README.md` を追加し、docs topology と design docs の subindex を分けました。
- `.gitignore`、`.gitattributes`、`docs/hygiene.md`、`docs/self-audit.md` を追加し、repository hygiene と self-audit loop の route を分けました。
- R0 から R4 までに追加した repository health route を、README から参照できる形に整理しました。

#### Changed

- calibration source のvisibility省略値を`LocalOnly`へ変更し、provenance未確定のJSONL wrapperとinferred sourceをpublic扱いしないようにしました。
- README routeの`Verified`を、存在確認済みのrepository-local targetへ限定しました。external、mail、anchor、unknown targetは`Routed`のまま扱います。
- Missing route priority、co-occurrence、README gapのnative JSON / Markdownは、固定集計値をobservation名ではなくestimate名で出力します。旧field名はdeserialize aliasとしてのみ受け付けます。
- `Verified`のmeaning atomを`RepositoryLocalTargetPresent`へ明確化し、旧`route_target_present`は読み込み互換だけに残しました。
- auditの判断経路をcanonical `EvidenceKernel`へ切り替え、旧`Evidence`と`EvidenceRecord`をkernel由来のcompatibility viewへ移しました。
- Markdownのbyte spanをevidence factへ直接渡し、`EvidenceSource.detail`からlineを再parseする処理を削除しました。
- reportとCodex contextにcanonical evidence fact数を追加しました。
- README route mapとrepository route判断をassessment先行へ変更し、旧`RouteState`をdeterministic compatibility projectionへ移しました。
- filesystem walkをimportant-file分類から分離し、`ReadmeSummary`とevidence生成をcanonical `DocumentScan` event stream由来へ変更しました。
- README は release 詳細を抱え込まず、`CHANGELOG.md` への入口を持つ route hub として維持します。
- README の docs route は docs topology に向け、詳細設計への導線は docs 側へ逃がします。
- README の hygiene route は `docs/hygiene.md` に向け、self-audit の詳細手順は `docs/self-audit.md` に逃がします。
- リリース作業は、CI、Dependabot、RepoSeiri audit の結果を確認してから行う手動判断として扱います。
- README に project identity と lifecycle / maintenance boundary の入口を追加し、Example Output を現行 self-audit の manual decision 数と合わせました。
- README、docs topology、publication checklist、self-audit、CI artifact を Q19 の compatibility v1 / native v2 / query / linter surface と一致させました。
- Rust 1.76 の宣言MSRVを検証できるように`clap`を4.5.53へ固定し、local / CIのall-targets checkを追加しました。

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

- Fixed the Evidence Kernel v3 Q12-Q19 implementation order and completion criteria in the low-level design roadmap.
- Added Q12 Semantic Firewall with `AggregateRepositoryEstimate` and compatibility-read tests so fixed aggregate values stay separate from repository observations.
- Added Q13 Compact Evidence Kernel with `EvidenceId`, `EvidenceDraft`, `EvidenceFact`, `EvidenceKernel`, and typed origins.
- Added Q14 RouteAssessment v3 with orthogonal typed components for route presence, README routing, target reachability, conflicts, and freshness.
- Added Q15 Scanner / Document Events with a typed repository walker, bounded Markdown scanning, canonical `DocumentEvent` values, and soft diagnostics.
- Added Q16 Pattern / Profile v3 by separating detectors, missing-decision boundaries, profiles, and negative fixtures at registry level, with a typed profile-score boundary limited to repository evidence and static registry weights.
- Added Q17 Streaming Calibration with a bounded JSONL aggregation path that retains neither all records nor repository-ID sets, using typed pattern slots, a fixed co-occurrence matrix, a deterministic replay digest, and a structural resource trace.
- Added Q18 Patch Proposal IR with typed text edits bound to a base digest, encoding, EOL convention, byte spans, and unresolved policy slots, plus Ready/Hold/Reject preflight before application.
- Added Q19 Renderer / Codex v4 with a v1 compatibility view, native v2, typed queries, full linter context, and program-plus-argv commands generated from one review kernel.
- Added `CHANGELOG.md` as the root entry for the repository release route.
- Added `docs/release.md` to separate versioning, pre-release checks, release notes, manual release, and compatibility boundaries.
- Added `docs/README.md` and `docs/design/README.md` to separate docs topology from the design docs subindex.
- Added `.gitignore`, `.gitattributes`, `docs/hygiene.md`, and `docs/self-audit.md` to separate repository hygiene from the self-audit loop.
- Organized the repository health routes added from R0 through R4 so the README can route to them.

#### Changed

- Changed omitted calibration source visibility to `LocalOnly`, so JSONL wrappers and inferred sources without established provenance are not treated as public.
- Limited README-route `Verified` to existence-checked repository-local targets. External, mail, anchor, and unknown targets remain `Routed`.
- Native JSON and Markdown for missing-route priority, co-occurrence, and README gaps now emit fixed aggregate values under estimate names rather than observation names. Old field names are accepted only as deserialization aliases.
- Clarified the `Verified` meaning atom as `RepositoryLocalTargetPresent`; legacy `route_target_present` remains read-compatible only.
- Moved audit decisions to the canonical `EvidenceKernel` and made legacy `Evidence` and `EvidenceRecord` kernel-derived compatibility views.
- Passed Markdown byte spans directly into evidence facts and removed line-number reparsing from `EvidenceSource.detail`.
- Added canonical evidence fact counts to report and Codex context output.
- Changed README route-map and repository-route decisions to assessment-first evaluation, with the old `RouteState` emitted as a deterministic compatibility projection.
- Separated filesystem walking from important-file classification and changed `ReadmeSummary` plus evidence generation to derive from the canonical `DocumentScan` event stream.
- The README stays a route hub with an entry to `CHANGELOG.md` instead of carrying release details.
- The README docs route points to docs topology, while detailed design routing moves into docs.
- The README hygiene route points to `docs/hygiene.md`, while detailed self-audit procedure moves into `docs/self-audit.md`.
- Release work is treated as a manual maintainer decision after checking CI, Dependabot, and RepoSeiri audit output.
- Added README entry points for project identity and lifecycle / maintenance boundaries, and aligned the Example Output with the current self-audit manual decision count.
- Aligned the README, docs topology, publication checklist, self-audit loop, and CI artifacts with the Q19 compatibility v1, native v2, query, and linter surfaces.
- Pinned `clap` to 4.5.53 and added local and CI all-target checks so the declared Rust 1.76 MSRV is exercised.

#### Security

- Security-related changes are recorded together with the reporting route in `SECURITY.md`.
- Vulnerability fixes, dependency updates, and disclosure timing are not handled by release notes alone.

### Release note policy

- Collect unreleased changes under `Unreleased`, then move them into a versioned and dated section during release.
- Prioritize user-facing changes, compatibility changes, and migration-relevant changes.
- Record internal implementation changes when they affect CLI output, Codex adapters, report schemas, or plugin behavior.
- When correcting a published release, add a correction to the relevant section and move risk into a new patch release when needed.
