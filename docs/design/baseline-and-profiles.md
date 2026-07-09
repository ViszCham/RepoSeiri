# Baseline And Profiles

## 日本語

### 1. データから採用する設計圧力

分析データでは、上位リポの品質は README 単体ではなく、README から他の運用面へ正しく導線がつながるかで決まります。

初期重みとして使う主要傾向:

| Signal | Rate | RepoSeiri での扱い |
|---|---:|---|
| README exists | 98.6% | ほぼ必須の入口。欠落は強い赤信号。 |
| README has docs / website route | 81.4% | README routing の中核。 |
| LICENSE file exists | 92.0% | OSS 配布では critical。org default で代替しない。 |
| CI workflow exists | 77.1% | 信頼 signal。ただし docs/spec では軽く扱う。 |
| CONTRIBUTING.md or guide exists | 67.5% | 外部貢献があるリポで重要。 |
| README has quickstart | 56.2% | usability signal。repo type で重みを変える。 |
| SUPPORT route exists | 49.7% | Issues へのサポート流入を制御する導線。 |
| SECURITY policy exists or inherited | 44.2% | SDK、CLI、infra、runtime で重要。 |
| Issue forms YAML exist | 17.8% | 普及率は低いが改善余地が大きい。 |
| Fuzzing signal exists | 8.9% | runtime、infra、security-sensitive repo で高価値。 |

初期改善 gap として優先するもの:

| Gap | Priority | RepoSeiri の設計反映 |
|---|---|---|
| Issue forms YAML missing | High | issue intake の構造化提案。ただし一律自動生成しない。 |
| Security policy missing | High | repo type と公開範囲を見て guarded proposal にする。 |
| Support route missing or weak | High | README、SUPPORT.md、Discussions への導線を提案する。 |
| CODEOWNERS missing | Medium-High | 存在ではなく coverage を評価し、原則 manual gate。 |
| Dependency bot config missing | Medium-High | supply-chain hygiene として guarded proposal。 |
| Quickstart missing or weak | Medium | library、CLI、tutorial で強く見る。 |
| Docs route missing or weak | Medium | README が詳細を抱え込みすぎる場合も検出する。 |
| LICENSE missing | Critical | OSS では最優先。内容選択は人間判断。 |

### 2. Common Baseline を先に実装する理由

Common baseline は「どの repo でも同じ正解」を意味しません。どの repo でも観測すべき evidence を先に安定化する、という意味です。

最初に実装する common baseline:

| Baseline | Detector |
|---|---|
| README exists and has identity | root README、title、summary、description。 |
| README route table | docs、quickstart、support、contributing、security、release、license link。 |
| Docs topology | external docs、`/docs`、wiki、docs repo、developer guide。 |
| Community health inheritance | repo `.github`、root、docs、org `.github`。LICENSE は継承扱いにしない。 |
| Support route | SUPPORT.md、Discussions、issue chooser、forum link。 |
| Security route | SECURITY.md、private disclosure、CodeQL、fuzzing、dependency bot。 |
| Issue/PR intake | issue templates、YAML forms、required fields、security redirection。 |
| CI signal | workflow、test/build/release/security job、badge。 |
| Release route | CHANGELOG、GitHub Releases、versioning、compatibility note。 |
| Ownership | CODEOWNERS existence and rough coverage。 |
| Hygiene | repo size、large file、binary、generated output、LFS。 |
| Lifecycle | archive flag、deprecation text、last release/push signal。 |

この順序が堅い理由は、README routing、docs topology、community health、security/support route が repo type overlay の前提入力になるためです。

### 3. Target Profiles

目的別 profile は、common baseline の上に重みを重ねます。profile は一つに固定せず、複数候補を出して confidence を持たせます。

