# Low-level Claim Boundary Roadmap

## 日本語

### 1. 固定する目的

この roadmap は、RepoSeiri を単なる repository 整理 CLI から、Rust の型で evidence、route state、claim boundary、wording、patch safety を分離する低レイヤな review engine へ寄せるための実装順序です。

現状の RepoSeiri は、file scan、Markdown route scan、pattern registry、profile、safe patch planner、Codex adapter を Rust core に寄せて実装済みです。一方で、claim boundary、report 文言、route meaning、patch operation の一部はまだ文字列中心です。次の実装では、出力文が直接自由文から出るのではなく、型付き evidence と型付き claim から生成される構造に寄せます。

### 2. 批判から固定する方針

| Risk | Design response |
| --- | --- |
| score や route state を品質の根拠のように読めてしまう | `ContentClaim` と `ClaimBoundaryKind` を導入し、許可された主張と禁止された主張を型で分ける。 |
| `reason: String` と `claim_boundary: String` が増えすぎる | 既存 JSON 互換は残しつつ、主要 report 文を claim id と evidence id に結びつける。 |
| `Verified` が security / quality の証明のように誤読される | `RouteMeaningRule` で route state ごとに「示すこと」と「示さないこと」を登録する。 |
| analysis input を runtime rule と混同する | calibration input は reviewable source として扱い、自動採用しない。 |
| local/private analysis が公開出力に漏れる | source visibility と redaction guard を追加し、path、本文、固有内容を public output に出さない。 |
| 低レイヤ化が大改造になり既存 CLI を壊す | 既存 schema を保ち、追加フィールドと新 command から段階的に入れる。 |

### 3. 実行境界

- この文書は実装順序を固定するだけで、GitHub 操作、branch 作成、commit、push、merge、PR 作成を許可しません。
- 実装 block の完了後も、GitHub 的な action は明示指示があるまで行いません。
- local/private calibration input は公開 repo に保存しません。
- public docs には、非公開分析の本文、path、固有ファイル名、未公開の詳細数値を入れません。
- RepoSeiri の出力は review aid であり、人気、信頼、安全性、品質、法務適合、公開可否を保証しません。

### 4. 実装ブロック

| Block | Scope | Files likely touched | Completion criteria |
| --- | --- | --- | --- |
| Q0: Authority / Privacy Guard | 実装前の漏洩防止と作業境界を固定する。 | docs, tests only if needed | private source の path / filename / body が tracked files に入っていない。`git status` が local edits だけを示す。 |
| Q1: Typed Claim Core | `ClaimId`, `ContentClaim`, `ClaimStrength`, `ClaimBoundaryKind`, `MeaningAtom` を core 型として追加する。 | `crates/seiri-core/src/lib.rs` | 型が `Serialize`, `Deserialize`, `Clone`, `Eq` を持つ。既存 `ClaimBoundary` は互換維持。cargo test が通る。 |
| Q2: RouteMeaning Registry | route/state ごとの meaning と non-claim を静的 registry にする。 | `crates/seiri-core`, `crates/seiri-report` | 全 `RouteKind` と主要 `RouteState` に meaning rule がある。`Verified` を品質や安全性の根拠へ昇格させない test がある。 |
| Q3: Lifecycle Route | `RouteKind::Lifecycle` を追加し、release とは別に maintenance / deprecation / support lifecycle を扱う。 | core, fs, markdown, report, patterns, planner | 全 match が exhaustive。pattern group `LIF` と route が一致する。audit/codex output が壊れない。 |
| Q4: ContentClaim Builder | snapshot から evidence-linked claim を生成する。 | `crates/seiri-report`, optional new module | claim は必ず evidence id を持つ。根拠なし claim は生成しない。JSON と Markdown で確認できる。 |
| Q5: Claim-bound Renderer | report / Codex context の主要文を claim 由来に寄せる。 | `crates/seiri-report`, `crates/seiri-codex` | route summary、boundary、priority rationale が claim id または boundary kind を参照する。既存文面の後方互換を保つ。 |
| Q6: Wording Linter | 過剰主張を byte span 付きで検出する。 | new crate or module, CLI, report | `seiri lint-wording --path . --format markdown|json` が動く。禁止語だけでなく許可境界例外も test する。 |
| Q7: Byte-span Markdown Scanner | Markdown heading/link/badge/candidate に byte range / line / column を追加する。 | `crates/seiri-markdown`, core span types | 既存 line-based 出力を壊さず、span 付き token が取得できる。multibyte text の fixture が通る。 |
| Q8: Patch Planner Expansion | patch operation を route と claim boundary に合わせて増やす。 | `crates/seiri-planner`, core, report | `AddClaimBoundaryNote`, `AddLifecycleRoute`, `AddSupportSkeletonDraft`, `AddSecuritySkeletonDraft`, `MoveReadmeDetailToDocsDraft` が型として存在する。すべて preview-only / guarded / manual 境界を守る。 |
| Q9: Local-only Calibration Guard | calibration source に visibility を追加する。 | core, calibration, report | `Public`, `LocalOnly`, `Redacted` を区別する。local-only source は public report / Codex context で redacted になる。 |
| Q10: Codex Adapter v3 | claim summary、wording lint summary、route meaning digest を Codex context に渡す。 | `crates/seiri-codex`, report | Codex context は safe review artifact のまま。branch、PR、GitHub API 操作を行わない。 |
| Q11: Regression Suite | claim、route meaning、lifecycle、wording、privacy guard の regression を固定する。 | fixtures, crate tests | `cargo test --workspace` で regression が走る。private leak guard が tracked text を検査する。 |

### 5. Block別の細かい完了条件

#### Q0: Authority / Privacy Guard

- `rg` で private source の path、filename、本文断片が repo に存在しない。
- roadmap 文書は抽象化した design input だけを書く。
- GitHub 操作は行わない。
- この block は code behavior を変えない。

#### Q1: Typed Claim Core

- `ContentClaim` は `id`, `route`, `state`, `strength`, `evidence_ids`, `allowed_meanings`, `boundaries` を持つ。
- `ClaimStrength` は少なくとも `Observed`, `Inferred`, `Suggested`, `Blocked` を持つ。
- `ClaimBoundaryKind` は、列挙済みの各 non-claim category を typed variant として表現できる。
- `stable_id` または同等の仕組みで claim id を deterministic にできる。

#### Q2: RouteMeaning Registry

- `RouteMeaningRule` は `route`, `state`, `indicates`, `does_not_indicate` を持つ。
- `Absent`, `Implicit`, `Weak`, `Routed`, `Structured`, `Verified`, `Inherited`, `Conflicting`, `Overloaded`, `Stale`, `UnsafeToInvent` の意味境界を扱う。
- `Overridden` は残す場合、意味と非意味を明文化する。不要なら将来の schema migration 対象として扱い、即削除しない。

#### Q3: Lifecycle Route

- `RouteKind::Lifecycle` を追加する。
- route parser が lifecycle / maintenance / deprecation / archival / supported versions の語を拾える。
- `route_priority` と `planner` の match に漏れがない。
- `PatternGroup::Lif` と `RouteKind::Lifecycle` が同じ意味領域を指す。

#### Q4: ContentClaim Builder

- route state report から claim を生成できる。
- missing route priority から suggestion strength の claim を生成できる。
- calibration由来の claim は automatic adoption にならない。
- claim のない自由文 summary は主要判断に使わない。

#### Q5: Claim-bound Renderer

- Markdown report は claim summary section を持つ。
- JSON report は machine-readable claim list を持つ。
- Codex context は claim boundary を短く表示し、詳細は claim id に逃がす。
- 既存 `audit`, `plan`, `codex`, `patterns`, `calibrate` command は壊さない。

#### Q6: Wording Linter

- lint finding は `path`, `line`, `column`, `byte_start`, `byte_end`, `boundary`, `replacement_hint` を持つ。
- README、docs、generated report のどれに対しても使える。
- 境界文として必要な禁止語は allowlist ではなく typed exception として扱う。
- lint は過剰主張の表現を検出するが、法務判断や security diagnosis はしない。

#### Q7: Byte-span Markdown Scanner

- scanner は byte index を保持し、UTF-8文字列でも line/column が破綻しない。
- 既存 `MarkdownHeading`, `MarkdownLink`, `MarkdownBadge`, `RouteCandidate` の互換を保つ。
- 将来の parser 差し替えに備え、span type を core 側に置く。
- scanner の目的は精密 evidence span であり、Markdown 完全準拠 parser を作ることではない。

#### Q8: Patch Planner Expansion

- operation kind は route追加、boundary note追加、docs逃がし、skeleton draft を区別する。
- `Safe` は既存 target へのroute追加などに限定する。
- `Guarded` は skeleton draft と wording変更を含めるが、review required にする。
- `Manual` は policy、legal、security SLA、ownership、contact、publication decision を含める。
- `UnsafeToInvent` は必ず blocked item になる。

#### Q9: Local-only Calibration Guard

- `CalibrationSourceVisibility` を追加する。
- local-only source は JSON/Markdown/Codex public output で `redacted` として出る。
- source count や reviewed status は出せても、local path と本文は出さない。
- test fixture は synthetic data のみ使う。

#### Q10: Codex Adapter v3

- Codex context に `claims`, `wording_lint`, `route_meanings` の digest を追加する。
- Codex action はすべて non-mutating command のまま。
- PR draft body は過剰主張をしない。
- GitHub API 呼び出しや branch 作成は実装しない。

#### Q11: Regression Suite

- lifecycle route fixture。
- verified route does not imply guarantee fixture。
- wording linter positive / negative fixture。
- local-only calibration redaction fixture。
- Codex context no GitHub mutation fixture。
- existing RepoSeiri self-audit smoke fixture。

### 6. 検証コマンド

```powershell
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run --quiet -p seiri-cli -- audit --path . --profile library --format markdown
cargo run --quiet -p seiri-cli -- codex --path . --profile library --format markdown
```

