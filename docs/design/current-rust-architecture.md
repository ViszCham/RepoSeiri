# Current Rust Architecture

## 日本語

この文書はRoadmapではなく、R11実装後の現行Rust構造を記録します。RepoSeiriは一度のbounded filesystem走査で得たrepository-relative recordとbounded source bytesから、Markdown、GitHubローカル文書、evidence、route、review、dry-run patchを導出します。

### データフロー

1. `seiri-fs`がroot境界、symlink、件数、深さ、サイズbudgetを検査し、repository-relative entryを生成します。
2. `seiri-markdown`が選択文書を一度だけ読み、`SourceStore`とbyte-accurateな`DocumentIndex`を生成します。
3. `seiri-core::SemanticIndex`がvisible Markdown eventを一度正規化し、code fence、inline code、HTML commentを検索対象から除外します。
4. `seiri-github-local`は同じ`SourceStore`からYAML、JSON、CODEOWNERSを解析します。
5. `seiri-report`がevidence、coverage、route axes、content slots、consistency、reviewを一つの`RepositoryAnalysis`へ組み立て、派生evidence参照を検証します。
6. `seiri-planner`は`RepositoryAnalysis`だけを読み、filesystemを再読込せず、既存targetへのsource-bound dry-run editまたはtyped holdを生成します。
7. `seiri-codex`はcanonical analysisとplanのborrowed projectionだけを表示します。

### 所有境界

| Crate | 所有する責務 | 所有しない責務 |
| --- | --- | --- |
| `seiri-core` | typed state、source store、semantic index、route registry、analysis integrity | filesystem I/O、CLI表示 |
| `seiri-fs` | bounded traversalとrepository-relative path | Markdown意味解析 |
| `seiri-markdown` | bounded document selection、source read、event IR | route priority |
| `seiri-github-local` | bounded GitHub構造文書parser | network GitHub API |
| `seiri-report` | 一回のaudit組立てと派生整合性 | patch write |
| `seiri-planner` | existing-target edit、skeleton/manual分類、stale binding | filesystem read、file write、policy生成 |
| `seiri-delta` | portable semantic fingerprintと比較 | host absolute pathのidentity化 |
| `seiri-codex` | bounded query projection | command実行、Git/GitHub操作 |

### 低レイヤ不変条件

- source bodyは`Arc<[u8]>`または`Arc<str>`としてsession内で共有し、公開wireへserializeしません。
- source spanはUTF-8 char boundaryとbyte offsetを検査し、planner bindingはbase digestとanchor contextを再検証します。
- routeのartifact、entrypoint、reachability、freshness、conflict、policyを独立軸で保持し、単一stateは表示用projectionです。
- route slug、日英label、target候補、policy境界は`ROUTE_SPECS`だけが所有します。
- public identityはframed SHA-256とrepository-relative inputから作り、host absolute pathとprivate calibration bodyを含めません。
- `AnalysisCoreView`はcanonical evidence、route、content、reviewを借用し、claim、finding、priorityのevidence参照をaudit完了前に検証します。
- plannerは`seiri-fs`へ依存せず、`SourceStore`と`LanguageTopologyIndex`から日英ペアeditを生成します。

### 境界

この構造は人気、信頼、安全性、品質、法的適合性、公開準備完了を保証しません。standard auditはnetwork、file write、Git、GitHub操作を開始しません。

---

## English

This document records the current Rust structure after R11 implementation; it is not a roadmap. RepoSeiri derives Markdown, local GitHub documents, evidence, routes, reviews, and dry-run patches from repository-relative records and bounded source bytes obtained by one bounded filesystem traversal.

### Data Flow

1. `seiri-fs` checks root boundaries, symlinks, entry, depth, and size budgets, then emits repository-relative entries.
2. `seiri-markdown` reads selected documents once and builds the `SourceStore` and byte-accurate `DocumentIndex`.
3. `seiri-core::SemanticIndex` normalizes visible Markdown events once and excludes code fences, inline code, and HTML comments from search.
4. `seiri-github-local` parses YAML, JSON, and CODEOWNERS from the same `SourceStore`.
5. `seiri-report` assembles evidence, coverage, route axes, content slots, consistency, and review into one `RepositoryAnalysis`, then validates derived evidence references.
6. `seiri-planner` reads only `RepositoryAnalysis`, performs no filesystem reread, and emits source-bound dry-run edits to existing targets or typed holds.
7. `seiri-codex` renders borrowed projections of the canonical analysis and plan.

### Ownership Boundaries

| Crate | Owns | Does not own |
| --- | --- | --- |
| `seiri-core` | Typed state, source store, semantic index, route registry, analysis integrity | Filesystem I/O and CLI rendering |
| `seiri-fs` | Bounded traversal and repository-relative paths | Markdown semantics |
| `seiri-markdown` | Bounded document selection, source reads, event IR | Route priority |
| `seiri-github-local` | Bounded GitHub structured-document parsers | Network GitHub APIs |
| `seiri-report` | Single audit assembly and derived consistency | Patch writes |
| `seiri-planner` | Existing-target edits, skeleton/manual classification, stale binding | Filesystem reads, file writes, policy invention |
| `seiri-delta` | Portable semantic fingerprints and comparison | Host absolute paths as identity |
| `seiri-codex` | Bounded query projections | Command execution and Git/GitHub operations |

### Low-Level Invariants

- Source bodies are shared inside the session as `Arc<[u8]>` or `Arc<str>` and are not serialized into public wires.
- Source spans validate UTF-8 character boundaries and byte offsets; planner bindings recheck base digests and anchor context.
- Route artifact, entrypoint, reachability, freshness, conflict, and policy remain independent axes; the single state is a display projection.
- `ROUTE_SPECS` is the sole owner of route slugs, Japanese and English labels, target candidates, and policy boundaries.
- Public identities use framed SHA-256 over repository-relative inputs and exclude host absolute paths and private calibration bodies.
- `AnalysisCoreView` borrows canonical evidence, routes, content, and reviews; audit completion validates evidence references from claims, findings, and priorities.
- The planner has no `seiri-fs` dependency and generates paired Japanese/English edits from `SourceStore` and `LanguageTopologyIndex`.

### Boundary

This structure does not guarantee popularity, trust, safety, quality, legal fitness, or publication readiness. Standard audits do not initiate network access, file writes, Git operations, or GitHub operations.