| Profile | Strong signals | Priority routes | Typical recommendations |
|---|---|---|---|
| Library / SDK | package manifests, API examples, semver, client code | install, API docs, quickstart, release, security | install example、API docs route、changelog、SECURITY、dependency bot。 |
| CLI / Tool / Agent | binaries, commands, config, releases | quickstart, install, support, release, security | first command、binary install、troubleshooting、release compatibility。 |
| Cloud Native / Infra | manifests, operators, Helm, deployment docs | docs, operations, security, community, ownership | ops docs route、CODEOWNERS coverage、security automation、support split。 |
| Product / Application | app docs, user support, screenshots, issue triage | support, docs, issue forms, release | SUPPORT、bug form、feature request form、release notes。 |
| Runtime / Compiler | toolchain, language docs, RFCs, tests | governance, build, release, security | governance/RFC route、build guide、security policy、release train docs。 |
| Docs / Spec / Governance | specs, proposals, website, process docs | structure, contribution, governance, freshness | contribution flow、proposal template、staleness signal、docs topology。 |
| Tutorial / Examples | examples, notebooks, sample apps | prerequisites, quickstart, reproducibility, data policy | prerequisites、run command、dataset location、expected output。 |
| ML / Data / Research | models, datasets, notebooks, papers | reproducibility, data license, large files, environment | model/data license、artifact storage、environment lock、large file policy。 |
| Template / Action / Operator | action.yml, template files, controller/operator code | usage example, version pinning, workflow security, release | pinned examples、permission notes、release tag guidance、security warning。 |

目的別 profile の原則:

- 欠落検出は repo type に依存して重みを変える。
- repo type の confidence が低い場合は、断定ではなく複数候補と差分を出す。
- docs/spec repo に heavy CI を要求しない。
- tutorial repo に enterprise governance を要求しない。
- SDK、CLI、infra、runtime では security route 欠落を強く扱う。
- ML/data repo では large file を単純減点せず、storage policy と license を見る。

### 4. Scoring と report の扱い

スコアは repo の価値を決める単一の真実ではなく、改善順序を決める view として扱います。

初期 weight:

| Category | Weight |
|---|---:|
| Identity / Discoverability | 15 |
| README routing | 15 |
| Docs / usability | 20 |
| Community / contribution | 15 |
| Security / supply chain | 20 |
| Maintenance / lifecycle | 10 |
| Repository hygiene | 5 |

Report は、点数よりも次を前面に出します。

- Missing trust routes
- Broken or weak routes
- Type-dependent risk
- Safe fixes available now
- Guarded proposals that need owner confirmation
- Manual decisions that RepoSeiri must not invent

---

## English

### 1. Design Pressure From The Data

The analysis shows that top repository quality is not determined by README alone. It depends on whether README routes users correctly to the operational surfaces around it.

Major tendencies used as initial weights:

| Signal | Rate | RepoSeiri handling |
|---|---:|---|
| README exists | 98.6% | Almost mandatory entry point. Missing README is a strong red flag. |
| README has docs / website route | 81.4% | Core README routing signal. |
| LICENSE file exists | 92.0% | Critical for OSS distribution. Do not replace with org defaults. |
| CI workflow exists | 77.1% | Trust signal, but lighter for docs/spec repositories. |
| CONTRIBUTING.md or guide exists | 67.5% | Important for repositories with external contribution. |
| README has quickstart | 56.2% | Usability signal. Weight depends on repo type. |
| SUPPORT route exists | 49.7% | Route that keeps support questions out of bug issues. |
| SECURITY policy exists or inherited | 44.2% | Important for SDK, CLI, infra, and runtime repositories. |
| Issue forms YAML exist | 17.8% | Low adoption, but high improvement leverage. |
| Fuzzing signal exists | 8.9% | High value for runtime, infra, and security-sensitive repositories. |

Initial improvement gaps:

| Gap | Priority | RepoSeiri design reflection |
|---|---|---|
| Issue forms YAML missing | High | Propose structured issue intake, but do not auto-generate it blindly. |
| Security policy missing | High | Make a guarded proposal based on repo type and exposure. |
| Support route missing or weak | High | Propose routes through README, SUPPORT.md, or Discussions. |
| CODEOWNERS missing | Medium-High | Evaluate coverage, not only existence, and default to manual gate. |
| Dependency bot config missing | Medium-High | Treat as supply-chain hygiene and use guarded proposal. |
| Quickstart missing or weak | Medium | Weight strongly for libraries, CLIs, and tutorials. |
| Docs route missing or weak | Medium | Also detect when README carries too much detail. |
| LICENSE missing | Critical | Highest priority for OSS. License choice remains human judgment. |