Q6 以降:

```powershell
cargo run --quiet -p seiri-cli -- lint-wording --path . --format markdown
cargo run --quiet -p seiri-cli -- lint-wording --path . --format json
```

Q9 以降:

```powershell
cargo test --test privacy_guard
```

local-only の token を手元で追加検査する場合:

```powershell
$env:REPOSEIRI_PUBLIC_BOUNDARY_TOKENS='<token-1>;<token-2>'
cargo test --test privacy_guard
Remove-Item Env:\REPOSEIRI_PUBLIC_BOUNDARY_TOKENS
```

上の検査は、実際の private source の本文や path を repo に入れるためではなく、漏洩防止の smoke check として使う。公開してよい一般語だけが残っていることを確認する。

### 7. 非目標

- GitHub repository visibility の変更。
- commit、push、merge、PR 作成。
- policy、license、security SLA、ownership、legal 判断の自動決定。
- 外部人気、trust、security、quality が確立済みだとする主張。
- private analysis data の公開保存。
- Markdown 完全準拠 parser の実装。
- unsafe code による性能最適化。

### 8. 実装順序

```text
Q0 -> Q1 -> Q2 -> Q3 -> Q4 -> Q5 -> Q6 -> Q7 -> Q8 -> Q9 -> Q10 -> Q11
```

最初の実装単位は `Q0 + Q1 + Q2` までを一塊にしてよい。ここは型と意味境界だけなので、後続の renderer、linter、planner を壊さずに進められる。

`Q3` は match 更新が広いため単独 block にする。`Q4 + Q5` は claim 生成と report 表示を同時に行う。`Q6 + Q7` は span 精度が関係するため、Q6 を最小 line/column 版で先に入れ、Q7 で byte span を拡張してもよい。

### 9. Evidence Kernel v3 固定ロードマップ

Q0 から Q11 で claim boundary と公開面の回帰を整えた後は、観測、推定、route 判定、patch proposal をより小さな型付き kernel に分ける。順序は次で固定する。

| Block | Scope | Completion criteria |
| --- | --- | --- |
| Q12: Semantic Firewall | 集計推定値、calibration visibility、`Verified` の意味境界を修正する。 | 固定集計値が observation 名で serialize されない。visibility 省略時は `LocalOnly`。`Verified` は存在確認済みの repository-local target と対応する構造 evidence に限定する。 |
| Q13: Compact Evidence Kernel | 重複する evidence / span 表現を typed id と compact fact に統合する。 | legacy evidence 生成経路を compatibility view の外へ出し、deterministic id、span、fact の invariant test を持つ。 |
| Q14: RouteAssessment v3 | route の存在、README routing、target reachability、conflict、freshness を直交成分にする。 | 単一 enum へ早期 collapse せず、旧 `RouteState` は deterministic projection として生成する。 |
| Q15: Scanner / Document Events | bounded walker と document event scanner を分離する。 | repository root、ignore、上限、UTF-8 span、Markdown event の失敗境界が明示される。 |
| Q16: Pattern / Profile v3 | detector、boundary、profile、fixture を registry 単位で分離する。 | 全 route group に detector と negative fixture があり、profile score は evidence と calibration estimate を混同しない。 |
| Q17: Streaming Calibration | 大規模入力を全件保持せず集計する。 | record 全体と per-pattern repository set を保持しない経路を持ち、deterministic replay と resource trace で検証する。 |
| Q18: Patch Proposal IR | text edit を base digest、encoding、EOL、span、unresolved slot と結びつける。 | overlap、stale base、unknown encoding、未確定 policy content を適用前に reject または hold する。 |
| Q19: Renderer / Codex v4 | native typed view と互換 view を分離する。 | v1 compatibility view と v2 native view、argv-safe command、query view、linter context が同じ kernel から生成される。 |

#### Q12 完了条件

- 静的な集計 calibration 値は `AggregateRepositoryEstimate` または同等の型を通り、repository scan の observation と同じ field 名を使わない。
- 旧 observation 名は読み込み互換の alias に限定し、native JSON / Markdown へは出さない。
- `CalibrationSourceVisibility::default()` は `LocalOnly` である。provenance を明示しない JSONL wrapper と inferred source も fail-closed にする。
- `ReadmeRouteTargetStatus::LocalPresent` のみが README route を `Verified` にできる。external、mail、anchor、unknown は `Routed`、local missing は `Stale` とする。
- aggregate route の `Verified` は、route-specific な root structure と repository-local README target の一致を要求する。identity は root README 自体を repository-local structure として扱う。
- JSON、Markdown、Codex context の boundary wording は estimate と observation を区別する。
- regression test は estimate field、visibility omission、external/local/missing target、aggregate route projection を覆う。
- `cargo fmt --all --check`、`cargo test --workspace`、`cargo clippy --workspace --all-targets -- -D warnings`、RepoSeiri self-audit を通す。

#### 実装境界

- Q12 では compact binary、arena、unsafe、parallel scanner を導入しない。それらは invariant と測定経路を準備してから Q13 以降で判断する。
- 推定値の由来を observation に昇格しない一方、既存の固定値を新しい分析結果として再解釈しない。
- schema 互換の最終整理は Q19 で行う。Q12 では旧 field 名を deserialize alias に留め、誤った native serialization を継続しない。
- この固定も GitHub action を許可しない。実装、検証、commit、push、merge はそれぞれ明示された権限境界に従う。

#### Q13 実装記録

- `EvidenceId` は内部では `NonZeroU32` のtyped idとして保持し、JSON互換面では従来の`evrec-NNNN`文字列としてserializeする。
- `EvidenceDraft`だけがID未確定状態を表し、`EvidenceKernel::from_drafts`がstorage orderに沿って連続IDを割り当てる。
- `EvidenceFact`をcanonicalなevidence表現とし、kind、route、scope、confidence、typed origin、正確な`SourceSpan`を一度だけ保持する。
- `SourceSpan`は1-based line/columnと順序付きbyte rangeをdeserialize時にも検証する。
- route state、pattern、profile、claimの判断は`EvidenceKernel`を読む。旧`Evidence`と`EvidenceRecord`はkernelから生成されるcompatibility viewであり、canonical判断への入力に戻さない。
- 旧`EvidenceSource.detail`からlineを再parseする経路を削除し、Markdown scannerが生成したbyte spanをfactへ直接渡す。
- kernel deserializeはfact IDがstorage orderに対して非連続または並べ替え済みなら拒否する。
- persistent binary、arena、SoA、parallel scan、unsafeはQ13の要件ではないため導入しない。将来の測定とABI要件が出るまで保留する。

#### Q14 実装記録

- `RouteAssessment`をrepository route判断のcanonical表現とし、旧`RouteState`を内部fieldとして保持しない。
- 構造存在は`RoutePresenceAssessment`、README導線は`ReadmeRoutingAssessment`、target到達性は`TargetReachabilityAssessment`、競合は`RouteConflictAssessment`、鮮度は`RouteFreshness`として直交させる。
- README route mapはraw candidate/target observationから`ReadmeRouteAssessment`を先に生成し、既存の`state`と`reason`をdeterministic compatibility projectionとして生成する。
- repository集約はroot structure、README routing、inherited evidenceを分離した`RouteEvidenceGroups`からassessmentを生成し、`route_states`をそのcompatibility viewとして出力する。
- target到達性はrepository-local present/missing、external、anchor、mail、unknownを別々に保持する。`Verified`への旧projectionはQ12と同様にrepository-local presentと対応するroot structureを要求する。
- 旧JSONにassessmentがなければcompatibility observationから再構成する。新形式でfreshness、state、compatibility count、evidence group、policyが矛盾する入力はdeserialize時に拒否する。
- downstreamのclaim、priority、Codex viewは当面deterministicな`route_states` compatibility viewを読める。native/compatibility rendererの完全分離はQ19が担当する。
- route数は小さくcold pathであるため、unsafe、bit packing、parallel projection、性能主張は追加しない。

#### Q15 実装記録

- `seiri-fs`は`RepositoryRoot`と`RepositoryWalk`を持つwalkerをimportant-file分類器から分離する。`RepoFsScan`は分類済みcompatibility viewと`RepositoryWalkSummary`を保持する。
- walkerの既定上限はdepth `32`、entry `100000`とする。`IgnorePolicy`が既定ignore名と追加ignore名をbasename単位で適用し、symlinkは記録するが追跡しない。
- Q15時点ではdepthまたはentry上限で`FsError::LimitExceeded`を返していた。Block Sでこれはtyped partial resultへ置き換え、部分走査はabsenceを生成しないcoverageへ接続した。
- Markdown scannerの既定上限はsource `2 MiB`、event `65536`、diagnostic `1024`とする。filesystem入力はbyte列として読み、UTF-8変換失敗を`InvalidUtf8`としてI/O失敗と分離する。
- `DocumentEvent`はheading、link、badge、route candidateをexact `SourceSpan`付きでsource順に保持する。`DocumentScan`は本文を保持せず、path、source byte数、event、diagnosticだけを保持する。
- event span欠落、source範囲外span、eventまたはdiagnosticの非決定的順序はconstructionとdeserializeで拒否する。
- 未閉鎖link label/targetはscan全体のhard failureへ昇格せず、byte span付き`DocumentDiagnostic`として保持する。source/event/diagnostic上限超過はhard failureにする。
- `ReadmeSummary`とREADME route mapは`DocumentScan`から生成するcompatibility viewとする。`EvidenceKernel`はsummaryを経由せず`DocumentEvent`をcanonical入力として読む。
- Q15時点ではauditへ接続するdocumentをroot README一件に限定していた。Block Sでbounded `DocumentIndex`へ拡張したが、CommonMark完全準拠や性能改善は主張しない。

#### Q16 実装記録

