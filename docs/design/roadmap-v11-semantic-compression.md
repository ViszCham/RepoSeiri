# Roadmap v11: Semantic Compression And Product Closure

## 日本語

### 1. 目的

Roadmap v11は、RepoSeiriの機能を単純に減らす計画ではありません。同じrepository observationをroute、content、claim、review、planner、wireが別々に再構成する状態を解消し、各意味に一つの正本を与える実装契約です。

圧縮はLOC、crate数、型数の削減ではなく、次で判定します。

1. 一つの意味を一つのownerが保持する。
2. source bytesを一つのbounded sessionから読む。
3. parser eventを一度index化し、各content slotで再構成しない。
4. canonical stateとderived viewを分ける。
5. public wireの意味変更をsemantic revisionまたは新versionで明示する。

private analysis body、exact prior、private path、秘密値由来fingerprintはsource、fixture、schema、log、report、bundle、public digestへ移しません。

### 2. 実装block

| Block | Owner | 完了条件 |
| --- | --- | --- |
| K0 | baseline obligation ledger | 現行contract、fixture、self-audit、既知failure、意図的semantic deltaを固定する |
| K1 | bounded source session | repository inputが一つのbounded readerとsource storeを通り、plannerが再読込しない |
| K2 | semantic index | visible eventを一度だけ正規化し、code/commentを除外したtoken/postingからcontentを評価する |
| K3 | route axes | artifact、README entrypoint、reachability、freshness、conflict、inheritance、policyを保持し、`RouteState`を表示projectionに限定する |
| K4 | language topology | 日本語前半・英語後半、monolingual、ambiguousを区別し、unpaired editをsafeにしない |
| K5 | canonical registry | route slug、日英label、target selector、policyとprofile slugの重複ownerを除く |
| K6 | analysis core | canonical storeを正とし、claim、finding、priority、queryを同じevidence identityから導出する |
| K7 | review model | diagnosis、evidence、severity、authority、boundary、recommendationを一つのreview projectionへ揃える |
| K8 | planner | source-bound diff、language-aware operation、manual draft、counterfactual deltaをpreview-onlyで生成する |
| K9 | wire and errors | shadow projection、closed schema、typed error、semantic revisionによってsilent meaning changeを防ぐ |
| K10 | product surface | default summaryを状態、根拠、次の行動、境界へ圧縮し、完全JSONを保持する |
| K11 | API, crate, docs topology | 独立invariantを持たないpublic/API/doc ownerだけを統合し、crate数自体を目標にしない |
| K12 | regression and completion | fmt、test、clippy、MSRV、schema、privacy、determinism、fuzz、completion、bundle evidenceを同じsourceへ結ぶ |

### 3. 許可するsemantic delta

- bilingual topologyが不明なpatchは`Safe`からholdへ変わります。
- document role外のsubstring一致によるcontent evidenceは消えます。
- route axesの修正に伴うsummary、claim、reviewの表示差分はsemantic revisionへ結びます。
- Markdownの既定表示は短くできますが、JSONのevidence、coverage、authority boundaryは失いません。

上記以外のfixture差分は回帰です。test passはrepository全体の正しさ、人気、信頼、安全性、法的適合性、公開準備完了を証明しません。

### 4. Rust実装原則

- bounded `File::take(limit + 1)`、repository-relative path、closed enum、newtype、typestate、固定長stack、borrowed viewを優先します。
- `unsafe`、mmap、async、parallelism、万能graph DSLは、明示的な必要条件と追加verificationなしでは導入しません。
- route/profileはclosed static registry、外部patternはbounded declarative predicate programとして扱います。
- wire objectをcanonical runtime storeとして使いません。wireはvalidated projectionです。

### 5. 完成境界

`ready_for_git`は同じsourceに対するrequired local checkの成功だけを表します。host verification、calibration、manual policy、evidence completenessを別claimとして保持します。Roadmap v11とR11-SCCP-v1はcommit、push、merge、release、plugin install、restart、visibility changeの権限を付与しません。

---

## English

### 1. Purpose

Roadmap v11 is not a plan to remove RepoSeiri features mechanically. It is the implementation contract for stopping route, content, claim, review, planner, and wire surfaces from independently reconstructing the same repository observation, and for assigning one canonical owner to each meaning.

Compression is not judged by LOC, crate count, or type count. It requires:

1. One owner for one meaning.
2. One bounded source session for source bytes.
3. One semantic index for parser events instead of per-slot reconstruction.
4. A separation between canonical state and derived views.
5. Explicit semantic revisions or new wire versions for public meaning changes.

Private analysis bodies, exact priors, private paths, and fingerprints derived from secret values never move into source, fixtures, schemas, logs, reports, bundles, or public digests.

### 2. Implementation Blocks

| Block | Owner | Completion condition |
| --- | --- | --- |
| K0 | baseline obligation ledger | Freeze current contracts, fixtures, self-audit, known failures, and intentional semantic deltas |
| K1 | bounded source session | Repository input passes through one bounded reader and source store, and the planner does not reread files |
| K2 | semantic index | Normalize visible events once and evaluate content from code/comment-excluding token postings |
| K3 | route axes | Preserve artifact, README entrypoint, reachability, freshness, conflict, inheritance, and policy; limit `RouteState` to display projection |
| K4 | language topology | Distinguish Japanese-first/English-second, monolingual, and ambiguous documents; never mark an unpaired edit safe |
| K5 | canonical registry | Remove duplicate owners for route slugs, Japanese/English labels, target selectors, policy, and profile slugs |
| K6 | analysis core | Make the canonical store authoritative and derive claims, findings, priorities, and queries from the same evidence identity |
| K7 | review model | Align diagnosis, evidence, severity, authority, boundary, and recommendation in one review projection |
| K8 | planner | Produce source-bound diffs, language-aware operations, manual drafts, and counterfactual deltas in preview-only mode |
| K9 | wire and errors | Prevent silent meaning changes through shadow projection, closed schemas, typed errors, and semantic revisions |
| K10 | product surface | Compress the default summary to state, evidence, next action, and boundary while retaining complete JSON |
| K11 | API, crate, docs topology | Consolidate only public/API/doc owners without independent invariants; do not target crate count itself |
| K12 | regression and completion | Bind fmt, tests, clippy, MSRV, schemas, privacy, determinism, fuzzing, completion, and bundle evidence to one source |

### 3. Allowed Semantic Deltas

- A patch with unknown bilingual topology changes from `Safe` to held.
- Content evidence caused by substring matching outside the allowed document role disappears.
- Summary, claim, and review display changes caused by corrected route axes bind to a semantic revision.
- Default Markdown may become shorter, but JSON evidence, coverage, and authority boundaries remain available.

Any other fixture difference is a regression. A passing test does not prove repository-wide correctness, popularity, trust, security, legal fitness, or publication readiness.

### 4. Rust Implementation Principles

- Prefer bounded `File::take(limit + 1)`, repository-relative paths, closed enums, newtypes, typestates, fixed stacks, and borrowed views.
- Do not add `unsafe`, mmap, async, parallelism, or a universal graph DSL without an explicit requirement and additional verification.
- Keep routes and profiles in closed static registries; treat external patterns as bounded declarative predicate programs.
- Do not use wire objects as the canonical runtime store. A wire object is a validated projection.

### 5. Completion Boundary

`ready_for_git` means only that required local checks passed against the same source. Host verification, calibration, manual policy, and evidence completeness remain separate claims. Roadmap v11 and R11-SCCP-v1 grant no authority to commit, push, merge, release, install a plugin, restart a host, or change repository visibility.
