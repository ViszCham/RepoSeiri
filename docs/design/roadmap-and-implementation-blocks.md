# Roadmap And Implementation Blocks

## 日本語

### 1. 固定する改定方針

RepoSeiri のロードマップは、Pattern Registry first、Profile branching second、Safe repair later に固定します。

- 最初から「この repo type ならこの正解」と決め打ちしません。
- まず全 repo type に共通する observable evidence を低レイヤで読み取ります。
- 目的別の傾向は、Pattern Registry と Profile Overlay に後から追加します。
- 10,000 件 benchmark aggregate と今後の 100,000 件分析データは、証明ではなく、重み付け、優先順位、分岐規則の calibration input として扱います。
- 「人気かつ信頼できるリポになる」という表現は、保証ではなく、信頼されやすい導線を揃える実装目標として扱います。

固定する実装順序:

```text
Core Evidence IR
  -> File / Git / Markdown / YAML scanner
  -> Pattern Registry
  -> Repository Trust Graph
  -> Common Baseline
  -> Profile Overlay
  -> Recommendation Gate
  -> Report
  -> Safe Patch Planner
  -> Codex adapter
```

### 2. 批判を反映した修正

| Earlier risk | Revision |
|---|---|
| repo type を早期に固定しすぎる | Pattern Registry を先に作り、profile は overlay として遅延させる。 |
| scoring を早く作りすぎる | score は初期表示用の view に留め、根拠は Evidence と Finding に残す。 |
| Codex plugin と core logic が密結合になる | Rust core を CLI / library として独立させ、Codex adapter は薄く保つ。 |
| benchmark aggregate を過信する | benchmark は calibration input として扱い、根拠文に限界を明記する。 |
| README 改善に偏りすぎる | README は routing hub として扱い、docs / security / support / release / governance への逃がし方を評価する。 |
| 自動修正が危険な policy file まで触る | Safe / Guarded / Manual の gate を必須にする。 |
| あらゆるパターンを hardcode しようとする | observed pattern を registry 化し、detector、evidence kind、profile rule、fixture を追加単位にする。 |

### 3. 拡張性の中核

| Extension point | Rule |
|---|---|
| EvidenceKind | repo から観測した事実だけを入れる。推測、評価、修正案を混ぜない。 |
| PatternId | benchmark や 100,000 件分析から増える傾向を安定 ID で表す。 |
| Detector | file tree、Git metadata、Markdown、YAML、workflow、GitHub metadata のどれを読むかを明示する。 |
| ProfileRule | library、CLI、infra、docs、tutorial、research、template など目的別の重みを持つ。 |
| Finding | Evidence から導かれる問題、欠落、矛盾、過剰を表す。 |
| Recommendation | Finding への対応候補を Safe / Guarded / Manual に分ける。 |
| ReportRenderer | JSON、Markdown、Codex summary、PR body を同じ IR から生成する。 |
| CalibrationRun | benchmark data から rule weight を更新した履歴を残す。 |

新しい傾向を追加する条件:

1. `PatternId` を追加する。
2. 対応する `EvidenceKind` または既存 evidence との対応を明記する。
3. detector の入力境界を決める。
4. positive fixture と negative fixture を置く。
5. profile への影響を rule として書く。
6. report 表示を追加する。
7. calibration data に出典と日付を残す。

### 4. 詳細ロードマップ