- pattern detector、欠落判断boundary、negative fixture、registryを独立moduleへ分離し、組み込みregistryは全13 `PatternGroup`のdetectorとfixture coverageをconstruction時に検証する。
- negative fixtureはmetadataだけで完了扱いにせず、各fixtureを実際にauditして紐づくdetectorがevidenceを返さないことを回帰で確認する。
- profile definitionをregistryへ移し、weightは`NonZeroU16`由来の`StaticProfileWeight`を通す。score入力はevidence-backed baseline statusとstatic registry weightに限定する。
- calibration suggestionはprofile scoreを変更しない。evidence idを伴わない`Present`もscoreでは`Missing`へ降格する。

#### Q17 実装記録

- `.jsonl` calibrationにbounded streaming経路を追加し、非空行を一件ずつdecodeして集計する。全`BenchmarkRepoRecord`、repository id、per-pattern repository setは保持しない。
- 既知patternは`PatternSlot(u16)`へ変換し、supportはchecked counter、co-occurrenceはregistry sizeで固定した行列へ集計する。重複排除用のpattern、route、pending keyは一record内だけの一時bufferとする。
- line byte、record内pattern数、pending pattern cardinality、metadata source cardinalityに非0上限を置き、超過、UTF-8不正、counter overflowをtyped errorとしてfail-closedにする。
- streaming identityは「非空JSONL一行が一repository record」である。global repository-id uniquenessは入力準備側の責務と明記し、暗黙の近似dedupeは行わない。
- `CalibrationResourceTrace`はretained record、repository-id entry、per-pattern set、aggregate slot、peak line/pattern、replay digestを記録する。これは構造diagnosticであり、測定memory、throughput、性能保証ではない。
- replay digestは入力順序を含むdeterministic FNV-1a diagnosticであり、暗号学的digestではない。local-only sourceをpublic出力へredactするときはdigestも除去する。
- materialized JSON dataset APIは互換経路として残し、report/CLIは`.jsonl`だけをstreamingへrouteする。unsafe、parallelism、persistent binary、性能向上claimはQ17へ導入しない。

#### Q18 実装記録

- `PatchProposal`は各`PatchTextEdit`をscanner由来の`TextDocumentBase`へ結びつける。baseはexact byte length、UTF-8/UTF-8 BOM/unknown encoding、LF/CRLF/none/mixed EOL、終端改行状態、deterministic FNV-1a digestを保持し、本文自体は保持しない。
- core実装はschema/modelを`patch_proposal.rs`、raw byte metadataを`patch_proposal/base.rs`、preflightと純粋apply kernelを`patch_proposal/engine.rs`へ分け、公開型と実行境界を分離する。
- byte spanは`start <= end`をdeserialize時にも検証する。構造preflightはout-of-bounds、overlap、同一位置への複数insert、output length overflow、replacement EOL不一致を`Reject`へ送る。
- apply前には現在bytesからbase metadataを再計算し、stale digest、encoding/EOL/終端改行差異、UTF-8 code pointを分断するspanを`Reject`する。policy、security、support、lifecycleなどの未確定内容はtyped `UnresolvedPolicySlot`として`Hold`し、literalへ暗黙変換しない。
- `apply_to_bytes`はReady proposalだけを新しいowned byte bufferへ純粋適用する。filesystem write、branch、commit、push、PR、policy選択は行わず、plannerは引き続きdry-runでapplicationも呼び出さない。
- Safe Patch Planner v3のREADME docs routeはproposalを必須とし、scanner pathとbase metadataが一致し、structural decisionがReadyの場合だけoperationへ昇格する。JSON/Markdown reportはdigest、encoding、EOL、span、content kind、decisionを表示する。
- FNV-1a digestはdeterministic stale-base guardであり、暗号学的integrity、security、quality、trust保証ではない。Q18はsafe Rustだけで実装し、未知encodingや曖昧EOLを推測しない。

#### Q19 実装記録

- `CodexReviewKernel`を唯一のCodex view生成境界とし、同じ`RepoSnapshot`、`PatchPlan`、`WordingLintReport`からv1 compatibility、native v2、query、linter contextを投影する。rendererがrepository判断を再実装しない。
- schema/argv invariantは`seiri-core/codex_view`、kernel projectionは`seiri-codex/v4.rs`、actions・linter・summary・rendererはv4 sibling module、CLI dispatchは`seiri-cli/codex.rs`へ分け、dependency境界ごとに検証できる構成にする。
- 既存`CodexReviewContext`と既定CLI出力はv1 compatibility viewとして維持する。既存schema、PR draft、flat digest、PowerShell表示commandを保ち、旧consumerのdefault commandを変更しない。
- native v2は`seiri.codex.native.v2`を使用し、canonical `DocumentScan`、`EvidenceKernel`、`RouteAssessment`、`ContentClaim`、Q18 `PatchPlan`、full linter contextを出力する。`readme`、legacy evidence/ledger、`route_states`、flat action/PR fieldsはnative rootへ混入させない。
- native summaryはlegacy count名を再利用せず、canonical evidence fact、route assessment、claimと直交route componentの件数だけを保持する。
- commandは`CodexCommand { program, args }`として保持し、empty programとNULをdeserialize時にも拒否する。repository pathは一つのargv要素のまま保持し、shell文字列はv1 compatibility projectionでだけsingle-quote escapingにより生成する。kernelはcommandを実行しない。
- queryは`summary`、`routes`、`patches`、`linter`、`actions`のtyped enumとし、`seiri.codex.query.v2` viewを同じkernelから必要部分だけmaterializeする。linter contextはdigestではなくfull finding、rule、boundaryを保持する。
- CLIは既定`--schema compatibility-v1 --view context`を保ち、`--schema native-v2`、`--view query --query <kind>`、`--view linter`を追加する。PR bodyは引き続きv1 presentation viewである。
- Q19はsafe Rustのread-only projectionであり、argv実行、filesystem mutation、GitHub API、branch、commit、push、merge、policy adoption、品質・信頼性保証を導入しない。

### 10. Evidence-Closed Route Engine Roadmap v3

Q12-Q19のtyped kernelを維持したまま、次段階では「経路があるか」だけでなく、「どの範囲を完全に走査できたか」「経路内部のどの内容を観測したか」「どのgapがroute/content/consistency/unknownのどれか」を分離する。このroadmapをQ20-Q34として固定する。

#### 批判から固定する修正

- `Verified` routeにcandidate pattern不足がある場合、それを`missing route`と呼ばない。route gapとcontent gapを別型にする。
- nested/fixture evidenceをorganization inheritanceへ暗黙投影しない。`Inherited`は明示的なorganization sourceだけに限定し、licenseは常にrepository-local decisionとする。
- scan limit、invalid UTF-8、parse failure、unsupported syntax、permission failureを`Missing`へ変換しない。完全走査を示すcoverageだけがabsenceを生成できる。
- aggregate calibration estimateはrepository observationと分離したまま保持し、品質、信頼、安全性、人気、保守状態の判定へ昇格させない。
- safe Rust、typed id、bounded allocation、deterministic orderを優先する。unsafe、SIMD、独自binary、parallelismは測定根拠が得られるまで導入しない。

#### 固定ロードマップ

| Phase | Scope | Completion criteria |
| --- | --- | --- |
| Q20: Gap Taxonomy | `RouteGap`、`ContentGap`、`ConsistencyGap`、`ObservationUnknown`を分離する。 | `Verified` routeのcontent不足がtop missing routeにならず、legacy summaryのtop routeはactual route gapだけを指す。 |
| Q21: Evidence Kernel v2 | typed `EvidenceAtom`、`EvidenceProvenance`、`DocumentId`、`SourceDomain`を追加する。 | pathをdocument tableへ一度だけ保持し、native v1/v2 wireへ新fieldを漏らさない。 |
| Q22: Coverage Algebra | `CoverageIndex`と`Observation<T>`を追加する。 | evidenceなしのPresent/Conflictを拒否し、complete coverageだけがAbsentを生成する。 |
| Q23: Partial Repository Walk | limit超過時もbounded partial recordsとtyped truncation reasonを返す。 | partial walkがabsenceを生成せず、symlink非追跡とdeterministic orderを維持する。 |
| Q24: DocumentIndex v2 | document role、digest、encoding、byte budget、scan statusを索引化する。 | README、docs、policy、GitHub config候補を一意のdocument idで参照できる。 |
| Q25: Multi-document Markdown | README以外の主要Markdownをbounded scanする。 | byte spanとdiagnosticを保ち、parser failureをUnknownへ伝播する。 |
| Q26: Predicate Program | typed atomを読むbounded postfix evaluatorを追加する。 | invalid stack、過剰atom、invalid arityをregistry load時に拒否し、test-only reference evaluatorと一致する。 |
| Q27: Route Content Engine | 14 routeのcontent atomを四状態で評価する。 | content presenceとcontent adequacyを分離し、正しさや十分性を断定しない。 |
| Q28: Structured GitHub Parsers | issue form、workflow、dependency bot、CODEOWNERSをtyped IRへ変換する。 | parser byte/node/depth/scalar budgetとspan-aware diagnosticを持ち、設定を実行しない。 |
| Q29: Profile Facets | package、binary、infra、docs、research、templateなどのfacetを併存可能にする。 | single winnerをrepository type assertionとして扱わず、facet evidenceを保持する。 |
| Q30: Obligation / Conflict Graph | facet条件付きobligationと文書間conflictを構築する。 | obligation理由とconflictの両側がEvidenceIdへ結びつく。 |
| Q31: Remote Evidence | opt-in read-only adapterを別crateへ隔離する。 | not requested、denied、not found、rate limited、unavailableを区別し、tokenをserializeしない。 |
| Q32: Pattern / Calibration v4 | pattern packとconditional denominatorを追加する。 | positive、negative、ambiguous、partial、malformed fixtureとregistry fingerprintを必須化する。 |
| Q33: Codex Native v3 | borrowed/query-first viewを追加する。 | compatibility-v1/native-v2 goldenを維持し、巨大collectionの不要cloneを避ける。 |
| Q34: Planner / Regression v4 | patchをanalysis run、base digest、anchor contextへ束縛する。 | policy-sensitive contentはmanual/holdのまま、stale analysisからpatchを生成しない。 |

