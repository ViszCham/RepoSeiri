# Documentation Topology

## 日本語

RepoSeiri の docs topology は、README を入口、`docs/README.md` を文書地図、各専門文書を詳細面として分けます。README は route hub のまま保ち、長い設計、運用手順、release 判断は docs 側へ逃がします。

この topology は、文書が完全であること、常に最新であること、または RepoSeiri の品質を保証するものではありません。どの文書を正として読むかを明確にするための整理です。

### First-read order

| Step | Entry | Role |
| --- | --- | --- |
| 1 | [README](../README.md) | 最初の route hub。概要、quickstart、主要 command、root route だけを置きます。 |
| 2 | [Documentation Topology](README.md) | docs 全体の地図。どこに何を書くかを決めます。 |
| 3 | [Publication Readiness](publication-readiness.md) | 公開リポジトリとして扱うための checklist です。 |
| 4 | [Design Documentation](design/README.md) | Repository Trust Graph、baseline、repair、roadmap の入口です。 |
| 5 | [Release Process](release.md) | release 判断、pre-release check、互換性境界、lifecycle / maintenance 境界の入口です。 |
| 6 | [Changelog](../CHANGELOG.md) | 利用者向け変更履歴の正です。 |
| 7 | [Repository Hygiene](hygiene.md) | `.gitignore`、`.gitattributes`、generated output 境界の入口です。 |
| 8 | [Self-Audit Loop](self-audit.md) | RepoSeiri が RepoSeiri 自身を確認する loop の入口です。 |

### Topology

| Area | Files | Owns | Does not own |
| --- | --- | --- | --- |
| Root route hub | `README.md` | 初回導線、主要 command、root policy への入口 | 詳細設計、長い運用手順、release 手順 |
| Docs topology | `docs/README.md` | 文書地図、文書追加ルール、所有境界 | 個別設計の本文、policy decision |
| Design docs | `docs/design/` | trust graph、baseline、profile、repair、roadmap | user support、security disclosure、release history |
| Release docs | `docs/release.md`, `CHANGELOG.md` | release 手順、互換性境界、変更履歴 | release 自動化、package publication |
| Hygiene docs | `docs/hygiene.md`, `docs/self-audit.md` | repository hygiene、self-audit loop、generated output 境界 | security incident、ownership、release 承認 |
| Publication readiness | `docs/publication-readiness.md`, `.github/CODEOWNERS` | 公開リポジトリとして扱うための checklist、ownership 境界 | visibility 変更の自動化、法務判断、security outcome の判定 |
| Governance | `GOVERNANCE.md` | 個人使用リポとしての意思決定境界、外部 contribution 境界 | 実装設計、support SLA、security outcome の判定 |
| Root policy files | `LICENSE`, `SECURITY.md`, `SUPPORT.md`, `CONTRIBUTING.md` | license、security、support、contribution 方針 | 詳細設計、内部実装計画 |
| GitHub operations | `.github/` | issue intake、PR template、workflow、dependency bot | maintainer の手動 release 判断 |

### Source of truth

| Question | Read first |
| --- | --- |
| RepoSeiri とは何か | [README](../README.md) |
| 公開状態の確認 | [Publication Readiness](publication-readiness.md) |
| lifecycle / maintenance 境界 | [Release Process](release.md) |
| 設計思想とtrust route | [Repository Trust Graph](design/repository-trust-graph.md) |
| baseline、profile、分析データの扱い | [Baseline And Profiles](design/baseline-and-profiles.md) |
| Rust module、gate、verification | [Repair, Implementation, And Verification](design/repair-implementation-and-verification.md) |
| Q12-Q34 evidence kernel、coverage、route/content gap、scanner、patch proposal、Codex view | [Low-level Claim Boundary Roadmap](design/low-level-claim-boundary-roadmap.md) |
| block roadmap と将来拡張 | [Roadmap And Implementation Blocks](design/roadmap-and-implementation-blocks.md) |
| release と互換性 | [Release Process](release.md) |
| 変更履歴 | [Changelog](../CHANGELOG.md) |
| security report | [Security Policy](../SECURITY.md) |
| support route | [Support](../SUPPORT.md) |
| contribution route | [Contributing](../CONTRIBUTING.md) |
| governance boundary | [Governance](../GOVERNANCE.md) |
| hygiene boundary | [Repository Hygiene](hygiene.md) |
| self-audit loop | [Self-Audit Loop](self-audit.md) |

### Documentation rules

- 新しい文書は、どの area に属するかを決めてから追加します。
- README に直接リンクを増やす前に、docs topology または design topology へ逃がせないかを確認します。
- root policy file の内容を docs 側で再定義しません。docs は正の policy file へ送ります。
- 日本語前半、英語後半の前提を、人間向け主要文書で維持します。
- RepoSeiri score、route state、profile confidence は review aid であり、品質や信頼の保証として書きません。

### Adding a document

1. 文書の目的を `design`、`release`、`policy`、`operations`、`hygiene` のどれかに分類します。
2. 既存の正の文書を置き換えるのか、補助するのかを決めます。
3. この topology または該当 subindex に入口を追加します。
4. root README に直接追加するのは、first-read route として必要な場合だけにします。
5. `cargo run --quiet -p seiri-cli -- audit --path . --profile library --format markdown` で README route map を確認します。