| Phase | Scope | Output | Exit condition |
|---|---|---|---|
| P0 | Architecture freeze | workspace 構成、crate 境界、IR 命名 | docs と crate plan が一致する。 |
| P1 | Core Evidence IR | `RepoSnapshot`, `Evidence`, `Finding`, `Recommendation` | serialization と unit test が通る。 |
| P2 | File / Git scanner | file tree、重要 file、repo hygiene signal | fixture repo を再現可能に走査できる。 |
| P3 | Markdown route scanner | README link、heading、badge、route candidate | README が routing hub か判断できる。 |
| P4 | YAML / workflow scanner | GitHub Actions、CodeQL、release workflow、dependency bot | automation evidence を抽出できる。 |
| P5 | Pattern Registry | common pattern、missing pattern、contradiction pattern | detector を追加単位として登録できる。 |
| P6 | Repository Trust Graph | node、edge、route、broken route、missing route | graph から finding を作れる。 |
| P7 | Common Baseline | identity、license、docs、security、support、release、CI | 全 repo type 共通の report が出る。 |
| P8 | Profile Overlay | library、CLI、infra、docs、tutorial など | profile ごとに recommendation order が変わる。 |
| P9 | Recommendation Gate | Safe / Guarded / Manual 分類 | policy file や security file を無確認で直さない。 |
| P10 | Report / CLI | `seiri audit`, JSON, Markdown report | 人間が review できる report が出る。 |
| P11 | Safe Patch Planner | safe routing patch、link repair、doc route creation | patch plan が dry-run と diff で確認できる。 |
| P12 | Codex adapter | Codex plugin action、PR draft context | core logic なしで adapter を差し替えられる。 |
| P13 | 100,000 data ingest | benchmark schema、pattern stats、calibration run | 新規傾向を registry に流し込める。 |
| P14 | Org-scale aggregation | multi-repo scan、team dashboard data | 組織内 repo の共通欠落を集計できる。 |

### 5. 一括実装ブロック

| Block | Include | Exclude | Exit condition |
|---|---|---|---|
| Block A: Foundation MVP Batch | workspace、core IR、file scanner、Markdown route scanner、JSON / Markdown report、`seiri audit` | patch generation、GitHub API、Codex adapter、100,000 件 calibration | local fixture に対して audit report が安定して出る。 |
| Block B: Pattern And Baseline | Pattern Registry、Common Baseline、finding generation、baseline report | profile scoring、auto fix、remote metadata | 共通傾向だけで actionable finding が出る。 |
| Block C: Profile Branching | profile rule、recommendation order、score view | safe patch、Codex PR | repo 目的ごとに優先順位が変わる。 |
| Block D: Safe Planning | Safe / Guarded / Manual gate、dry-run patch plan | GitHub write、policy 自動確定 | 安全な routing patch だけ生成できる。 |
| Block E: Data Calibration | benchmark schema、100,000 件 ingest、pattern stats、weight suggestion | automatic truth claim、未検証 rule の本採用 | data 由来の候補 rule をレビューできる。 |
| Block F: Codex Integration | Codex adapter、PR body、review context、user-facing actions | core logic の再実装 | Codex が Rust core の結果を使って PR 草案を作れる。 |

### 6. 最初の一括実装ブロック

最初にまとめて実装してよい範囲は Block A: Foundation MVP Batch です。これは後続のすべての block が依存する土台であり、外部 API や自動修正を含まないため、blast radius を抑えながら一括実装できます。

Block A に含めるもの:

- Cargo workspace。
- `crates/seiri-core`: `RepoSnapshot`, `Evidence`, `EvidenceKind`, `RouteKind`, `Finding`, `Recommendation`, `Severity`, `GateKind`。
- `crates/seiri-fs`: repo root detection、file inventory、important file detection、ignore policy。
- `crates/seiri-markdown`: README heading、link、badge、route candidate extraction。
- `crates/seiri-report`: JSON report と Markdown report。
- `crates/seiri-cli`: `seiri audit --path <repo> --format json|markdown`。
- `fixtures/`: minimal repo、README route repo、missing README repo、docs routed repo。
- `tests/`: scanner、Markdown extraction、report snapshot。

Block A から外すもの:

- GitHub API 認証。
- Codex plugin manifest と app action。
- PR 作成。
- patch generation。
- `SECURITY.md`、`CODEOWNERS`、issue template、workflow の自動生成。
- profile scoring。
- 100,000 件分析データ ingest。
- unsafe code。

Block A の acceptance criteria:

- `cargo fmt --all --check` が通る。
- `cargo test` が通る。
- `cargo clippy --all-targets -- -D warnings` が通る。
- `seiri audit --path <fixture> --format json` が stable schema を返す。
- `seiri audit --path <fixture> --format markdown` が human-readable report を返す。
- README route がある repo とない repo を区別できる。
- report は finding の根拠 evidence を参照できる。