#### Implementation Blocks

| Block | Phases | Boundary |
| --- | --- | --- |
| R | Q20-Q22 | Meaning repair、typed evidence、coverageだけを実装する。 |
| S | Q23-Q27 | Local filesystem、document、predicate、content evaluationを実装する。 |
| T | Q28 | GitHub local structured parserを実装する。 |
| U | Q29-Q30 | Facet、conditional obligation、conflict graphを実装する。 |
| V | Q31-Q32 | Optional remote、pattern pack、privacy-safe calibrationを実装する。 |
| W | Q33-Q34 | Native v3、patch planner、全体regressionを実装する。 |

#### Block R 実装記録

- `ReviewGap`はroute、content、consistency、observation unknownをtagged enumとして分離する。legacy `MissingRoutePriorityReport`は互換用に残すが、`top_route`と`top_priority_x100`はactual route gapだけから選ぶ。
- `RepoSnapshot`は`ReviewPriorityReport`をcanonical internal fieldとして保持し、Q33までlegacy/native v2 wireにはserializeしない。
- `EvidenceKernelV2`は既存kernelからdeterministically構築し、document pathをsorted document tableへ一度だけ保持する。factはtyped `EvidenceAtom`、source domain、producer、document id、`u32` byte spanを持つ。
- `Root`、`Nested`、`Generated`はrepository-local domainであり、`Fixture`はfixture domainである。organization inheritanceは明示sourceがない限り生成しない。
- route aggregationはv2 provenanceの`OrganizationInherited`だけをinherited evidenceへ入れ、licenseでは常に空にする。
- `CoverageIndex`はscope重複を拒否し、non-zero contiguous idを割り当てる。`observe_absence`はCompleteでだけAbsentを返し、partial/not-requestedはtyped Unknownを返す。
- `Observation::Present`と`Observation::Conflict`はnon-empty、sorted、deduplicated `EvidenceSet`を要求する。
- v2 kernel、coverage、review priorityはcompatibility wireから除外し、Q12-Q19 consumerを壊さない。
- Block Sはfilesystem partial result、predicate VM、multi-document parser、route content observationを追加した。remote sourceとnative v3は後続blockのscopeである。

#### Block S 実装記録

- walkerはdirectory entryをpath順に処理し、entry/depth上限では`WalkCompletion::Truncated(WalkTruncation)`付きの部分結果を返す。symlinkは引き続き記録のみで追跡せず、部分repository coverageは`Unknown(LimitExceeded)`を生むためabsenceを確定しない。
- `DocumentIndex`はrole、宣言byte数、scan status、FNV-1a base digest、encoding、後段v2 document idをpath順に保持する。README、docs、policy Markdown、GitHub configuration候補をindexし、configurationは実行・構文解釈せずbounded raw-byte baseだけを取得する。
- Markdown scanはdocument数と総source byte budgetを持つ。UTF-8、I/O、source/event/diagnostic failureは監査全体のhard failureではなくdocument statusとtyped partial coverageへ局所化する。README compatibility viewはroot READMEだけから生成する。
- multi-document eventはcanonical evidenceへ取り込むが、README route assessmentと`ReadmeRoute` detectorはroot README pathだけを読む。root policy documentがREADME導線へ昇格することはない。
- `PredicateProgram`は最大atom数、operation数、stack depthを検証するtyped postfix VMである。invalid atom、arity、threshold、stack、final depthをconstruction/registry validationで拒否し、test-only reference evaluatorと照合する。
- `RouteContentAtom`は14 routeに各2 atomを置き、`Present`、`Absent`、`Unknown`、`Conflict`を保持する。これはobserved markerの存在だけを表し、contentの正しさ、十分性、品質、support/security policyの妥当性を断定しない。
- `DocumentIndex`とroute contentは現時点ではinternal snapshot stateであり、compatibility JSON/native v2 wireへserializeしない。Q28 structured parserはBlock Tで追加し、remote sourceとnative v3は未実装である。

#### Block T 実装記録

- `seiri-github-local`をlocal-only crateとして追加し、issue form、GitHub Actions workflow、Dependabot/Renovate、CODEOWNERSだけをread-only typed IRへ変換する。parserはconfigurationを実行せず、network、GitHub API、workflow action、dependency updateを呼び出さない。
- parserはsource byte、node、indent depth、scalar byte、diagnostic数の上限を持つ。超過は`StructuredBudgetKind`付きstatusとspan-aware diagnosticになり、audit全体を失敗させずGitHub configuration coverageをpartial/Unknownへ落とす。
- YAML経路はmapping/list indentationの制限付きsubsetだけを受理する。anchor、alias、block scalar、tab indentation、構造外の行は`UnsupportedSyntax`へ分類する。Renovate JSONはsource budget後にread-only decodeし、node/depth/scalarを別途検証する。
- 各documentはBlock Sの`DocumentIndex`から得たdocument id、path、kind、status、diagnostics、optional IRを保持する。CODEOWNERSはpatternとowner tokenを行単位で記録し、ownerなしの行はexact byte span付き`MissingCodeowner`になる。
- parsed statusだけがGitHub configuration/document coverageをCompleteにする。UTF-8、permission、malformed、unsupported syntax、budget超過はtyped incomplete coverageであり、設定の有効性、workflow成功、security、ownership、dependency更新の正しさを断定しない。
- IRとdiagnosticはinternal snapshot stateであり、compatibility JSON/native v2 wireへserializeしない。Q29 facet、Q30 conflict graph、remote source、native v3は後続blockのscopeである。

#### Block U 実装記録

- `RepositoryFacet`はpackage、binary、infrastructure、documentation、research、template、productを同時に保持するfixed complete setであり、単一profile branchの勝者やrepository type assertionを置き換えない。各`FacetAssessment`は`EvidenceId`付き`Observation<()>`を保持し、evidenceなしはrepository filesystem coverageがCompleteの場合だけAbsent、それ以外はUnknownになる。
- facet signalは既存の重要file、Markdown evidence、manifest、entrypoint、限定したdirectory markerからdeterministically得る。facet用途のfile markerもcanonical `EvidenceKernel` factとして記録するため、facet理由をpath stringだけに落とさない。
- `ConditionalObligation`はobserved facetだけから生成し、facet evidenceをnon-empty `EvidenceSet`として理由に保持する。package/researchのdocs・quickstart、binaryのquickstart・release、infrastructureのsecurity・automation、documentation/productのdocs・support、templateのquickstart・contributingを条件付きroute expectationとして評価する。これは法律、security、quality、policyの保証や自動patch指示ではない。
- obligation routeのevidenceがなければ`CoverageScope::RepositoryFiles`がCompleteのときだけAbsentとなる。partial/not-requested coverageではUnknownとなるため、bounded walkやparser failureからmissing obligationを断定しない。
- document consistencyは異なるdocumentの同route local targetが異なる場合だけ`DocumentConflict`を発行する。双方は`DocumentId`と`EvidenceId`を持ち、target group、document、evidence orderはdeterministicである。graphはtarget group 128件、conflict pair 64件でboundedにし、上限時は`conflict_coverage`をpartialに落とす。これは競合候補の観測であり、文書全体の矛盾、不正確さ、完全性を断定しない。
- facetとdocument consistency reportはinternal snapshot stateのままとし、compatibility JSON/native v2 wireにはserializeしない。remote evidence、facet calibration、native v3は後続blockのscopeである。

#### Block V 実装記録

- `seiri-remote`は`seiri-core`だけに依存するopt-in read-only adapter crateである。通常のauditはtransportを持たずnetworkを開始しない。remote requestは明示的に注入されたtransportだけを通り、GitHub mutation、workflow実行、cache書込み、repository変更を行わない。
- remote resultは`NotRequested`、`Denied`、`NotFound`、`RateLimited`、`Unavailable`、`Observed`をtyped statusとして保持する。`NotFound`だけはcomplete read resultであり、denied/rate-limited/unavailableはtyped partial coverageになる。remote reportはinternal snapshot stateであり、compatibility JSON/native v2 wireにはserializeしない。
- authorizationは`RemoteReadAuthorization`というnon-serializable opaque typeであり、Debug表示もredactedである。core remote IR、request report、snapshot、error、markdown/JSON rendererにはtoken fieldを置かない。responseはsource byte上限後に必要なGitHub metadataだけをread-only decodeする。
- `PatternPack`はregistry、condition、positive/negative/ambiguous/partial/malformed fixture、FNV-1a registry fingerprintを一体として検証する。built-in common packは各pattern groupに全fixture kindを要求し、fixture id、group、pattern対応の不整合をload時に拒否する。
- calibrationは`calibrate_dataset_with_pattern_pack`でpack conditionに一致したrecordだけを集計し、eligible/excluded record数、condition、fingerprintを`CalibrationPatternPack`としてrunに保持する。profile packはprofile hintをconditional denominatorに使う。streaming JSONLはcommon all-record pack metadataを出力する。
- calibration inputはin-memory/local pathのままであり、public JSON/Markdownは既存のlocal-only source redactionを通す。pack metadataはsource path、body、token、private analysis値を含まない。calibration suggestionは引き続きpending reviewであり、ruleやweightを自動採用しない。

---

#### Block W 実装記録

