# Roadmap v9: Semantic Identity And Verification Closure

## 日本語

Roadmap v9は、RepoSeiriの観測が「どこに書かれていたか」「何を意味するか」「どの状態だったか」を混同しないようにし、private calibrationとcompletion evidenceを同じsource境界へ結び付ける実装契約です。非公開分析本文、exact prior、秘密値由来の公開fingerprintは成果物へ含めません。

### 不変条件

- Markdownのheading、link、imageはoffset付きparser eventだけから生成し、fence、indented code、inline code、comment、raw HTML dead zoneを根拠にしません。
- repository pathはUTF-8のrepository-relative pathとして扱い、lossy変換やhost absolute pathを公開surfaceへ出しません。
- test、fixture、example、generated、vendored、submoduleはprimary repository evidenceと区別します。
- private calibrationのsemantic digestは非公開型に閉じます。比較可能性は入力者が明示したopaque revisionだけで与えます。
- evidence fingerprintはidentity、state、occurrenceを分離し、run-local `EvidenceId`をportable recordへ保存しません。
- completionは検証前後のGit index/worktree bindingが一致した場合だけsource-stableです。host manifestは同じsource digestとCargo.lock digestへ結び付きます。
- 実装、ローカル検証、外部host証跡、Git操作は別の状態です。

### Sequential implementation units

| Unit | 実装責任 | 完了条件 |
| --- | --- | --- |
| SI0 | baselineとauthority | worktree、既存test、Git非権限を記録する |
| SI1 | semantic Markdown | offset event、dead-zone除外、正確なspanを持つ |
| SI2 | path privacy | strict repository-relative pathとredacted errorを持つ |
| SI3 | path classification | regionとusageを一つのclassifierで決める |
| SI4 | facet witnesses | semantic manifestと最大2件の決定的witnessを使う |
| SI5 | private calibration | bounded single-read、非公開digest、opaque revisionを使う |
| SI6 | stable digest | domain-separated、length-delimited、fixed-endian SHA-256を使う |
| SI7 | portable delta v2 | run-local IDを除き、explicit stable digestで比較する |
| SI8 | planner provenance v4 | claim、gate、priority、3層fingerprintを保持する |
| SI9 | schema closure | portable audit v2、delta v2、completion v3を固定する |
| SI10 | completion v3 | pre/post source bindingとsource-bound host receiptを検証する |
| SI11 | regression corpus | hostile Markdown、facet、privacy、delta、completion testを持つ |
| SI12 | closure | fmt、check、test、clippy、self-auditとblocked evidenceを分離する |

### 完了状態

`IMPLEMENTED`はsource変更が存在する状態、`LOCALLY_VERIFIED`は同じsourceでblocking checkが通った状態、`EVIDENCE_COMPLETE`はrequired Windows/Linux host receiptまで同じsourceへ結び付いた状態です。環境ポリシーで実行できないcheckはpassへ昇格しません。いずれの状態もcommit、push、merge、release、公開、plugin再インストールを許可しません。

---

## English

Roadmap v9 is the implementation contract that prevents RepoSeiri from conflating where an observation occurred, what it means, and which state it had. It also binds private calibration and completion evidence to explicit source boundaries. Private analysis bodies, exact priors, and public fingerprints derived from secret values are excluded from artifacts.

### Invariants

- Generate Markdown headings, links, and images only from offset parser events. Fences, indented code, inline code, comments, and raw-HTML dead zones are not evidence.
- Treat repository paths as UTF-8 repository-relative paths. Do not emit lossy conversions or host absolute paths on public surfaces.
- Distinguish tests, fixtures, examples, generated content, vendored content, and submodules from primary repository evidence.
- Keep the semantic digest of private calibration in a private type. Only an input-owner supplied opaque revision grants comparability.
- Separate evidence fingerprints into identity, state, and occurrence. Do not store run-local `EvidenceId` values in portable records.
- Completion is source-stable only when the Git index/worktree binding is identical before and after verification. Host manifests bind to the same source digest and Cargo.lock digest.
- Implementation, local verification, external host evidence, and Git operations are separate states.

### Sequential Implementation Units

| Unit | Implementation responsibility | Completion condition |
| --- | --- | --- |
| SI0 | baseline and authority | Record worktree, existing tests, and lack of Git authority |
| SI1 | semantic Markdown | Use offset events, dead-zone exclusion, and exact spans |
| SI2 | path privacy | Use strict repository-relative paths and redacted errors |
| SI3 | path classification | Decide region and usage in one classifier |
| SI4 | facet witnesses | Use semantic manifests and at most two deterministic witnesses |
| SI5 | private calibration | Use one bounded read, a private digest, and opaque revisions |
| SI6 | stable digest | Use domain-separated, length-delimited, fixed-endian SHA-256 |
| SI7 | portable delta v2 | Remove run-local IDs and compare explicit stable digests |
| SI8 | planner provenance v4 | Retain claims, gates, priorities, and three-layer fingerprints |
| SI9 | schema closure | Freeze portable audit v2, delta v2, and completion v3 |
| SI10 | completion v3 | Verify pre/post source binding and source-bound host receipts |
| SI11 | regression corpus | Cover hostile Markdown, facets, privacy, delta, and completion |
| SI12 | closure | Separate fmt, check, test, clippy, self-audit, and blocked evidence |

### Completion States

`IMPLEMENTED` means the source changes exist. `LOCALLY_VERIFIED` means blocking checks passed against the same source. `EVIDENCE_COMPLETE` additionally requires Windows and Linux host receipts bound to that source. A check blocked by environment policy is never promoted to pass. No state grants authority to commit, push, merge, release, publish, or reinstall the plugin.