### 7. 100,000 件データへの準備

100,000 件データを受け取る前に、次の schema を先に確保します。

| Schema | Purpose |
|---|---|
| `BenchmarkDataset` | dataset 名、取得日、抽出条件、制限事項。 |
| `BenchmarkRepoRecord` | repo identity、stars、age、language、topic、activity、metadata source。 |
| `ObservedPattern` | 観測された構造、pattern id、evidence kind、出現位置。 |
| `PatternStats` | frequency、co-occurrence、repo type correlation、confidence note。 |
| `ProfileRule` | repo 目的ごとの weight、required / optional / harmful の区別。 |
| `CalibrationRun` | rule weight を更新した実行履歴。 |
| `EvidenceSchemaVersion` | 古い scan と新しい scan の互換性境界。 |

100,000 件分析データは、次の順序で取り込みます。

1. raw aggregate を保存する。
2. pattern candidate を抽出する。
3. 既存 `PatternId` に対応づける。
4. 未対応の pattern を pending registry に置く。
5. profile rule の重み候補を作る。
6. 人間が採用、保留、破棄を判断する。
7. 採用した rule だけ runtime registry に入れる。

### 8. 実装順序

次の実作業は、この順で進めます。

1. Block A を一括実装する。
2. Block A の schema と report snapshot を固定する。
3. Block B で Pattern Registry と Common Baseline を足す。
4. Block C で目的別 profile を足す。
5. Block D で safe patch planning を足す。
6. 100,000 件データ受領後、Block E で calibration pipeline を足す。
7. 最後に Block F で Codex adapter と PR workflow を足す。

### 9. Roadmap v2 固定

Block A-F の実装後は、100,000 件 / 1,000,000 件級の分析データを calibration input として扱い、次の Roadmap v2 を固定する。

この roadmap は、人気リポジトリの形をコピーするものではない。RepoSeiri は、目的別の trust route、route state、co-occurrence、Safe / Guarded / Manual gate を観測 evidence から組み立てる。

| Block | Scope | 1,000,000 件級データの使い方 | Exit condition |
|---|---|---|---|
| Block G: Evidence Ledger / RouteState v2 | `EvidenceRecord`, `EvidenceId`, `EvidenceSpan`, `EvidenceConfidence`, `RouteState` | README 98.6%、LICENSE 92.0%、docs route 81.4% を critical baseline の初期重みに使う | route が `absent`, `implicit`, `weak`, `routed`, `structured`, `verified`, `inherited`, `overridden`, `conflicting`, `overloaded`, `stale`, `unsafe_to_invent` で表現される |
| Block H: README Route Analyzer v2 | README を routing hub として解析し、docs / quickstart / support / security / release / governance への到達 map を作る | docs route 欠落 186,000、quickstart 欠落 438,000、release route 欠落 454,000 を優先度に反映する | README route map が weak / conflicting / overloaded / stale を検出できる |
| Block I: Pattern Registry v2 | `IDN`, `DOC`, `QST`, `SUP`, `SEC`, `CTR`, `INT`, `AUT`, `REL`, `OWN`, `GOV`, `HYG`, `LIF` の registry に分割する | SEC-001、INT-001、AUT-001、REL-002、OWN-001 などを高 impact 候補として登録する | registry が JSON と Markdown に render でき、pattern 追加が小さい差分で済む |
| Block J: Profile Branch Confidence | repo type を単一断定せず、複数 profile と confidence を出す | Library 28%、Infra 17%、CLI 11%、Product 9%、Runtime 4.5%、Docs 9.5%、Tutorial 10%、ML 7.5%、Template 3.5% を初期 prior に使う | `Library 0.72 / CLI 0.31` のような複数候補が出る |
| Block K: Missing Route Priority + Co-occurrence Engine | missing route の優先度と co-occurrence を評価する | SECURITY 欠落 558,000、SUPPORT 欠落 503,000、Issue forms 欠落 822,000、CODEOWNERS 欠落 605,000 を高 leverage gap として扱う | `README + Security + CI + Dependency bot` などの組み合わせ gap が説明される |
| Block L: Security / Support / Intake / Automation | SECURITY、SUPPORT、issue forms、PR template、dependency bot、security automation を重点実装する | SECURITY 44.2%、Issue templates 41.6%、Dependency bot 34.8%、Security automation 31.2% を「未普及だが高価値」の領域として扱う | route 可視化は Safe、雛形は Guarded、contact / SLA / CODEOWNERS は Manual になる |
| Block M: Safe Patch Planner v2 | safe patch、guarded draft、manual decision を厳密に分ける | Safe / Guarded / Manual の表を patch planner の分類基準にする | `unsafe_to_invent` は必ず blocked item に落ちる |
| Block N: Report / Codex Adapter v2 | score 中心ではなく route 中心の report と Codex context を出す | strong routes、weak routes、missing routes、co-occurrence gaps、safe fixes、guarded drafts、manual decisions を report に出す | Codex に渡るのは safe patch と reviewable guarded draft に限定される |
| Block O: Calibration Ingest v2 | `CalibrationSource`, `PatternStats`, `RouteRequirement`, `ProfileBranch`, `WeightSuggestion`, `ClaimBoundary` を強化する | 1,000,000 件級データを runtime rule ではなく reviewable calibration として保存する | 新しい 100,000 / 1,000,000 件データを差し替えても core が壊れない |
| Block P: Fixture / Regression Suite | profile、route state、gate、co-occurrence の fixture と snapshot test を増やす | 9 profile と route state の代表ケースを作る | confidence、route state、gate 分類が regression test で固定される |

