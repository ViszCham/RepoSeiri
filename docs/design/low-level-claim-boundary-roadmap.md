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
- depthまたはentry上限を超えた場合は`FsError::LimitExceeded`を返し、切り詰めた部分結果を監査へ渡さない。
- Markdown scannerの既定上限はsource `2 MiB`、event `65536`、diagnostic `1024`とする。filesystem入力はbyte列として読み、UTF-8変換失敗を`InvalidUtf8`としてI/O失敗と分離する。
- `DocumentEvent`はheading、link、badge、route candidateをexact `SourceSpan`付きでsource順に保持する。`DocumentScan`は本文を保持せず、path、source byte数、event、diagnosticだけを保持する。
- event span欠落、source範囲外span、eventまたはdiagnosticの非決定的順序はconstructionとdeserializeで拒否する。
- 未閉鎖link label/targetはscan全体のhard failureへ昇格せず、byte span付き`DocumentDiagnostic`として保持する。source/event/diagnostic上限超過はhard failureにする。
- `ReadmeSummary`とREADME route mapは`DocumentScan`から生成するcompatibility viewとする。`EvidenceKernel`はsummaryを経由せず`DocumentEvent`をcanonical入力として読む。
- Q15でauditへ接続するdocumentはroot README一件に限定する。複数docsの選択・予算配分は後続拡張で扱い、CommonMark完全準拠や性能改善は主張しない。

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

---

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
- Exceeding the depth or entry bound returns `FsError::LimitExceeded`; a truncated partial result never enters the audit.
- Default Markdown limits are `2 MiB` of source, `65536` events, and `1024` diagnostics. Filesystem input is read as bytes so invalid UTF-8 is reported as `InvalidUtf8` separately from I/O failure.
- `DocumentEvent` preserves headings, links, badges, and route candidates in source order with exact `SourceSpan`. `DocumentScan` stores only path, source byte count, events, and diagnostics, not source text.
- Construction and deserialization reject missing event spans, out-of-range spans, and non-deterministic event or diagnostic order.
- Unclosed link labels and targets remain byte-spanned `DocumentDiagnostic` soft failures. Source, event, and diagnostic limit violations are hard failures.
- `ReadmeSummary` and the README route map are compatibility views generated from `DocumentScan`. `EvidenceKernel` consumes canonical `DocumentEvent` values without routing through the summary.
- Q15 connects one root README document to the audit. Multi-document docs selection and budget allocation remain future extensions; this block does not claim full CommonMark compliance or measured performance improvement.

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
