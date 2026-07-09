# Repository Trust Graph Design

## 日本語

### 1. 固定した設計判断

RepoSeiri は、単なる GitHub 掃除ツールではなく、Repository Trust Graph、Trust Path Planner、Safe Repair Engine として設計します。

- Repository Trust Graph は、README、Docs、Security、Contribution、CI、Release、Governance、License などの信頼導線を graph として表現します。
- Trust Path Planner は、リポジトリの目的に応じて不足している導線と過剰な導線を見つけ、次に直すべき順序を出します。
- Safe Repair Engine は、安全に自動生成できる差分、確認付きで生成すべき差分、人間の判断が必要な差分を分けます。
- 目的は「人気になることの保証」ではありません。人気かつ信頼されているリポに多い構造を、観測可能な evidence として扱い、リポジトリの目的に合わせて整えます。
- benchmark aggregate は、初期の重み付けと優先順位付けの材料です。完全な 10,000 リポジトリ全件 root tree crawl や統計的証明として扱いません。
- 実装順序は、全体に共通して見られる observable evidence を先に実装し、その上に目的別 profile を重ねます。

短い定義:

```text
RepoSeiri は、リポジトリの目的に応じて、信頼されるための導線を見える化し、安全に整える。
```

### 2. 設計ドキュメント構成

詳細設計は、主設計を入口にして分割します。

| Document | Role |
|---|---|
| `repository-trust-graph.md` | 固定した製品定義、trust graph、README/docs の役割、全体 architecture。 |
| `baseline-and-profiles.md` | benchmark aggregate から採用する共通傾向、common baseline、目的別 profile、scoring。 |
| `repair-implementation-and-verification.md` | Core IR、Rust module 境界、safe/guarded/manual gate、MVP、verification。 |

README はこの主設計 doc へ導線を置き、詳細な判断は docs/design 配下へ逃がします。

### 3. 製品定義

RepoSeiri は、単一リポと組織内複数リポの両方を対象にします。初期実装は単一リポから始め、IR と report 形式を組織集計へ拡張できるようにします。

想定する user workflow:

| Workflow | Purpose | Output |
|---|---|---|
| `seiri audit` | 現状を読む | `SEIRI_REPORT.md` または JSON report |
| `seiri plan` | 次に直す順序を決める | Finding と recommendation の一覧 |
| `seiri fix --safe` | 安全な routing 修正だけ生成する | reviewable patch |
| `seiri fix --guarded` | テンプレートや policy 草案を生成する | 確認前提の patch plan |
| `seiri archive-check` | archive / deprecated 候補を確認する | release、issue、README、migration 導線の checklist |
| Codex plugin action | Codex に分析と PR 生成を任せる | 説明付きの改善 PR |

### 4. Repository Trust Graph

信頼導線は次のような graph として扱います。

```text
README
  -> Docs
  -> Quickstart
  -> Support
  -> Contributing
  -> Security
  -> Release
  -> Governance
  -> License
  -> Automation
  -> Ownership
  -> Hygiene
```

各 route は、存在だけではなく、到達可能性、説明の明確さ、repo type との適合、壊れた link、重複、矛盾を持ちます。

| Route | Evidence examples | Meaning |
|---|---|---|
| Identity | README title, description, topics, badges | 何のリポかすぐ分かるか。 |
| Docs | external docs, `/docs`, wiki, docs repo | 詳細が適切な場所へ逃がされているか。 |
| Quickstart | install, first run, minimal example | 初回利用までの距離。 |
| Support | SUPPORT.md, Discussions, forum, Slack, issue chooser | 質問と bug report を分けられるか。 |
| Contributing | CONTRIBUTING, dev guide, good first issue | 外部貢献が再現可能か。 |
| Security | SECURITY.md, disclosure route, CodeQL, fuzzing | 脆弱性報告と supply-chain hygiene。 |
| Release | releases, changelog, versioning, compatibility | 利用者が更新判断できるか。 |
| Governance | RFC, proposal, roadmap, steering docs | 意思決定経路が必要な repo type で見えるか。 |
| License | LICENSE file | 利用と再配布の前提が明確か。 |
| Automation | CI, release workflow, dependency bot | 信頼 signal が自動化されているか。 |
| Ownership | CODEOWNERS, maintainer docs | 責任境界が見えるか。 |
| Hygiene | repo size, large files, generated files, LFS | ソース tree が用途に合っているか。 |

