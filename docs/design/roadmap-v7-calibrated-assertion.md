# Roadmap v7: Calibrated Assertion

## 日本語

### 1. 目的

RepoSeiriは、bounded local evidenceが支える最も強い命題を先に表示します。未確認の人気、信頼、安全性、品質、法務適合、保守、production readiness、publication readinessは引き続き保証しません。境界を増やすこと自体を安全性とみなさず、根拠のある肯定文を消す過少主張も損失として扱います。

### 2. 固定契約

- `seiri.analysis.v2`、`seiri.codex.v2`、10個のCodex query、既存CLIは変更しません。
- `ContentClaim`のJSON fieldは`id`、`route`、`state`、`strength`、`evidence_ids`、`allowed_meanings`、`boundaries`のままです。
- observed claimは空でないevidenceとpositive assertionを持ちます。
- claim-local boundaryはroute、state、strength、meaningとの関連があるものだけにします。
- 広域保証の禁止はCodex/reportのglobal boundaryに一度だけ残します。
- private calibrationはobserved claimを弱めず、source path、body、exact valueをpublic surfaceへ出しません。

### 3. Block AE: Baseline / Contract Freeze

- 現行wire shape、query集合、claim数、privacy fixtureを回帰条件として固定します。
- universal per-claim boundaryとpositive wording欠落をbaseline defectとして記録します。
- 完了条件はpublic schema追加なし、private input追加なし、roadmapとdocs topologyの同期です。

### 4. Block AF: Low-Level Claim Calibration IR

- `MeaningMask(u16)`と`ClaimBoundaryMask(u16)`でmeaning/boundary集合を決定的に表します。
- evidence posture、assertion kind、projection candidate、`UnderclaimLoss`をtyped IRにします。
- enumからbitへの変換はmatchで固定し、`unsafe`、transmute、暗黙のdiscriminant依存を使いません。
- 完了条件はobserved/evidence-matched projectionのunderclaim lossが0で、boundary-only projectionが非0になることです。

### 5. Block AG: Boundary Relevance Resolver

- route固有boundaryにstate、strength、meaning由来boundaryをunionします。
- suggested/calibration candidateだけにautomatic policy/weight adoption boundaryを加えます。
- security、license、automationなどのroute固有boundaryを他routeへ一律複製しません。
- 完了条件は同一inputで順序と集合が決定的で、claim JSON shapeが不変であることです。

### 6. Block AH: Positive-First Projection

- `Verified`はrouteとrepository-local targetの観測を明記します。
- `Structured`はstructured evidence、`Routed`はREADME entryを明記します。
- inferred stateはscanned scopeまたはstateを明示し、suggested claimはreview candidateと表現します。
- boundary-only、evidence omission、observed downgrade、boundary-firstを`UnderclaimLoss`原因として区別します。

### 7. Block AI: Renderer / Wording / Codex Adapter

- canonical MarkdownとCodex governanceでassertionをboundaryより先に表示します。
- PR draftはobserved claimの件数とbounded exampleを表示します。
- wording lintのoverclaim規則、`seiri.wording-lint.v1`、Codex query集合は維持します。
- 完了条件はgenerated surfaceにpositive assertionがあり、global boundaryが重複しないことです。

### 8. Block AJ: Private Calibration / Regression / Completion

- synthetic local-only priorでobserved projectionが単調に維持されることを検証します。
- private value、private marker、source detailがJSON/Markdownへ出ないことを検証します。
- fmt、workspace test、clippy、MSRV、audit、secret scan、self-audit、schema/privacy regressionを最終gateにします。
- private分析本文はrepository、fixture、commit、PR、CI logへ移しません。

---

## English

### 1. Goal

RepoSeiri states the strongest proposition supported by bounded local evidence first. It still does not guarantee unverified popularity, trust, security, quality, legal fitness, maintenance, production readiness, or publication readiness. Adding more boundaries is not treated as safety by itself, and erasing an evidence-backed positive statement is tracked as underclaim loss.

### 2. Frozen Contract

- Keep `seiri.analysis.v2`, `seiri.codex.v2`, the ten Codex queries, and the existing CLI unchanged.
- Keep the `ContentClaim` JSON fields as `id`, `route`, `state`, `strength`, `evidence_ids`, `allowed_meanings`, and `boundaries`.
- Every observed claim has nonempty evidence and a positive assertion.
- Claim-local boundaries are limited to boundaries relevant to the route, state, strength, or meaning.
- The global Codex/report boundary blocks broad unsupported claims once.
- Private calibration cannot weaken observed claims or expose source paths, bodies, or exact values on public surfaces.

### 3. Block AE: Baseline / Contract Freeze

- Freeze the current wire shape, query set, claim count behavior, and privacy fixtures as regression conditions.
- Record universal per-claim boundaries and missing positive wording as the baseline defect.
- Completion requires no public schema addition, no private input addition, and synchronized roadmap/docs topology.

### 4. Block AF: Low-Level Claim Calibration IR

- Represent meaning and boundary sets deterministically with `MeaningMask(u16)` and `ClaimBoundaryMask(u16)`.
- Add typed evidence posture, assertion kind, projection candidate, and `UnderclaimLoss` IR.
- Fix enum-to-bit conversion with explicit matches; do not use `unsafe`, transmute, or implicit discriminant dependence.
- Completion requires zero underclaim loss for observed evidence-matched projections and nonzero loss for boundary-only projections.

### 5. Block AG: Boundary Relevance Resolver

- Union route-specific boundaries with boundaries derived from state, strength, and meaning.
- Add automatic policy/weight adoption boundaries only to suggestions and calibration candidates.
- Do not copy security, license, automation, or other route-specific boundaries to every route.
- Completion requires deterministic ordering and sets for identical input while preserving the claim JSON shape.

### 6. Block AH: Positive-First Projection

- `Verified` explicitly states the observed route and repository-local target.
- `Structured` states structured evidence; `Routed` states the README entry.
- Inferred states name the scanned scope or state, while suggested claims are rendered as review candidates.
- Distinguish boundary-only output, evidence omission, observed downgrade, and boundary-first output as `UnderclaimLoss` causes.

### 7. Block AI: Renderer / Wording / Codex Adapter

- Render the assertion before boundaries in canonical Markdown and Codex governance.
- Include the observed-claim count and bounded examples in the PR draft.
- Preserve the wording-lint overclaim rules, `seiri.wording-lint.v1`, and the Codex query set.
- Completion requires positive assertions on generated surfaces and no repeated global boundary.

### 8. Block AJ: Private Calibration / Regression / Completion

- Verify with a synthetic local-only prior that observed projections remain monotonic.
- Verify that private values, markers, and source details do not reach JSON or Markdown.
- Use fmt, workspace tests, clippy, MSRV, audit, secret scan, self-audit, and schema/privacy regressions as the final gate.
- Never move private analysis bodies into the repository, fixtures, commits, PRs, or CI logs.