- `CodexNativeV3View<'a>` は snapshot、patch plan、wording report を借用し、v1/v2 の所有型 `CodexReviewKernel` とは別に query-first の v3 view を構成する。大きな route、evidence、document、governance collection は clone せず slice/reference として返す。v3 は summary、routes、evidence、documents、governance、patches、linter、actions、remote を個別 query として公開する。
- Native v3 の patch query だけが内部の `analysis_run` と operation binding を明示的に serialize する。`PatchPlan` と `PatchPlanOperation` の binding field は legacy serde から skip するため、compatibility-v1 と native-v2 の wire output に新しい内部 field は混入しない。Codex v1/v2 の report kernel は `safe_patch_planner.v3` compatibility projection を明示的に使用する。
- `safe_patch_planner.v4` は Safe proposal の生成直前に repository root 内の現在の README bytes を再読込し、scanner-owned `TextDocumentBase` と一致しない場合は proposal を返さず blocked item にする。root から逃げる path、symlink 解決後に root 外へ出る path、非 regular file は拒否する。
- `PatchAnalysisRun` は snapshot metadata の deterministic FNV-1a digest で識別される。`PatchProposalBinding` は analysis run、proposal id/path、base digest、各 edit の最大 96 bytes 前後 context digest を保持するが、source text 自体は保持しない。apply 前の bound preflight は proposal/base/anchor の不一致を Reject にする。
- policy-sensitive candidate は既存の Guarded/Manual/UnresolvedPolicySlot boundary に留まり、v4 binding を得ない。v4 も dry-run のままで、filesystem write、patch apply、GitHub action、branch、commit、push、merge、policy decision を実行しない。
- Q33/Q34 regression は borrowed route slice の pointer identity、v1/v2 wire への binding 非混入、v3 binding serialization、bound in-memory apply、analysis 後に変更された README からの preview 拒否、policy-sensitive item の unbound 状態を確認する。

## English

### 1. Fixed Purpose

This roadmap moves RepoSeiri from a repository organization CLI toward a lower-level Rust review engine that separates evidence, route state, claim boundaries, wording, and patch safety through typed structures.

RepoSeiri already keeps file scanning, Markdown route scanning, the pattern registry, profiles, the safe patch planner, and the Codex adapter close to the Rust core. The remaining weakness is that claim boundaries, report wording, route meanings, and some patch operations are still string-centered. The next implementation should make public output come from typed evidence and typed claims instead of free-form text.

### 2. Decisions Fixed From Critique

| Risk | Design response |
| --- | --- |
| Scores or route states can read like evidence of quality. | Add `ContentClaim` and `ClaimBoundaryKind` so allowed claims and blocked claims are separated by type. |
| `reason: String` and `claim_boundary: String` keep growing. | Preserve existing JSON compatibility, but bind major report sentences to claim ids and evidence ids. |
| `Verified` can be misread as proof of security or quality. | Register what each route state indicates and does not indicate through `RouteMeaningRule`. |
| Analysis input can be confused with runtime rules. | Treat calibration input as reviewable source material and never auto-adopt it. |
| Local/private analysis can leak into public output. | Add source visibility and redaction guards so paths, body text, and private details do not appear in public output. |
| Low-level work can become a breaking rewrite. | Keep existing schemas and add fields and new commands incrementally. |

### 3. Execution Boundary

- This document fixes implementation order only. It does not authorize GitHub actions, branch creation, commits, pushes, merges, or pull requests.
- Even after implementation blocks are complete, GitHub actions require explicit instruction.
- Local/private calibration input is not stored in the public repository.
- Public docs must not contain private analysis body text, paths, specific filenames, or unpublished detailed figures.
- RepoSeiri output is a review aid. It does not guarantee popularity, trust, safety, quality, legal fitness, or publication readiness.

### 4. Implementation Blocks

| Block | Scope | Files likely touched | Completion criteria |
| --- | --- | --- | --- |
| Q0: Authority / Privacy Guard | Fix pre-implementation leak prevention and work boundaries. | docs, tests only if needed | No private source path, filename, or body text is present in tracked files. `git status` shows local edits only. |
| Q1: Typed Claim Core | Add `ClaimId`, `ContentClaim`, `ClaimStrength`, `ClaimBoundaryKind`, and `MeaningAtom` as core types. | `crates/seiri-core/src/lib.rs` | Types implement `Serialize`, `Deserialize`, `Clone`, and `Eq`. Existing `ClaimBoundary` compatibility remains. cargo test passes. |
| Q2: RouteMeaning Registry | Add a static registry for meaning and non-claim boundaries per route/state. | `crates/seiri-core`, `crates/seiri-report` | Every `RouteKind` and major `RouteState` has a meaning rule. Tests keep `Verified` from being promoted to evidence of quality or safety. |
| Q3: Lifecycle Route | Add `RouteKind::Lifecycle` for maintenance, deprecation, and support lifecycle separate from release. | core, fs, markdown, report, patterns, planner | All matches are exhaustive. Pattern group `LIF` and the route align. audit/codex output remains stable. |
| Q4: ContentClaim Builder | Generate evidence-linked claims from snapshots. | `crates/seiri-report`, optional new module | Every claim has evidence ids. Claims without evidence are not generated. JSON and Markdown expose them. |
| Q5: Claim-bound Renderer | Move major report and Codex wording toward claim-derived output. | `crates/seiri-report`, `crates/seiri-codex` | Route summaries, boundaries, and priority rationale reference claim ids or boundary kinds while preserving current behavior. |
| Q6: Wording Linter | Detect overclaims with byte spans. | new crate or module, CLI, report | `seiri lint-wording --path . --format markdown|json` works. Tests cover banned terms and typed boundary exceptions. |
| Q7: Byte-span Markdown Scanner | Add byte range / line / column to Markdown headings, links, badges, and candidates. | `crates/seiri-markdown`, core span types | Existing line-based output remains compatible, and span-aware tokens are available. Multibyte fixtures pass. |
| Q8: Patch Planner Expansion | Expand patch operations around routes and claim boundaries. | `crates/seiri-planner`, core, report | `AddClaimBoundaryNote`, `AddLifecycleRoute`, `AddSupportSkeletonDraft`, `AddSecuritySkeletonDraft`, and `MoveReadmeDetailToDocsDraft` exist as typed operations and obey preview / guarded / manual boundaries. |
| Q9: Local-only Calibration Guard | Add visibility to calibration sources. | core, calibration, report | `Public`, `LocalOnly`, and `Redacted` are distinct. Local-only sources are redacted from public report and Codex context. |
| Q10: Codex Adapter v3 | Pass claim summary, wording lint summary, and route meaning digest into Codex context. | `crates/seiri-codex`, report | Codex context remains a safe review artifact and does not create branches, PRs, or GitHub API calls. |
| Q11: Regression Suite | Fix regressions for claims, route meanings, lifecycle, wording, and privacy guard. | fixtures, crate tests | `cargo test --workspace` runs the regressions. Private leak guard checks tracked text. |

### 5. Detailed Completion Criteria By Block

#### Q0: Authority / Privacy Guard

- `rg` finds no private source path, filename, or body fragment in the repository.
- Roadmap docs contain only abstracted design input.
- No GitHub action is performed.
- This block does not change code behavior.

#### Q1: Typed Claim Core

- `ContentClaim` has `id`, `route`, `state`, `strength`, `evidence_ids`, `allowed_meanings`, and `boundaries`.
- `ClaimStrength` includes at least `Observed`, `Inferred`, `Suggested`, and `Blocked`.
- `ClaimBoundaryKind` represents every listed non-claim category as a typed variant.
- `stable_id` or an equivalent mechanism can create deterministic claim ids.

#### Q2: RouteMeaning Registry

- `RouteMeaningRule` has `route`, `state`, `indicates`, and `does_not_indicate`.
- It covers `Absent`, `Implicit`, `Weak`, `Routed`, `Structured`, `Verified`, `Inherited`, `Conflicting`, `Overloaded`, `Stale`, and `UnsafeToInvent`.
- If `Overridden` remains, its meaning and non-meaning are documented. If it is unnecessary, it becomes a future schema migration target rather than being removed immediately.

#### Q3: Lifecycle Route

- `RouteKind::Lifecycle` is added.
- The route parser recognizes lifecycle, maintenance, deprecation, archival, and supported-version language.
- `route_priority` and `planner` matches are exhaustive.
- `PatternGroup::Lif` and `RouteKind::Lifecycle` refer to the same meaning area.

#### Q4: ContentClaim Builder

- Claims can be generated from route state reports.
- Missing route priority can generate suggestion-strength claims.
- Calibration-derived claims never become automatic adoption.
- Free-form summary text without claims is not used for major decisions.

#### Q5: Claim-bound Renderer

- Markdown reports include a claim summary section.
- JSON reports include a machine-readable claim list.
- Codex context keeps claim boundaries short and moves detail to claim ids.
- Existing `audit`, `plan`, `codex`, `patterns`, and `calibrate` commands keep working.

#### Q6: Wording Linter

- Lint findings contain `path`, `line`, `column`, `byte_start`, `byte_end`, `boundary`, and `replacement_hint`.
- It works against README, docs, and generated reports.
- Required boundary language is handled through typed exceptions rather than a broad allowlist.
- The linter detects overclaim wording but does not make legal judgments or security diagnoses.

#### Q7: Byte-span Markdown Scanner

- The scanner preserves byte indices and keeps line/column correct for UTF-8 text.
- Existing `MarkdownHeading`, `MarkdownLink`, `MarkdownBadge`, and `RouteCandidate` compatibility remains.
- Span types live in core so future parser replacement stays possible.
- The goal is precise evidence spans, not a fully compliant Markdown parser.

#### Q8: Patch Planner Expansion

- Operation kinds distinguish route additions, boundary notes, docs relocation, and skeleton drafts.
- `Safe` is limited to existing-target route additions and similarly constrained changes.
- `Guarded` can include skeleton drafts and wording changes, but always requires review.
- `Manual` covers policy, legal, security SLA, ownership, contact, and publication decisions.
- `UnsafeToInvent` always becomes a blocked item.

#### Q9: Local-only Calibration Guard

- `CalibrationSourceVisibility` is added.
- Local-only sources render as `redacted` in JSON, Markdown, and Codex public output.
- Source counts and review status may be shown, but local paths and body text are not shown.
- Test fixtures use synthetic data only.