Roadmap v2 の実装順序は `G -> H -> I -> J -> K -> L -> M -> N -> O -> P` とする。特に G/H/I は次期実装の土台であり、ここを飛ばして security や issue forms を増やすと、単純な「ファイルがある / ない」判定に戻る。

### 10. Claim boundary

RepoSeiri は、人気、信頼、セキュリティ、保守性を保証しません。RepoSeiri が出すのは、観測した evidence、そこから導いた finding、目的別の recommendation、そして安全 gate を通した patch plan です。信頼されやすいリポジトリに多い導線を整えることはできますが、外部評価、利用者数、star 数、security outcome を保証するものではありません。

---

## English

### 1. Fixed Revision Direction

The RepoSeiri roadmap is fixed as Pattern Registry first, Profile branching second, and Safe repair later.

- Do not decide too early that one repository type has one correct answer.
- First read observable evidence shared across repository types at a low layer.
- Add purpose-specific tendencies later through the Pattern Registry and Profile Overlay.
- Treat the 10,000-repository benchmark aggregate and the future 100,000-repository analysis data as calibration input for weights, priorities, and branching rules, not as proof.
- Treat the phrase "becoming a popular and trusted repository" as an implementation goal for arranging trust routes, not as a guarantee.

Fixed implementation order:

```text
Core Evidence IR
  -> File / Git / Markdown / YAML scanner
  -> Pattern Registry
  -> Repository Trust Graph
  -> Common Baseline
  -> Profile Overlay
  -> Recommendation Gate
  -> Report
  -> Safe Patch Planner
  -> Codex adapter
```

### 2. Revisions From Critique

| Earlier risk | Revision |
|---|---|
| Fixing repo type too early | Build Pattern Registry first and delay profiles as overlays. |
| Building scoring too early | Keep score as an initial view, while preserving Evidence and Finding as the basis. |
| Tight coupling between Codex plugin and core logic | Keep the Rust core independent as CLI / library, and keep the Codex adapter thin. |
| Over-trusting the benchmark aggregate | Treat the benchmark as calibration input and state its limits in generated rationale. |
| Over-focusing on README improvements | Treat README as a routing hub and evaluate how it routes to docs, security, support, release, and governance. |
| Automatically touching risky policy files | Require the Safe / Guarded / Manual gate. |
| Trying to hardcode every pattern | Store observed patterns in a registry, with detector, evidence kind, profile rule, and fixture as the unit of addition. |