### 2. Why Common Baseline Comes First

Common baseline does not mean "the same correct answer for every repository." It means stabilizing the evidence that should be observed for every repository before applying profiles.

First common baseline:

| Baseline | Detector |
|---|---|
| README exists and has identity | root README, title, summary, description. |
| README route table | docs, quickstart, support, contributing, security, release, license link. |
| Docs topology | external docs, `/docs`, wiki, docs repo, developer guide. |
| Community health inheritance | repo `.github`, root, docs, org `.github`. LICENSE is not inherited. |
| Support route | SUPPORT.md, Discussions, issue chooser, forum link. |
| Security route | SECURITY.md, private disclosure, CodeQL, fuzzing, dependency bot. |
| Issue/PR intake | issue templates, YAML forms, required fields, security redirection. |
| CI signal | workflow, test/build/release/security job, badge. |
| Release route | CHANGELOG, GitHub Releases, versioning, compatibility note. |
| Ownership | CODEOWNERS existence and rough coverage. |
| Hygiene | repo size, large file, binary, generated output, LFS. |
| Lifecycle | archive flag, deprecation text, last release/push signal. |

This order is stable because README routing, docs topology, community health, and security/support routes become inputs for the repo type overlay.

### 3. Target Profiles

Purpose-specific profiles layer weights on top of the common baseline. A profile is not fixed to one answer; RepoSeiri should emit multiple candidates with confidence when needed.

| Profile | Strong signals | Priority routes | Typical recommendations |
|---|---|---|---|
| Library / SDK | package manifests, API examples, semver, client code | install, API docs, quickstart, release, security | install example, API docs route, changelog, SECURITY, dependency bot. |
| CLI / Tool / Agent | binaries, commands, config, releases | quickstart, install, support, release, security | first command, binary install, troubleshooting, release compatibility. |
| Cloud Native / Infra | manifests, operators, Helm, deployment docs | docs, operations, security, community, ownership | ops docs route, CODEOWNERS coverage, security automation, support split. |
| Product / Application | app docs, user support, screenshots, issue triage | support, docs, issue forms, release | SUPPORT, bug form, feature request form, release notes. |
| Runtime / Compiler | toolchain, language docs, RFCs, tests | governance, build, release, security | governance/RFC route, build guide, security policy, release train docs. |
| Docs / Spec / Governance | specs, proposals, website, process docs | structure, contribution, governance, freshness | contribution flow, proposal template, staleness signal, docs topology. |
| Tutorial / Examples | examples, notebooks, sample apps | prerequisites, quickstart, reproducibility, data policy | prerequisites, run command, dataset location, expected output. |
| ML / Data / Research | models, datasets, notebooks, papers | reproducibility, data license, large files, environment | model/data license, artifact storage, environment lock, large file policy. |
| Template / Action / Operator | action.yml, template files, controller/operator code | usage example, version pinning, workflow security, release | pinned examples, permission notes, release tag guidance, security warning. |

Profile rules:

- Missing-route weights depend on repo type.
- If repo type confidence is low, emit multiple candidates and their differences instead of asserting one type.
- Do not require heavy CI from docs/spec repositories.
- Do not require enterprise governance from tutorial repositories.
- Treat missing security routes strongly for SDK, CLI, infra, and runtime repositories.
- For ML/data repositories, do not penalize large files blindly. Check storage policy and license.

### 4. Scoring And Report Handling

The score is not a single truth about repository value. It is a view for ordering improvements.

Initial weights:

| Category | Weight |
|---|---:|
| Identity / Discoverability | 15 |
| README routing | 15 |
| Docs / usability | 20 |
| Community / contribution | 15 |
| Security / supply chain | 20 |
| Maintenance / lifecycle | 10 |
| Repository hygiene | 5 |

The report should foreground these items over the raw score:

- Missing trust routes
- Broken or weak routes
- Type-dependent risk
- Safe fixes available now
- Guarded proposals that need owner confirmation
- Manual decisions RepoSeiri must not invent