### 5. README と docs への逃がし方

README は manual ではなく routing hub として扱います。

README に残すもの:

- 一文 summary
- この repo を使う理由
- install または first run への最短導線
- docs、support、contributing、security、license、release への link
- 必要な badge と信頼 signal
- repo type に応じた最小限の注意事項

README から逃がすもの:

| Content | Destination |
|---|---|
| 長い API 説明 | docs site、API reference、`/docs` |
| 開発者向け手順 | CONTRIBUTING、developer guide |
| セキュリティ報告 | SECURITY.md |
| サポート質問 | SUPPORT.md、Discussions、forum |
| 大きな設計判断 | RFC、proposal、governance docs |
| release 互換性 | CHANGELOG、release notes、compatibility policy |
| 大容量 data / model | release assets、object storage、LFS policy |

RepoSeiri の README analyzer は、README が短いことを自動的に悪く判定しません。短くても必要な route があるなら高く評価します。逆に README が長くても、trust route が壊れているなら改善対象にします。

### 6. アーキテクチャ

初期 architecture は次の pipeline に固定します。

```text
Evidence Scanner
  -> Repository Trust Graph
  -> Common Baseline
  -> Target Profile Overlay
  -> Recommendation Gate
  -> Patch Plan
  -> Report / PR
```

役割:

| Layer | Role |
|---|---|
| Evidence Scanner | file、Git tree、Markdown、YAML、workflow、GitHub metadata から evidence を集める。 |
| Repository Trust Graph | route、node、edge、missing route、broken route、contradiction を表現する。 |
| Common Baseline | 全 repo type で観測する共通 evidence を評価する。 |
| Target Profile Overlay | library、CLI、infra、docs、tutorial などの目的別重みを重ねる。 |
| Recommendation Gate | safe、guarded、manual に分ける。 |
| Patch Plan | 差分生成可能な単位へ落とす。 |
| Report / PR | 人間が review できる説明と patch を出す。 |

### 7. 次の詳細設計

次に読むべき詳細:

1. [Baseline And Profiles](baseline-and-profiles.md)
2. [Repair, Implementation, And Verification](repair-implementation-and-verification.md)

---

## English

### 1. Fixed Design Decision

RepoSeiri is designed not as a simple GitHub cleanup tool, but as a Repository Trust Graph, Trust Path Planner, and Safe Repair Engine.

- Repository Trust Graph represents trust routes such as README, Docs, Security, Contribution, CI, Release, Governance, and License as a graph.
- Trust Path Planner finds missing and excessive routes according to the repository purpose, then orders what should be fixed next.
- Safe Repair Engine separates diffs that can be generated safely, diffs that need confirmation, and diffs that require human judgment.
- The goal is not to guarantee popularity. RepoSeiri treats structures commonly found in popular and trusted repositories as observable evidence, then adapts them to the repository purpose.
- The benchmark aggregate is input for initial weighting and prioritization. It is not treated as a complete 10,000-repository root-tree crawl or statistical proof.
- The implementation order is to build common observable evidence first, then layer purpose-specific profiles on top.

Short definition:

```text
RepoSeiri helps a repository expose the right trust routes for its purpose and safely organize them.
```

### 2. Design Document Structure

The detailed design is split with this main design as the entry point.

| Document | Role |
|---|---|
| `repository-trust-graph.md` | Fixed product definition, trust graph, README/docs roles, and overall architecture. |
| `baseline-and-profiles.md` | Common tendencies adopted from the benchmark aggregate, common baseline, target profiles, and scoring. |
| `repair-implementation-and-verification.md` | Core IR, Rust module boundaries, safe/guarded/manual gate, MVP, and verification. |

README keeps the route to this main design document, while detailed decisions move under docs/design.

### 3. Product Definition

RepoSeiri targets both a single repository and multiple repositories inside an organization. The initial implementation starts with one repository, while keeping the IR and report format extensible to organization-level aggregation.

Expected user workflow:

| Workflow | Purpose | Output |
|---|---|---|
| `seiri audit` | Read the current state | `SEIRI_REPORT.md` or JSON report |
| `seiri plan` | Decide the next fix order | List of findings and recommendations |
| `seiri fix --safe` | Generate only safe routing fixes | Reviewable patch |
| `seiri fix --guarded` | Draft templates or policy files | Patch plan that requires confirmation |
| `seiri archive-check` | Check archive / deprecated candidates | Checklist for release, issues, README, and migration routes |
| Codex plugin action | Let Codex analyze and prepare a PR | Improvement PR with explanation |

### 4. Repository Trust Graph

Trust routes are represented as a graph like this:

```text
README
  -> Docs
  -> Quickstart
  -> Support
  -> Contributing
  -> Security
  -> Release
  -> Governance
  -> License
  -> Automation
  -> Ownership
  -> Hygiene
```

Each route carries not only existence, but also reachability, clarity, repo-type fit, broken links, duplication, and contradictions.

| Route | Evidence examples | Meaning |
|---|---|---|
| Identity | README title, description, topics, badges | Whether the repository purpose is immediately clear. |
| Docs | external docs, `/docs`, wiki, docs repo | Whether detailed material is routed to the right place. |
| Quickstart | install, first run, minimal example | Distance to first successful use. |
| Support | SUPPORT.md, Discussions, forum, Slack, issue chooser | Whether questions and bug reports can be separated. |
| Contributing | CONTRIBUTING, dev guide, good first issue | Whether external contribution is reproducible. |
| Security | SECURITY.md, disclosure route, CodeQL, fuzzing | Vulnerability reporting and supply-chain hygiene. |
| Release | releases, changelog, versioning, compatibility | Whether users can judge update risk. |
| Governance | RFC, proposal, roadmap, steering docs | Whether decision paths are visible where needed. |
| License | LICENSE file | Whether use and redistribution terms are clear. |
| Automation | CI, release workflow, dependency bot | Whether trust signals are automated. |
| Ownership | CODEOWNERS, maintainer docs | Whether responsibility boundaries are visible. |
| Hygiene | repo size, large files, generated files, LFS | Whether the source tree fits the repository purpose. |

### 5. README And Docs Escape Policy

README is treated as a routing hub, not as a manual.

Keep in README:

- One-sentence summary
- Why this repository matters
- Shortest route to install or first run
- Links to docs, support, contributing, security, license, and release information
- Required badges and trust signals
- Minimal repo-type-specific cautions

Move out of README:

| Content | Destination |
|---|---|
| Long API explanation | docs site, API reference, `/docs` |
| Developer procedures | CONTRIBUTING, developer guide |
| Security reporting | SECURITY.md |
| Support questions | SUPPORT.md, Discussions, forum |
| Large design decisions | RFC, proposal, governance docs |
| Release compatibility | CHANGELOG, release notes, compatibility policy |
| Large data / model artifacts | release assets, object storage, LFS policy |

The RepoSeiri README analyzer does not automatically penalize a short README. A short README scores well if the required routes exist. A long README still becomes an improvement target if trust routes are broken.

### 6. Architecture

The initial architecture is fixed to this pipeline:

```text
Evidence Scanner
  -> Repository Trust Graph
  -> Common Baseline
  -> Target Profile Overlay
  -> Recommendation Gate
  -> Patch Plan
  -> Report / PR
```

Responsibilities:

| Layer | Role |
|---|---|
| Evidence Scanner | Collect evidence from files, Git tree, Markdown, YAML, workflows, and GitHub metadata. |
| Repository Trust Graph | Represent routes, nodes, edges, missing routes, broken routes, and contradictions. |
| Common Baseline | Evaluate common evidence observed across repo types. |
| Target Profile Overlay | Layer purpose-specific weights such as library, CLI, infra, docs, or tutorial. |
| Recommendation Gate | Separate safe, guarded, and manual actions. |
| Patch Plan | Lower recommendations into diff-generatable units. |
| Report / PR | Produce reviewable explanation and patches. |

### 7. Next Detailed Design

Read these details next:

1. [Baseline And Profiles](baseline-and-profiles.md)
2. [Repair, Implementation, And Verification](repair-implementation-and-verification.md)