### 3. Extensibility Core

| Extension point | Rule |
|---|---|
| EvidenceKind | Store only facts observed from the repository. Do not mix inference, evaluation, or repair proposals. |
| PatternId | Represent tendencies from benchmarks and the future 100,000-repository analysis with stable IDs. |
| Detector | State whether it reads the file tree, Git metadata, Markdown, YAML, workflow, or GitHub metadata. |
| ProfileRule | Hold purpose-specific weights such as library, CLI, infra, docs, tutorial, research, and template. |
| Finding | Represent a problem, absence, contradiction, or excess derived from Evidence. |
| Recommendation | Separate responses to Findings into Safe / Guarded / Manual actions. |
| ReportRenderer | Generate JSON, Markdown, Codex summary, and PR body from the same IR. |
| CalibrationRun | Record the history of rule-weight updates derived from benchmark data. |

Conditions for adding a new tendency:

1. Add a `PatternId`.
2. State the corresponding `EvidenceKind` or mapping to existing evidence.
3. Define the detector input boundary.
4. Add a positive fixture and a negative fixture.
5. Write the profile impact as a rule.
6. Add report rendering.
7. Record the source and date in calibration data.

### 4. Detailed Roadmap

| Phase | Scope | Output | Exit condition |
|---|---|---|---|
| P0 | Architecture freeze | Workspace structure, crate boundaries, IR naming | Docs and crate plan agree. |
| P1 | Core Evidence IR | `RepoSnapshot`, `Evidence`, `Finding`, `Recommendation` | Serialization and unit tests pass. |
| P2 | File / Git scanner | File tree, important files, repo hygiene signals | Fixture repos can be scanned reproducibly. |
| P3 | Markdown route scanner | README links, headings, badges, route candidates | The tool can judge whether README acts as a routing hub. |
| P4 | YAML / workflow scanner | GitHub Actions, CodeQL, release workflow, dependency bot | Automation evidence can be extracted. |
| P5 | Pattern Registry | Common patterns, missing patterns, contradiction patterns | Detectors can be registered as addition units. |
| P6 | Repository Trust Graph | Nodes, edges, routes, broken routes, missing routes | Findings can be created from the graph. |
| P7 | Common Baseline | Identity, license, docs, security, support, release, CI | Common reports work across repo types. |
| P8 | Profile Overlay | Library, CLI, infra, docs, tutorial, and similar profiles | Recommendation order changes by profile. |
| P9 | Recommendation Gate | Safe / Guarded / Manual classification | Policy and security files are not changed without confirmation. |
| P10 | Report / CLI | `seiri audit`, JSON, Markdown report | Human-reviewable reports are produced. |
| P11 | Safe Patch Planner | Safe routing patches, link repair, doc route creation | Patch plans are inspectable through dry-run and diff. |
| P12 | Codex adapter | Codex plugin action, PR draft context | The adapter can be replaced without core logic changes. |
| P13 | 100,000 data ingest | Benchmark schema, pattern stats, calibration run | New tendencies can flow into the registry. |
| P14 | Org-scale aggregation | Multi-repo scan, team dashboard data | Common gaps across organization repositories can be aggregated. |

### 5. Implementation Blocks

| Block | Include | Exclude | Exit condition |
|---|---|---|---|
| Block A: Foundation MVP Batch | Workspace, core IR, file scanner, Markdown route scanner, JSON / Markdown report, `seiri audit` | Patch generation, GitHub API, Codex adapter, 100,000-repository calibration | Audit reports are stable against local fixtures. |
| Block B: Pattern And Baseline | Pattern Registry, Common Baseline, finding generation, baseline report | Profile scoring, auto fix, remote metadata | Actionable findings are produced from common tendencies only. |
| Block C: Profile Branching | Profile rules, recommendation order, score view | Safe patches, Codex PR | Priority changes by repository purpose. |
| Block D: Safe Planning | Safe / Guarded / Manual gate, dry-run patch plan | GitHub write, automatic policy decisions | Only safe routing patches can be generated. |
| Block E: Data Calibration | Benchmark schema, 100,000-repository ingest, pattern stats, weight suggestions | Automatic truth claims, adoption of unverified rules | Data-derived candidate rules can be reviewed. |
| Block F: Codex Integration | Codex adapter, PR body, review context, user-facing actions | Reimplementation of core logic | Codex can draft PRs from Rust core results. |

