# Repair, Implementation, And Verification

## 日本語

### 1. Core IR

Rust 実装では、文字列レポートを先に作らず、typed IR を先に作ります。

| IR | Main fields | Role |
|---|---|---|
| `RepoSnapshot` | path, remote, default_branch, file_index, git_metadata | 入力 repo の低レイヤ snapshot。 |
| `Evidence` | kind, path, span, source, confidence, inherited | 検出結果の最小単位。 |
| `TrustNode` | node_kind, label, location | README、docs、policy file などの node。 |
| `TrustRoute` | route_kind, from, to, status, strength, blockers | 導線の状態。 |
| `TrustGraph` | nodes, routes, missing_routes, contradictions | repo の信頼構造。 |
| `Finding` | id, severity, route, evidence_refs, profile_scope | 問題または改善余地。 |
| `Recommendation` | action, gate, target_files, rationale, risk | 提案。 |
| `PatchPlan` | edits, created_files, skipped_actions, manual_actions | 実際に生成する差分計画。 |
| `TargetProfile` | repo_type, goal, weights, required_routes | 目的別 overlay。 |
| `SeiriReport` | summary, score_view, findings, recommendations | 人間向け report と JSON 出力の共通元。 |

### 2. Rust module 境界

低レイヤ優先の crate / module 境界:

| Module | Responsibility |
|---|---|
| `seiri-fs` | directory walk、file size、binary sniff、ignore rules。 |
| `seiri-git` | Git object、tree、remote、branch、history metadata。 |
| `seiri-markdown` | heading、link、badge、section classification。 |
| `seiri-yaml` | issue forms、Dependabot、workflow YAML の typed parse。 |
| `seiri-workflow` | GitHub Actions signal、job kind、security scanner detection。 |
| `seiri-core` | IR、graph、scoring、profile overlay。 |
| `seiri-planner` | recommendation gate、patch plan、diff generation。 |
| `seiri-report` | Markdown / JSON report rendering。 |
| `seiri-codex` | Codex plugin boundary、tool command、PR 作成連携。 |

低レイヤ化する理由:

- file traversal、Git tree、Markdown link 解析は大量 repo で効く。
- typed parser と explicit error は false positive の原因を report に残しやすい。
- streaming / low-copy は大規模 repo と monorepo で効く。
- GitHub API、Codex host、PR 作成などの外部境界は、安全性と互換性を優先して高レイヤ API を使う。

### 3. Recommendation Gate

RepoSeiri は、自動修正できるものと、人間が決めるべきものを分けます。

| Gate | Meaning | Examples |
|---|---|---|
| Safe | 既存事実から低リスクに生成できる。 | README の route link 追加、`SEIRI_REPORT.md`、壊れた link の report、docs route の整理案。 |
| Guarded | 草案生成はできるが、project owner の確認が必要。 | SUPPORT.md、SECURITY.md skeleton、issue forms、PR template、Dependabot config。 |
| Manual | 自動生成より人間判断が主。 | LICENSE choice、CODEOWNERS ownership、security SLA、governance、large CI redesign、archive decision。 |

Safe patch の条件:

- 既存ファイルや既存 docs への link を追加するだけ。
- 既存 evidence と矛盾しない。
- policy、owner、license、security response promise を勝手に作らない。
- README の日本語/英語など既存構造を壊さない。

### 4. MVP

MVP は、graph と safety gate が通る最小単位に絞ります。

| Milestone | Scope | Exit condition |
|---|---|---|
| M0 | repo snapshot、file index、Markdown heading/link parse | README の route evidence を JSON 出力できる。 |
| M1 | README Router Analyzer | docs、quickstart、support、contributing、security、license、release route を分類できる。 |
| M2 | Docs Topology + Community Health | `/docs`、org default、repo `.github`、LICENSE boundary を評価できる。 |
| M3 | Security / Support / Issue Intake | SECURITY、SUPPORT、issue template、PR template、YAML form を評価できる。 |
| M4 | Recommendation Gate + Report | safe/guarded/manual の finding と `SEIRI_REPORT.md` を生成できる。 |
| M5 | Patch Plan | safe README route patch と report patch を生成できる。 |
| M6 | Codex Plugin Boundary | Codex から audit/plan/fix を呼び、PR 草案へつなげる。 |

初期は M0-M4 を優先します。M5 以降は patch safety と reviewability が十分になってから進めます。

### 5. Verification と calibration

検証は、人気や信頼の保証ではなく、detector と recommendation の妥当性確認です。

| Verification | Purpose |
|---|---|
| fixture repos by type | repo type ごとの false positive を見る。 |
| golden `SeiriReport` snapshots | report の安定性を保つ。 |
| README route roundtrip | README から route を抽出し、人間向け説明へ戻して崩れないか見る。 |
| malformed Markdown/YAML fixtures | parser robustness を見る。 |
| large file fixtures | size threshold と type-dependent warning を確認する。 |
| safe patch diff tests | safe gate が policy や owner を勝手に作らないことを確認する。 |
| benchmark recalibration | aggregate weight を実測 crawl に置き換えられるようにする。 |

### 6. Anti-goals

RepoSeiri がやらないこと:

- 人気が確立済みだと主張しない。
- 単一スコアで repo の価値を断定しない。
- 全 repo に同じ template を押し付けない。
- LICENSE、CODEOWNERS、security SLA、governance を勝手に決めない。
- README に全情報を詰め込む方向へ誘導しない。
- benchmark aggregate を統計的証明として扱わない。

### 7. 今後の設計作業

次に詳細化する順序:

1. `TrustGraph` と `Evidence` の Rust type を定義する。
2. README section classifier の rule と fixture を作る。
3. route strength と missing route の scoring rule を固定する。
4. repo type classifier の feature と confidence を設計する。
5. `SEIRI_REPORT.md` と JSON report schema を固定する。
6. safe/guarded/manual patch policy をテスト可能な rule に落とす。

---

## English

### 1. Core IR

The Rust implementation should build typed IR before rendering string reports.

| IR | Main fields | Role |
|---|---|---|
| `RepoSnapshot` | path, remote, default_branch, file_index, git_metadata | Low-level snapshot of the input repository. |
| `Evidence` | kind, path, span, source, confidence, inherited | Smallest unit of detected evidence. |
| `TrustNode` | node_kind, label, location | Node such as README, docs, or policy file. |
| `TrustRoute` | route_kind, from, to, status, strength, blockers | Route state. |
| `TrustGraph` | nodes, routes, missing_routes, contradictions | Trust structure of the repository. |
| `Finding` | id, severity, route, evidence_refs, profile_scope | Problem or improvement opportunity. |
| `Recommendation` | action, gate, target_files, rationale, risk | Proposal. |
| `PatchPlan` | edits, created_files, skipped_actions, manual_actions | Actual diff plan. |
| `TargetProfile` | repo_type, goal, weights, required_routes | Purpose-specific overlay. |
| `SeiriReport` | summary, score_view, findings, recommendations | Shared source for human report and JSON output. |

### 2. Rust Module Boundaries

Low-level-first crate / module boundaries:

| Module | Responsibility |
|---|---|
| `seiri-fs` | directory walk, file size, binary sniffing, ignore rules. |
| `seiri-git` | Git object, tree, remote, branch, and history metadata. |
| `seiri-markdown` | heading, link, badge, and section classification. |
| `seiri-yaml` | typed parsing for issue forms, Dependabot, and workflow YAML. |
| `seiri-workflow` | GitHub Actions signal, job kind, and security scanner detection. |
| `seiri-core` | IR, graph, scoring, and profile overlay. |
| `seiri-planner` | recommendation gate, patch plan, and diff generation. |
| `seiri-report` | Markdown / JSON report rendering. |
| `seiri-codex` | Codex plugin boundary, tool commands, and PR integration. |

Reasons to implement low-level parts:

- file traversal, Git tree reading, and Markdown link parsing matter across many repositories.
- typed parsers and explicit errors make false-positive causes visible in reports.
- streaming / low-copy design helps large repositories and monorepos.
- external boundaries such as GitHub API, Codex host, and PR creation should use higher-level APIs when they improve safety and compatibility.

### 3. Recommendation Gate

RepoSeiri separates what can be auto-fixed from what people must decide.

| Gate | Meaning | Examples |
|---|---|---|
| Safe | Low-risk generation from existing facts. | Add README route links, create `SEIRI_REPORT.md`, report broken links, organize docs routes. |
| Guarded | Draft can be generated, but project owner confirmation is required. | SUPPORT.md, SECURITY.md skeleton, issue forms, PR template, Dependabot config. |
| Manual | Human judgment is primary. | LICENSE choice, CODEOWNERS ownership, security SLA, governance, large CI redesign, archive decision. |

Safe patch conditions:

- Only add links to existing files or docs.
- Do not contradict existing evidence.
- Do not invent policy, owner, license, or security response promises.
- Do not break existing structures such as bilingual README layout.

### 4. MVP

The MVP is limited to the smallest scope that proves the graph and safety gate.

| Milestone | Scope | Exit condition |
|---|---|---|
| M0 | repo snapshot, file index, Markdown heading/link parse | Can output README route evidence as JSON. |
| M1 | README Router Analyzer | Can classify docs, quickstart, support, contributing, security, license, and release routes. |
| M2 | Docs Topology + Community Health | Can evaluate `/docs`, org defaults, repo `.github`, and LICENSE boundary. |
| M3 | Security / Support / Issue Intake | Can evaluate SECURITY, SUPPORT, issue template, PR template, and YAML forms. |
| M4 | Recommendation Gate + Report | Can generate safe/guarded/manual findings and `SEIRI_REPORT.md`. |
| M5 | Patch Plan | Can generate safe README route patches and report patches. |
| M6 | Codex Plugin Boundary | Can call audit/plan/fix from Codex and prepare PR drafts. |

Prioritize M0-M4 first. Move to M5 and later only after patch safety and reviewability are strong enough.

### 5. Verification And Calibration

Verification is for detector and recommendation validity, not for guaranteeing popularity or trust.

| Verification | Purpose |
|---|---|
| fixture repos by type | Measure false positives by repo type. |
| golden `SeiriReport` snapshots | Keep report output stable. |
| README route roundtrip | Extract routes from README and render them back into human explanation without distortion. |
| malformed Markdown/YAML fixtures | Check parser robustness. |
| large file fixtures | Check size thresholds and type-dependent warnings. |
| safe patch diff tests | Ensure safe gate does not invent policies or owners. |
| benchmark recalibration | Allow aggregate weights to be replaced by measured crawls later. |

### 6. Anti-goals

RepoSeiri does not:

- claim established popularity.
- assert repository value from one score.
- force the same template on every repository.
- choose LICENSE, CODEOWNERS, security SLA, or governance automatically.
- push all information into README.
- treat the benchmark aggregate as statistical proof.

### 7. Next Design Work

Next details to define:

1. Define Rust types for `TrustGraph` and `Evidence`.
2. Create README section classifier rules and fixtures.
3. Fix scoring rules for route strength and missing routes.
4. Design repo type classifier features and confidence.
5. Fix `SEIRI_REPORT.md` and JSON report schema.
6. Lower safe/guarded/manual patch policy into testable rules.