---

## English

RepoSeiri docs topology separates the README as the entry point, `docs/README.md` as the documentation map, and specialized documents as detail surfaces. The README stays a route hub, while long design material, operational procedure, and release decisions move into docs.

This topology does not guarantee that the documentation is complete, always current, or that RepoSeiri has a particular quality level. It clarifies which document should be treated as authoritative for each question.

### First-read order

| Step | Entry | Role |
| --- | --- | --- |
| 1 | [README](../README.md) | First route hub. It keeps overview, quickstart, main commands, and root routes only. |
| 2 | [Documentation Topology](README.md) | Map of all docs. It decides where each kind of content belongs. |
| 3 | [Publication Readiness](publication-readiness.md) | Checklist for treating the repository as a public repository. |
| 4 | [Design Documentation](design/README.md) | Entry for Repository Trust Graph, baseline, repair, and roadmap material. |
| 5 | [Release Process](release.md) | Entry for release decisions, pre-release checks, compatibility boundaries, and lifecycle / maintenance boundaries. |
| 6 | [Changelog](../CHANGELOG.md) | Canonical user-facing change history. |
| 7 | [Repository Hygiene](hygiene.md) | Entry for `.gitignore`, `.gitattributes`, and generated-output boundaries. |
| 8 | [Self-Audit Loop](self-audit.md) | Entry for the loop where RepoSeiri checks RepoSeiri itself. |

### Topology

| Area | Files | Owns | Does not own |
| --- | --- | --- | --- |
| Root route hub | `README.md` | First route, main commands, and entries to root policies | Detailed design, long operating procedure, release procedure |
| Docs topology | `docs/README.md` | Documentation map, document-addition rules, and ownership boundaries | Individual design body, policy decisions |
| Design docs | `docs/design/` | Trust graph, baseline, profile, repair, and roadmap | User support, security disclosure, release history |
| Release docs | `docs/release.md`, `CHANGELOG.md` | Release procedure, compatibility boundaries, and change history | Release automation, package publication |
| Hygiene docs | `docs/hygiene.md`, `docs/self-audit.md` | Repository hygiene, self-audit loop, and generated-output boundaries | Security incidents, ownership, release approval |
| Publication readiness | `docs/publication-readiness.md`, `.github/CODEOWNERS` | Checklist for treating the repository as public, and ownership boundary | Automating visibility changes, legal judgment, security outcome decisions |
| Governance | `GOVERNANCE.md` | Decision boundary and external contribution boundary for a personal-use repository | Implementation design, support SLA, security outcome decisions |
| Root policy files | `LICENSE`, `SECURITY.md`, `SUPPORT.md`, `CONTRIBUTING.md` | License, security, support, and contribution policy | Detailed design and internal implementation plans |
| GitHub operations | `.github/` | Issue intake, PR template, workflow, and dependency bot | Manual maintainer release decisions |

### Source of truth

| Question | Read first |
| --- | --- |
| What RepoSeiri is | [README](../README.md) |
| Public-state checks | [Publication Readiness](publication-readiness.md) |
| Lifecycle / maintenance boundary | [Release Process](release.md) |
| Design philosophy and trust routes | [Repository Trust Graph](design/repository-trust-graph.md) |
| Baselines, profiles, and analysis data handling | [Baseline And Profiles](design/baseline-and-profiles.md) |
| Rust modules, gates, and verification | [Repair, Implementation, And Verification](design/repair-implementation-and-verification.md) |
| Q12-Q34 evidence kernel, coverage, route/content gaps, scanner, patch proposal, and Codex views | [Low-level Claim Boundary Roadmap](design/low-level-claim-boundary-roadmap.md) |
| Block roadmap and future extension | [Roadmap And Implementation Blocks](design/roadmap-and-implementation-blocks.md) |
| Release and compatibility | [Release Process](release.md) |
| Change history | [Changelog](../CHANGELOG.md) |
| Security reporting | [Security Policy](../SECURITY.md) |
| Support route | [Support](../SUPPORT.md) |
| Contribution route | [Contributing](../CONTRIBUTING.md) |
| Governance boundary | [Governance](../GOVERNANCE.md) |
| Hygiene boundary | [Repository Hygiene](hygiene.md) |
| Self-audit loop | [Self-Audit Loop](self-audit.md) |

### Documentation rules

- Decide which area a new document belongs to before adding it.
- Before adding more direct links to the README, check whether the content can move through the docs topology or design topology.
- Do not redefine root policy files inside docs. Docs route readers to the authoritative policy files.
- Keep the Japanese-first and English-second premise for major human-facing documents.
- RepoSeiri scores, route states, and profile confidence are review aids; do not describe them as quality or trust guarantees.

### Adding a document

1. Classify the document purpose as `design`, `release`, `policy`, `operations`, or `hygiene`.
2. Decide whether it replaces an authoritative document or supports one.
3. Add an entry to this topology or the relevant subindex.
4. Add a direct root README entry only when it is needed as a first-read route.
5. Run `cargo run --quiet -p seiri-cli -- audit --path . --profile library --format markdown` and check the README route map.