### 6. First Batch Implementation Block

The first scope that should be implemented as one batch is Block A: Foundation MVP Batch. It is the base that every later block depends on, and it avoids external APIs and automatic repair, so it can be implemented together while keeping the blast radius controlled.

Block A includes:

- Cargo workspace.
- `crates/seiri-core`: `RepoSnapshot`, `Evidence`, `EvidenceKind`, `RouteKind`, `Finding`, `Recommendation`, `Severity`, `GateKind`.
- `crates/seiri-fs`: repo root detection, file inventory, important file detection, ignore policy.
- `crates/seiri-markdown`: README heading, link, badge, and route candidate extraction.
- `crates/seiri-report`: JSON report and Markdown report.
- `crates/seiri-cli`: `seiri audit --path <repo> --format json|markdown`.
- `fixtures/`: minimal repo, README route repo, missing README repo, docs routed repo.
- `tests/`: scanner, Markdown extraction, report snapshot.

Block A excludes:

- GitHub API authentication.
- Codex plugin manifest and app action.
- PR creation.
- Patch generation.
- Automatic generation of `SECURITY.md`, `CODEOWNERS`, issue templates, or workflows.
- Profile scoring.
- 100,000-repository analysis data ingest.
- unsafe code.

Block A acceptance criteria:

- `cargo fmt --all --check` passes.
- `cargo test` passes.
- `cargo clippy --all-targets -- -D warnings` passes.
- `seiri audit --path <fixture> --format json` returns a stable schema.
- `seiri audit --path <fixture> --format markdown` returns a human-readable report.
- A repository with README routes and a repository without README routes can be distinguished.
- The report can reference the evidence behind each finding.

### 7. Preparation For 100,000-Repository Data

Before receiving the 100,000-repository data, reserve these schemas.

| Schema | Purpose |
|---|---|
| `BenchmarkDataset` | Dataset name, collection date, extraction conditions, limitations. |
| `BenchmarkRepoRecord` | Repo identity, stars, age, language, topics, activity, metadata source. |
| `ObservedPattern` | Observed structure, pattern id, evidence kind, location. |
| `PatternStats` | Frequency, co-occurrence, repo-type correlation, confidence note. |
| `ProfileRule` | Purpose-specific weight and required / optional / harmful classification. |
| `CalibrationRun` | Execution history for rule-weight updates. |
| `EvidenceSchemaVersion` | Compatibility boundary between older and newer scans. |

Ingest the 100,000-repository analysis data in this order:

1. Store the raw aggregate.
2. Extract pattern candidates.
3. Map them to existing `PatternId`s.
4. Put unmapped patterns into the pending registry.
5. Create candidate profile-rule weights.
6. Let a human adopt, defer, or reject them.
7. Add only adopted rules to the runtime registry.

### 8. Implementation Order

Proceed in this order:

1. Implement Block A as one batch.
2. Freeze the Block A schema and report snapshots.
3. Add Pattern Registry and Common Baseline in Block B.
4. Add purpose-specific profiles in Block C.
5. Add safe patch planning in Block D.
6. After receiving the 100,000-repository data, add the calibration pipeline in Block E.
7. Add the Codex adapter and PR workflow last in Block F.

### 9. Roadmap v2 Lock

After Blocks A-F, fix Roadmap v2 around the 100,000 / 1,000,000-repository analysis data as calibration input.

This roadmap does not copy the shape of popular repositories. RepoSeiri builds purpose-specific trust routes, route states, co-occurrence, and Safe / Guarded / Manual gates from observed evidence.