#### Q10: Codex Adapter v3

- Codex context includes `claims`, `wording_lint`, and `route_meanings` digests.
- Codex actions remain non-mutating commands.
- PR draft body avoids overclaims.
- GitHub API calls and branch creation are not implemented.

#### Q11: Regression Suite

- Lifecycle route fixture.
- Verified route does not imply guarantee fixture.
- Wording linter positive and negative fixtures.
- Local-only calibration redaction fixture.
- Codex context no GitHub mutation fixture.
- Existing RepoSeiri self-audit smoke fixture.

### 6. Verification Commands

```powershell
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run --quiet -p seiri-cli -- audit --path . --profile library --format markdown
cargo run --quiet -p seiri-cli -- codex --path . --profile library --format markdown
```

After Q6:

```powershell
cargo run --quiet -p seiri-cli -- lint-wording --path . --format markdown
cargo run --quiet -p seiri-cli -- lint-wording --path . --format json
```

After Q9:

```powershell
cargo test --test privacy_guard
```

For local-only token checks:

```powershell
$env:REPOSEIRI_PUBLIC_BOUNDARY_TOKENS='<token-1>;<token-2>'
cargo test --test privacy_guard
Remove-Item Env:\REPOSEIRI_PUBLIC_BOUNDARY_TOKENS
```

The check above is a leak-prevention smoke check. It is not a reason to store actual private source body text or paths in the repository. Only public-safe generic terms should remain.

### 7. Non-goals

- Changing GitHub repository visibility.
- Commits, pushes, merges, or pull requests.
- Automated decisions for policy, license, security SLA, ownership, or legal judgment.
- Claims that external popularity, trust, security, or quality are established.
- Public storage of private analysis data.
- A fully compliant Markdown parser.
- Performance optimization through unsafe code.

### 8. Implementation Order

```text
Q0 -> Q1 -> Q2 -> Q3 -> Q4 -> Q5 -> Q6 -> Q7 -> Q8 -> Q9 -> Q10 -> Q11
```

The first implementation unit may combine `Q0 + Q1 + Q2`. These blocks only establish types and meaning boundaries, so they can proceed without disturbing later renderers, linters, and planners.

`Q3` should be its own block because it touches many exhaustive matches. `Q4 + Q5` can be paired because claim generation and report rendering are tightly connected. `Q6 + Q7` both depend on span precision, but Q6 can land first with minimal line/column support and Q7 can later extend it with byte spans.

### 9. Fixed Evidence Kernel v3 Roadmap

After Q0 through Q11 establish claim boundaries and public-surface regressions, the next phase separates observations, estimates, route decisions, and patch proposals into a smaller typed kernel. The order is fixed as follows.

| Block | Scope | Completion criteria |
| --- | --- | --- |
| Q12: Semantic Firewall | Repair the meaning boundaries for aggregate estimates, calibration visibility, and `Verified`. | Fixed aggregate values never serialize under observation names. Omitted visibility becomes `LocalOnly`. `Verified` is limited to an existence-checked repository-local target and matching structural evidence. |
| Q13: Compact Evidence Kernel | Unify duplicate evidence and span representations around typed ids and compact facts. | Legacy evidence generation moves outside the kernel into a compatibility view, with invariant tests for deterministic ids, spans, and facts. |
| Q14: RouteAssessment v3 | Represent route presence, README routing, target reachability, conflicts, and freshness as orthogonal components. | Assessment does not collapse early into one enum; the old `RouteState` is emitted as a deterministic projection. |
| Q15: Scanner / Document Events | Separate the bounded repository walker from document event scanning. | Repository root, ignores, limits, UTF-8 spans, and Markdown event failure boundaries are explicit. |
| Q16: Pattern / Profile v3 | Separate detectors, boundaries, profiles, and fixtures at registry level. | Every route group has a detector and negative fixture, and profile scoring does not mix evidence with calibration estimates. |
| Q17: Streaming Calibration | Aggregate large inputs without retaining all records. | The streaming path retains neither every record nor per-pattern repository sets and is checked through deterministic replay and resource traces. |
| Q18: Patch Proposal IR | Bind text edits to a base digest, encoding, EOL, spans, and unresolved slots. | Overlaps, stale bases, unknown encodings, and unresolved policy content are rejected or held before application. |
| Q19: Renderer / Codex v4 | Separate native typed views from compatibility views. | A v1 compatibility view and v2 native view, argv-safe commands, query views, and linter context are generated from the same kernel. |

#### Q12 Completion Criteria