| Block | Scope | Use of 1,000,000-level data | Exit condition |
|---|---|---|---|
| Block G: Evidence Ledger / RouteState v2 | Add `EvidenceRecord`, `EvidenceId`, `EvidenceSpan`, `EvidenceConfidence`, and `RouteState` | Use README 98.6%, LICENSE 92.0%, and docs route 81.4% as initial weights for critical baseline routes | Routes are represented as `absent`, `implicit`, `weak`, `routed`, `structured`, `verified`, `inherited`, `overridden`, `conflicting`, `overloaded`, `stale`, or `unsafe_to_invent` |
| Block H: README Route Analyzer v2 | Analyze README as a routing hub and build reachability maps to docs / quickstart / support / security / release / governance | Reflect docs route gaps 186,000, quickstart gaps 438,000, and release route gaps 454,000 in priority | README route maps detect weak, conflicting, overloaded, and stale routes |
| Block I: Pattern Registry v2 | Split the registry into `IDN`, `DOC`, `QST`, `SUP`, `SEC`, `CTR`, `INT`, `AUT`, `REL`, `OWN`, `GOV`, `HYG`, and `LIF` groups | Register SEC-001, INT-001, AUT-001, REL-002, OWN-001, and similar high-impact candidates | The registry renders as JSON and Markdown, and adding a pattern stays a small diff |
| Block J: Profile Branch Confidence | Emit multiple profiles with confidence instead of asserting one repo type | Use Library 28%, Infra 17%, CLI 11%, Product 9%, Runtime 4.5%, Docs 9.5%, Tutorial 10%, ML 7.5%, and Template 3.5% as initial priors | The report can emit candidates such as `Library 0.72 / CLI 0.31` |
| Block K: Missing Route Priority + Co-occurrence Engine | Evaluate missing-route priority and co-occurrence | Treat SECURITY gaps 558,000, SUPPORT gaps 503,000, Issue forms gaps 822,000, and CODEOWNERS gaps 605,000 as high-leverage gaps | Combination gaps such as `README + Security + CI + Dependency bot` are explained |
| Block L: Security / Support / Intake / Automation | Focus on SECURITY, SUPPORT, issue forms, PR templates, dependency bots, and security automation | Treat SECURITY 44.2%, Issue templates 41.6%, Dependency bot 34.8%, and Security automation 31.2% as lower-adoption but high-value areas | Route visualization is Safe, skeleton drafts are Guarded, and contact / SLA / CODEOWNERS remain Manual |
| Block M: Safe Patch Planner v2 | Strictly separate safe patches, guarded drafts, and manual decisions | Use the Safe / Guarded / Manual table as patch planner classification input | `unsafe_to_invent` always becomes a blocked item |
| Block N: Report / Codex Adapter v2 | Emit route-centered reports and Codex context instead of score-centered output | Show strong routes, weak routes, missing routes, co-occurrence gaps, safe fixes, guarded drafts, and manual decisions | Codex receives only safe patches and reviewable guarded drafts |
| Block O: Calibration Ingest v2 | Strengthen `CalibrationSource`, `PatternStats`, `RouteRequirement`, `ProfileBranch`, `WeightSuggestion`, and `ClaimBoundary` | Store 1,000,000-level data as reviewable calibration, not as runtime rules | New 100,000 / 1,000,000-repository data can be swapped in without breaking the core |
| Block P: Fixture / Regression Suite | Add fixture and snapshot coverage for profiles, route states, gates, and co-occurrence | Create representative cases for the 9 profiles and route states | Confidence, route state, and gate classification are fixed by regression tests |

The Roadmap v2 implementation order is `G -> H -> I -> J -> K -> L -> M -> N -> O -> P`. G/H/I are the next foundation. If security or issue form work is added before them, the system risks falling back to simple file-present / file-missing checks.

### 10. Claim Boundary

RepoSeiri does not guarantee popularity, trust, security, or maintainability. RepoSeiri produces observed evidence, findings derived from that evidence, purpose-specific recommendations, and patch plans passed through safety gates. It can organize routes commonly found in trusted repositories, but it does not guarantee external evaluation, user count, stars, or security outcomes.