- Static aggregate calibration values pass through `AggregateRepositoryEstimate` or an equivalent type and do not reuse repository-scan observation field names.
- Legacy observation names are accepted only as deserialization aliases and never appear in native JSON or Markdown.
- `CalibrationSourceVisibility::default()` is `LocalOnly`. JSONL wrappers and inferred sources without explicit provenance also fail closed.
- Only `ReadmeRouteTargetStatus::LocalPresent` can make a README route `Verified`. External, mail, anchor, and unknown targets remain `Routed`; local missing targets become `Stale`.
- Aggregate `Verified` requires agreement between route-specific root structure and a repository-local README target. Identity treats the root README itself as repository-local structure.
- JSON, Markdown, and Codex boundary wording distinguish estimates from observations.
- Regression tests cover estimate fields, omitted visibility, external/local/missing targets, and aggregate route projection.
- `cargo fmt --all --check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and the RepoSeiri self-audit pass.

#### Implementation Boundary

- Q12 does not add compact binary storage, arenas, unsafe code, or a parallel scanner. Those choices wait for Q13 or later, after invariants and measurement paths exist.
- Aggregate estimates are not promoted to observations, and existing fixed values are not reinterpreted as new analysis results.
- Q19 owns the final schema compatibility organization. Q12 keeps old field names as deserialization aliases only instead of continuing incorrect native serialization.
- This fixed roadmap does not authorize GitHub actions. Implementation, verification, commits, pushes, and merges remain separate explicit authority boundaries.

#### Q13 Implementation Record

- `EvidenceId` is stored internally as a typed `NonZeroU32` and serialized as the existing `evrec-NNNN` string on the JSON compatibility surface.
- Only `EvidenceDraft` represents the pre-id state; `EvidenceKernel::from_drafts` assigns contiguous ids from storage order.
- `EvidenceFact` is the canonical evidence representation and stores kind, route, scope, confidence, typed origin, and exact `SourceSpan` once.
- `SourceSpan` validates 1-based line/column and an ordered byte range during deserialization as well as construction.
- Route-state, pattern, profile, and claim decisions read `EvidenceKernel`. Legacy `Evidence` and `EvidenceRecord` are compatibility views generated from the kernel and do not feed canonical decisions.
- The path that reparsed line numbers from legacy `EvidenceSource.detail` was removed; Markdown scanner byte spans flow directly into facts.
- Kernel deserialization rejects fact ids that are non-contiguous or reordered relative to storage order.
- Persistent binary storage, arenas, SoA, parallel scanning, and unsafe code are not required by Q13 and remain deferred until measurement or ABI requirements justify them.

#### Q14 Implementation Record

- `RouteAssessment` is the canonical repository-route decision representation and does not store the old `RouteState` as an internal field.
- Structural presence uses `RoutePresenceAssessment`, README routing uses `ReadmeRoutingAssessment`, target reachability uses `TargetReachabilityAssessment`, conflicts use `RouteConflictAssessment`, and freshness uses `RouteFreshness` as orthogonal components.
- The README route map first builds `ReadmeRouteAssessment` from raw candidate and target observations, then emits the existing `state` and `reason` as deterministic compatibility projections.
- Repository aggregation separates root structure, README routing, and inherited evidence in `RouteEvidenceGroups`; `route_states` is emitted as a compatibility view of those assessments.
- Reachability keeps repository-local present/missing, external, anchor, mail, and unknown targets separate. The legacy `Verified` projection continues to require a repository-local present target and matching root structure as fixed by Q12.
- Legacy JSON without an assessment rebuilds it from compatibility observations. Native inputs with inconsistent freshness, state, compatibility counts, evidence groups, or policy are rejected during deserialization.
- Downstream claims, priorities, and Codex views may continue reading the deterministic `route_states` compatibility view for now. Q19 owns complete native/compatibility renderer separation.
- Route assessment is a small cold path, so Q14 adds no unsafe code, bit packing, parallel projection, or performance claim.

#### Q15 Implementation Record

- `seiri-fs` separates a walker built around `RepositoryRoot` and `RepositoryWalk` from important-file classification. `RepoFsScan` remains the classified compatibility view and carries `RepositoryWalkSummary`.
- Default walker limits are depth `32` and `100000` entries. `IgnorePolicy` applies default and caller-supplied ignore names by basename, and symlinks are recorded without being followed.
- At Q15, exceeding the depth or entry bound returned `FsError::LimitExceeded`. Block S replaces that behavior with typed partial results whose coverage cannot generate absence.
- Default Markdown limits are `2 MiB` of source, `65536` events, and `1024` diagnostics. Filesystem input is read as bytes so invalid UTF-8 is reported as `InvalidUtf8` separately from I/O failure.
- `DocumentEvent` preserves headings, links, badges, and route candidates in source order with exact `SourceSpan`. `DocumentScan` stores only path, source byte count, events, and diagnostics, not source text.
- Construction and deserialization reject missing event spans, out-of-range spans, and non-deterministic event or diagnostic order.
- Unclosed link labels and targets remain byte-spanned `DocumentDiagnostic` soft failures. Source, event, and diagnostic limit violations are hard failures.
- `ReadmeSummary` and the README route map are compatibility views generated from `DocumentScan`. `EvidenceKernel` consumes canonical `DocumentEvent` values without routing through the summary.
- At Q15, the audit connected one root README document only. Block S extends that to a bounded `DocumentIndex`; this work still does not claim full CommonMark compliance or measured performance improvement.

#### Q16 Implementation Record

- Pattern detectors, missing-decision boundaries, negative fixtures, and the registry live in separate modules, and the built-in registry validates detector and fixture coverage for all 13 `PatternGroup` values during construction.
- Negative fixtures are not metadata-only completion signals: regressions audit every fixture and require its linked detector to return no evidence.
- Profile definitions live in a registry, and weights pass through `StaticProfileWeight` backed by `NonZeroU16`. Score inputs are limited to evidence-backed baseline status and static registry weights.
- Calibration suggestions do not mutate profile scores. A `Present` status without evidence ids is also downgraded to `Missing` for scoring.

#### Q17 Implementation Record

- `.jsonl` calibration has a bounded streaming path that decodes and aggregates one non-empty line at a time. It retains no full `BenchmarkRepoRecord` collection, repository ids, or per-pattern repository sets.
- Known patterns map to `PatternSlot(u16)` values; checked counters hold support, and a registry-sized fixed matrix holds co-occurrence counts. Pattern, route, and pending-key dedupe buffers live only for one record.
- Non-zero limits bound line bytes, patterns per record, pending-pattern cardinality, and metadata-source cardinality. Limit violations, invalid UTF-8, and counter overflow fail closed through typed errors.
- Streaming identity means one non-empty JSONL line is one repository record. Global repository-id uniqueness is explicitly an input-preparation responsibility; the kernel performs no implicit approximate dedupe.
- `CalibrationResourceTrace` records retained records, repository-id entries, per-pattern sets, aggregate slots, peak line/pattern size, and a replay digest. These are structural diagnostics, not measured memory, throughput, or performance guarantees.
- The replay digest is a deterministic FNV-1a diagnostic over ordered input records, not a cryptographic digest. Public redaction of local-only sources also removes the digest.
- The materialized JSON dataset API remains as a compatibility path, while report/CLI routing uses streaming only for `.jsonl`. Q17 adds no unsafe code, parallelism, persistent binary format, or performance-improvement claim.

#### Q18 Implementation Record

- `PatchProposal` binds every `PatchTextEdit` to a scanner-derived `TextDocumentBase`. The base retains the exact byte length, UTF-8/UTF-8 BOM/unknown encoding, LF/CRLF/none/mixed EOL, terminal-line-ending state, and a deterministic FNV-1a digest without retaining the document body.
- The core separates schema/model types in `patch_proposal.rs`, raw-byte metadata in `patch_proposal/base.rs`, and preflight plus the pure apply kernel in `patch_proposal/engine.rs` so public types and execution boundaries remain distinct.
- Byte spans enforce `start <= end`, including during deserialization. Structural preflight sends out-of-bounds spans, overlaps, coincident insertions, output-length overflow, and replacement-EOL mismatches to `Reject`.
- Before application, current bytes are re-profiled. A stale digest, encoding/EOL/terminal-line-ending mismatch, or a span that splits a UTF-8 code point is rejected. Undecided policy, security, support, or lifecycle content remains a typed `UnresolvedPolicySlot` with a `Hold` decision and is never silently converted to literal text.
- `apply_to_bytes` is a pure operation that creates a new owned byte buffer only for a Ready proposal. It performs no filesystem write, branch, commit, push, PR, or policy selection, and the planner remains dry-run without invoking application.
- Safe Patch Planner v3 requires the README docs-route operation to carry a proposal. It promotes the candidate only when scanner path/base metadata match and structural preflight is Ready. JSON and Markdown reports expose the digest, encoding, EOL, spans, content kind, and decision.
- The FNV-1a digest is a deterministic stale-base guard, not a cryptographic integrity, security, quality, or trust guarantee. Q18 uses safe Rust only and does not guess unknown encodings or ambiguous EOL conventions.

#### Q19 Implementation Record

- `CodexReviewKernel` is the only Codex view-generation boundary. It projects the v1 compatibility view, native v2, queries, and linter context from the same `RepoSnapshot`, `PatchPlan`, and `WordingLintReport`; renderers do not reimplement repository decisions.
- Schema and argv invariants live in `seiri-core/codex_view`; kernel projection lives in `seiri-codex/v4.rs`; actions, linter, summary, and renderers are v4 sibling modules; and CLI dispatch lives in `seiri-cli/codex.rs`, allowing verification by dependency boundary.
- The existing `CodexReviewContext` and default CLI output remain the v1 compatibility view. Existing schema, PR draft, flat digests, and rendered PowerShell commands stay available, so the default command for old consumers does not change.
- Native v2 uses `seiri.codex.native.v2` and emits canonical `DocumentScan`, `EvidenceKernel`, `RouteAssessment`, `ContentClaim`, the Q18 `PatchPlan`, and full linter context. Root-level `readme`, legacy evidence/ledger, `route_states`, flat actions, and PR fields do not leak into the native root.
- Native summaries do not reuse legacy count names; they retain counts for canonical evidence facts, route assessments, claims, and orthogonal route components only.
- Commands are stored as `CodexCommand { program, args }`; empty programs and NUL are rejected during deserialization. A repository path remains one argv element, while a shell string is generated only in the v1 compatibility projection with single-quote escaping. The kernel never executes commands.
- Queries use typed `summary`, `routes`, `patches`, `linter`, and `actions` variants. A `seiri.codex.query.v2` view materializes only the requested projection from the same kernel. Linter context carries full findings, rules, and boundaries instead of only a digest.
- The CLI keeps `--schema compatibility-v1 --view context` as its default and adds `--schema native-v2`, `--view query --query <kind>`, and `--view linter`. PR body output remains a v1 presentation view.
- Q19 is a safe-Rust, read-only projection. It adds no argv execution, filesystem mutation, GitHub API call, branch, commit, push, merge, policy adoption, or quality/trust guarantee.

### 10. Evidence-Closed Route Engine Roadmap v3

While preserving the Q12-Q19 typed kernel, the next stage separates whether a scope was scanned completely, which content was observed inside a route, and whether a gap is a route, content, consistency, or unknown gap. This roadmap is fixed as Q20-Q34.

#### Corrections Fixed From Critique

- A missing candidate pattern on a `Verified` route is not called a missing route. Route gaps and content gaps use separate types.
- Nested or fixture evidence is not implicitly projected into organization inheritance. `Inherited` is limited to an explicit organization source, and license remains repository-local.
- Scan limits, invalid UTF-8, parse failures, unsupported syntax, and permission failures never become `Missing`. Only complete coverage can produce absence.
- Aggregate calibration estimates remain separate from repository observations and are not promoted into judgments of quality, trust, security, popularity, or maintenance.
- Prefer safe Rust, typed ids, bounded allocation, and deterministic order. Unsafe code, SIMD, custom binary formats, and parallelism remain deferred until measurement justifies them.

#### Fixed Roadmap

| Phase | Scope | Completion criteria |
| --- | --- | --- |
| Q20: Gap Taxonomy | Separate `RouteGap`, `ContentGap`, `ConsistencyGap`, and `ObservationUnknown`. | A content gap on a `Verified` route cannot become the top missing route, and legacy top-route fields refer only to actual route gaps. |
| Q21: Evidence Kernel v2 | Add typed `EvidenceAtom`, `EvidenceProvenance`, `DocumentId`, and `SourceDomain`. | Store each path once in a document table without leaking new fields into native v1/v2 wire output. |
| Q22: Coverage Algebra | Add `CoverageIndex` and `Observation<T>`. | Reject Present/Conflict without evidence, and allow only complete coverage to produce Absent. |
| Q23: Partial Repository Walk | Return bounded partial records with a typed truncation reason on limits. | A partial walk cannot produce absence, while symlinks remain unfollowed and order remains deterministic. |
| Q24: DocumentIndex v2 | Index document role, digest, encoding, byte budget, and scan status. | README, docs, policy, and GitHub configuration candidates are referenced by unique document ids. |
| Q25: Multi-document Markdown | Bounded-scan major Markdown documents beyond the README. | Preserve byte spans and diagnostics, and propagate parser failure as Unknown. |
| Q26: Predicate Program | Add a bounded postfix evaluator over typed atoms. | Reject invalid stacks, excessive atoms, and invalid arity at registry load, and match a test-only reference evaluator. |
| Q27: Route Content Engine | Evaluate content atoms for all 14 routes using four states. | Separate content presence from content adequacy without asserting correctness or sufficiency. |
| Q28: Structured GitHub Parsers | Convert issue forms, workflows, dependency bots, and CODEOWNERS into typed IR. | Enforce parser byte/node/depth/scalar budgets and span-aware diagnostics without executing configuration. |
| Q29: Profile Facets | Allow package, binary, infrastructure, docs, research, template, and other facets to coexist. | Do not treat a single winner as a repository-type assertion; retain facet evidence. |
| Q30: Obligation / Conflict Graph | Build facet-conditional obligations and cross-document conflicts. | Bind obligation reasons and both sides of a conflict to EvidenceIds. |
| Q31: Remote Evidence | Isolate an opt-in read-only adapter in a separate crate. | Distinguish not requested, denied, not found, rate limited, and unavailable, and never serialize tokens. |
| Q32: Pattern / Calibration v4 | Add pattern packs and conditional denominators. | Require positive, negative, ambiguous, partial, and malformed fixtures plus a registry fingerprint. |
| Q33: Codex Native v3 | Add borrowed, query-first views. | Preserve compatibility-v1/native-v2 golden output and avoid unnecessary clones of large collections. |
| Q34: Planner / Regression v4 | Bind patches to analysis run, base digest, and anchor context. | Keep policy-sensitive content manual/held and refuse patches from stale analysis. |

#### Implementation Blocks

| Block | Phases | Boundary |
| --- | --- | --- |
| R | Q20-Q22 | Implement meaning repair, typed evidence, and coverage only. |
| S | Q23-Q27 | Implement local filesystem, document, predicate, and content evaluation. |
| T | Q28 | Implement local structured GitHub parsers. |
| U | Q29-Q30 | Implement facets, conditional obligations, and the conflict graph. |
| V | Q31-Q32 | Implement optional remote input, pattern packs, and privacy-safe calibration. |
| W | Q33-Q34 | Implement Native v3, patch planning, and whole-system regression. |

#### Block R Implementation Record

- `ReviewGap` separates route, content, consistency, and observation-unknown cases as a tagged enum. The legacy `MissingRoutePriorityReport` remains for compatibility, but `top_route` and `top_priority_x100` are selected only from actual route gaps.
- `RepoSnapshot` stores `ReviewPriorityReport` as a canonical internal field and does not serialize it into legacy/native v2 wire output before Q33.
- `EvidenceKernelV2` is built deterministically from the existing kernel, stores document paths once in a sorted document table, and gives facts typed atoms, source domains, producers, document ids, and `u32` byte spans.
- `Root`, `Nested`, and `Generated` are repository-local domains, while `Fixture` is a fixture domain. Organization inheritance is not generated without an explicit source.
- Route aggregation accepts only v2 provenance marked `OrganizationInherited` as inherited evidence and always leaves license inheritance empty.
- `CoverageIndex` rejects duplicate scopes and assigns contiguous non-zero ids. `observe_absence` returns Absent only for Complete and returns typed Unknown for partial or not-requested scopes.
- `Observation::Present` and `Observation::Conflict` require non-empty, sorted, deduplicated `EvidenceSet` values.
- The v2 kernel, coverage index, and review priority remain outside compatibility wire output, preserving Q12-Q19 consumers.
- Block S adds partial filesystem results, a predicate VM, multi-document parsing, and route-content observations. Remote sources and Native v3 remain later work.

#### Block S Implementation Record

- The walker processes directory entries in path order and returns a partial result with `WalkCompletion::Truncated(WalkTruncation)` at entry/depth bounds. Symlinks remain recorded but untraversed, and partial repository coverage produces `Unknown(LimitExceeded)` rather than absence.
- `DocumentIndex` retains role, declared byte count, scan status, FNV-1a base digest, encoding, and a later v2 document id in path order. It indexes README, docs, policy Markdown, and GitHub configuration candidates; configuration receives only a bounded raw-byte base without execution or syntax interpretation.
- Markdown scanning has document-count and aggregate-source-byte budgets. UTF-8, I/O, source/event/diagnostic failures become per-document status and typed partial coverage rather than audit-wide hard failures. The README compatibility view still derives from the root README only.
- Multi-document events enter canonical evidence, while README route assessment and the `ReadmeRoute` detector read only the root README path. A root policy document cannot be promoted into a README route.
- `PredicateProgram` is a typed postfix VM with validated atom, operation, and stack bounds. Invalid atoms, arities, thresholds, stack shapes, and final depths are rejected during construction/registry validation and checked against a test-only reference evaluator.
- `RouteContentAtom` assigns two atoms to each of the 14 routes and retains `Present`, `Absent`, `Unknown`, or `Conflict`. It records observed markers only and does not assert content correctness, adequacy, quality, or the validity of support/security policy.
- `DocumentIndex` and route content remain internal snapshot state and are excluded from compatibility JSON/native v2 wire output. Block T adds Q28 structured parsing; remote sources and Native v3 remain unimplemented.

#### Block T Implementation Record

- `seiri-github-local` is a local-only crate that converts only issue forms, GitHub Actions workflows, Dependabot/Renovate configurations, and CODEOWNERS into read-only typed IR. It never executes configuration, uses the network, calls the GitHub API, triggers workflow actions, or performs dependency updates.
- The parser enforces source-byte, node, indentation-depth, scalar-byte, and diagnostic-count limits. A limit creates a `StructuredBudgetKind` status and span-aware diagnostic instead of failing the whole audit, and reduces GitHub-configuration coverage to partial/Unknown.
- The YAML path accepts a deliberately restricted mapping/list-indentation subset. Anchors, aliases, block scalars, tab indentation, and out-of-subset lines become `UnsupportedSyntax`. Renovate JSON is decoded read-only after the source budget and separately checked for node, depth, and scalar limits.
- Each document retains the Block S `DocumentIndex` document id, path, kind, status, diagnostics, and optional IR. CODEOWNERS retains patterns and owner tokens line by line; a line without an owner produces `MissingCodeowner` with its exact byte span.
- Only Parsed status makes GitHub configuration/document coverage Complete. UTF-8, permission, malformed, unsupported-syntax, and budget outcomes remain typed incomplete coverage and do not assert configuration validity, workflow success, security, ownership, or correct dependency updates.
- The IR and diagnostics remain internal snapshot state and are excluded from compatibility JSON/native v2 wire output. Q29 facets, Q30 conflict graph, remote sources, and Native v3 remain later work.

#### Block U Implementation Record

- `RepositoryFacet` keeps package, binary, infrastructure, documentation, research, template, and product as a fixed complete set that can coexist. It neither replaces a winning profile branch nor asserts a repository type. Each `FacetAssessment` retains an `Observation<()>` with `EvidenceId`s; no evidence becomes Absent only when repository filesystem coverage is Complete, otherwise it becomes Unknown.
- Facet signals are deterministically derived from existing important files, Markdown evidence, manifests, entrypoints, and limited directory markers. File markers used by facets are also recorded as canonical `EvidenceKernel` facts, so facet rationale does not collapse into path strings alone.
- `ConditionalObligation` is generated only from observed facets and retains facet evidence as a non-empty `EvidenceSet` rationale. It evaluates docs/quickstart for package and research, quickstart/release for binary, security/automation for infrastructure, docs/support for documentation and product, and quickstart/contributing for template as conditional route expectations. This is not a legal, security, quality, or policy guarantee, nor an automatic patch instruction.
- When an obligation route has no evidence, it becomes Absent only if `CoverageScope::RepositoryFiles` is Complete. Partial or not-requested coverage becomes Unknown, so a bounded walk or parser failure cannot assert a missing obligation.
- Document consistency emits `DocumentConflict` only when different documents expose different local targets for the same route. Both sides carry `DocumentId` and `EvidenceId`, and target-group, document, and evidence order are deterministic. The graph is bounded to 128 target groups and 64 conflict pairs; `conflict_coverage` becomes partial at either limit. This observes a potential routing conflict and does not assert whole-document contradiction, inaccuracy, or completeness.
- Facet and document consistency reports remain internal snapshot state and do not serialize into compatibility JSON/native v2 wire. Remote evidence, facet calibration, and native v3 remain later-block scope.

#### Block V Implementation Record

- `seiri-remote` is an opt-in, read-only adapter crate that depends only on `seiri-core`. A normal audit has no transport and never initiates network activity. A remote request passes only through an explicitly injected transport and performs no GitHub mutation, workflow execution, cache write, or repository change.
- Remote results retain `NotRequested`, `Denied`, `NotFound`, `RateLimited`, `Unavailable`, or `Observed` as typed status. Only `NotFound` is a complete read result; denied, rate-limited, and unavailable remain typed partial coverage. The remote report is internal snapshot state and does not serialize into compatibility JSON/native v2 wire.
- Authorization uses `RemoteReadAuthorization`, a non-serializable opaque type whose Debug form is redacted. Core remote IR, request reports, snapshots, errors, and Markdown/JSON renderers have no token field. The adapter applies a source-byte limit and decodes only required GitHub metadata read-only.
- `PatternPack` validates a registry, condition, positive/negative/ambiguous/partial/malformed fixtures, and an FNV-1a registry fingerprint together. The built-in common pack requires every fixture kind for every pattern group and rejects fixture id, group, or pattern mismatches at load time.
- `calibrate_dataset_with_pattern_pack` aggregates only records that match the pack condition and retains eligible/excluded record counts, condition, and fingerprint in `CalibrationPatternPack`. A profile pack uses its profile hint as the conditional denominator. Streaming JSONL emits common all-record pack metadata.
- Calibration input remains in-memory/local-path state, while public JSON/Markdown continues through existing local-only source redaction. Pack metadata contains no source path, body, token, or private analysis value. Calibration suggestions remain pending review and do not automatically adopt a rule or weight.

#### Block W Implementation Record

- `CodexNativeV3View<'a>` borrows the snapshot, patch plan, and wording report and builds a query-first v3 view separately from the owning v1/v2 `CodexReviewKernel`. Large route, evidence, document, and governance collections are returned as slices or references rather than cloned. V3 exposes independent summary, routes, evidence, documents, governance, patches, linter, actions, and remote queries.
- Only the native v3 patch query serializes internal `analysis_run` and operation bindings explicitly. The binding fields on `PatchPlan` and `PatchPlanOperation` are skipped by legacy serde, so compatibility-v1 and native-v2 wire output does not gain new internal fields. The v1/v2 report kernel explicitly uses the `safe_patch_planner.v3` compatibility projection.
- `safe_patch_planner.v4` rereads current README bytes inside the repository root immediately before it creates a Safe proposal. If they differ from the scanner-owned `TextDocumentBase`, it returns no proposal and emits a blocked item. Paths that escape the root, resolve outside it through a symlink, or are not regular files are rejected.
- `PatchAnalysisRun` is identified by a deterministic FNV-1a digest over snapshot metadata. `PatchProposalBinding` retains the analysis run, proposal id/path, base digest, and digest-only context of up to 96 bytes before and after each edit, without retaining source text. Bound preflight rejects proposal, base, or anchor mismatches before application.
- Policy-sensitive candidates remain behind the existing Guarded, Manual, and `UnresolvedPolicySlot` boundaries and receive no v4 binding. V4 remains dry-run only; it does not write files, apply patches, perform GitHub actions, create branches, commit, push, merge, or make policy decisions.
- Q33/Q34 regressions check borrowed route-slice pointer identity, absence of bindings from v1/v2 wire output, v3 binding serialization, bound in-memory application, rejection of a README changed after analysis, and the unbound state of policy-sensitive items.
