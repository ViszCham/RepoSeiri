# RepoSeiri Roadmap v4: Low-level Semantic Closure And Expansion

## 日本語

### 0. 固定状態

この文書は、Q20-Q34完了後のRepoSeiri次期実装をRoadmap v4として固定します。対象はBlock X-Y-Z-AA-AB-AC-ADです。

- このroadmapは実装順序と完了条件を固定しますが、実装完了、性能向上、外部評価、人気、信頼、安全性、品質を保証しません。
- このroadmapの固定は、branch作成、commit、push、merge、PR作成、GitHub API mutation、repository visibility変更を許可しません。
- 非公開分析はlocal calibration inputです。本文、path、固有ファイル名、未公開の集計値をtracked file、public report、Codex contextへ出しません。
- 10,000件、100,000件、1,000,000件級の値は層化calibration modelであり、完全live crawlや統計的証明ではありません。
- compatibility-v1とnative-v2の既存wire outputを維持します。native-v3は既存能力の露出を完成させます。
- 実装はRust 1.76を下限とし、workspace crateではsafe Rust、bounded input、deterministic order、typed failureを優先します。

| Actor / object | 固定するauthority境界 |
| --- | --- |
| local developerまたはCodex | 人間が対象Blockの実装を明示した場合だけ、RepoSeiriのtracked source、tests、fixtures、docsを変更できる。 |
| roadmap本文 | 実装、file mutation、calibration adoption、plugin install、GitHub操作のauthorityを付与しない。 |
| maintainer review | private calibrationの採用、policy-sensitive文面、compatibility break、plugin install、release、GitHub mutationを決定する。 |
| public audit object | 明示されたrepository root/scope内のlocal bytesと、opt-in read-only adapterのredacted observationだけを対象にする。 |
| private calibration object | tracked tree、golden、Debug、panic、report、Codex context、digest payloadの外側に保持する。 |
| planner object | canonical audit IRとin-memory patch previewまで。working tree、index、refs、remote stateは変更しない。 |

### 1. 分析から固定する前提

| 観測傾向 | Roadmapで固定する応答 |
| --- | --- |
| READMEは詳細manualではなくrouting hubとして機能する | route targetをcanonical、detail、example、alternate、migration、shared hubへ分ける。 |
| 全repo共通の存在確認とprofile別期待値は別物 | common evidence、facet、conditional obligation、local calibration priorを別IRに保つ。 |
| 単独signalよりrouteの組み合わせが重要 | co-occurrenceはFindingの根拠候補とするが、固定頻度をrepository factへ昇格させない。 |
| routeの存在は内容の正しさを示さない | route-specific content contractとclaim boundaryを追加する。 |
| GitHub構造は目的別に意味が変わる | 単一profile断定ではなく、複数facetとheuristic fitを残す。 |
| security、support、ownership、governanceは自動確定できない | Safeは既存targetへのrouting previewまで、policy内容はGuardedまたはManualに固定する。 |
| maintenance/freshnessはlink existenceとは別である | target reachability、temporal activity、lifecycle signalを別型へ分離する。 |

### 2. 現行実装への批判と修正順

| 現行状態 | 問題 | 固定する修正 |
| --- | --- | --- |
| public coreに固定aggregate priorがある | private analysis由来の値が公開runtime既定値になり得る | 値をcoreから除去し、明示local providerだけが供給する。 |
| `profile_score_x100`と`confidence_x100` | 品質点または統計確率に見える | `ProfileFit`、`evidence_match`、`rank_score`へ意味を分割する。 |
| self-auditが複数のdocument conflictを数える | 同routeの異なる関連linkが即competitionになる | target roleとrelationを先に判定し、`Competes`だけをconflictへ昇格する。 |
| native-v3内部queryとCLI queryが不一致 | coreのevidence/documents/governance/remote viewをCodexから要求できない | query parserを一元化し、9 queryを露出する。 |
| PatternPack fixtureがmetadata coverage中心 | pattern behaviorの回帰を証明できない | fixture repositoryを実際にauditし、expected observationを比較する。 |
| route contentはmarker存在中心 | routeの何が観測されたかを細かく示せない | 14 routeのcontent slotをtyped registryへ固定する。 |
| freshnessがlocal target reachabilityから導出される | 時間的鮮度と誤読できる | `TargetReachability`と`TemporalActivity`へ分割する。 |
| repository root判定がlocal markerを先に見る | monorepo subdirectoryをGitHub repository rootと誤認し得る | `.git`/gitfile rootを既定にし、subtree解析を明示optionへ移す。 |

### 3. 低レイヤ実装ポリシー

| Surface | 実装レイヤ | 理由 |
| --- | --- | --- |
| byte range、UTF-8 boundary、path normalization、digest input | RepoSeiri内の低レイヤRust | span、失敗位置、allocation上限を制御する。 |
| target relation、coverage algebra、delta、patch binding | RepoSeiri内のtyped IR/pure function | repository mutationなしでproperty testできる。 |
| Markdown event stream | offsetを保持できる実績あるRust parserを薄くadapterする | CommonMark/GFM grammarの再実装を避け、既存byte spanを維持する。 |
| Git object database | `gix`等のread-only純Rust backendをtrait behindで使う | pack/object decodingを独自実装せず、shell、hook、networkを避ける。 |
| workflow/issue form YAML | 現行bounded parserを維持し、static semantic passを追加する | expression実行や完全YAML実装を避ける。 |
| CODEOWNERS matching | 制限付きpatternをbounded opcodeへcompileする | regex backtrackingとunsupported syntaxのsilent acceptanceを避ける。 |
| GitHub API/auth | `seiri-remote`の高レイヤ境界 | token、rate limit、host compatibilityをcoreから隔離する。 |

次の条件を満たす測定が出るまで、unsafe、SIMD、rayon、mmap、独自binary store、custom allocatorを導入しません。

1. 同一fixtureと同一configによるbefore/after benchmarkがある。
2. wall time、peak RSS、allocation countの少なくとも二つが測定される。
3. deterministic outputとRust 1.76互換を維持する。
4. 追加依存またはunsafe boundaryに専用testとfallbackがある。

### 4. 依存順

```text
Q20-Q34 complete
  -> Block X: semantic integrity / private calibration boundary
       -> Block Y: native-v3 and plugin surface completion
       -> Block Z: executable pattern packs
       -> Block AB: git and repository scope substrate
  Block Z + Block X
       -> Block AA: route content contracts
  Block AA + Block AB
       -> Block AC: workflow / intake / ownership semantics
  Block Y + Block AA + Block AB + Block AC
       -> Block AD: audit delta and planner v5
```

- Block Y、Z、ABはBlock X完了後に独立して進められます。
- Block AAはBlock Zのexecutable fixture contractを必須とします。
- Block ACはBlock ABのscope graphとBlock AAのcontent slotを入力にします。
- Block ADは全canonical IRが安定した後にのみ開始します。

### 5. 共通の低レイヤRust contract

以下は設計を固定するためのcontract断片です。実装時にmodule配置を調整しても、variant、failure boundary、serialization boundaryを弱めてはいけません。

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapacityExceeded {
    pub attempted: usize,
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedVec<T, const N: usize>(Vec<T>);

impl<T, const N: usize> BoundedVec<T, N> {
    pub fn try_push(&mut self, value: T) -> Result<(), CapacityExceeded> {
        if self.0.len() >= N {
            return Err(CapacityExceeded {
                attempted: self.0.len().saturating_add(1),
                limit: N,
            });
        }
        self.0.push(value);
        Ok(())
    }

    pub fn as_slice(&self) -> &[T] {
        &self.0
    }
}
```

共通規則:

- allocation上限到達はempty、Absent、Missingへ変換せず、`Partial(LimitExceeded)`または`Unknown`にします。
- pathはrepository-relative normalized UTF-8 viewとraw platform pathを混同しません。
- byte spanは`start <= end <= source_len`をconstructorとDeserializeの両方で検証します。
- rendererはcanonical IRを判断し直しません。
- compatibility projectionだけがlegacy string/count fieldを生成します。

### 6. Block X: Semantic Integrity / Privacy Boundary

#### 6.1 目的

公開coreからprivate-derived fixed priorを除去し、score、target、conflictの意味を修正します。このblockは後続すべての意味境界です。

実装状態: 2026-07-11にlocal working treeでX0-X6を実装済みです。commit、push、merge、plugin再installは実行していません。

#### 6.2 サブフェーズ

| Phase | Scope | 主なwrite surface | 完了条件 |
| --- | --- | --- | --- |
| X0 | 現行golden capture | tests、fixture snapshots | compatibility-v1/native-v2/native-v3 summary、self-audit、pattern outputを固定する。 |
| X1 | fixed prior extraction | `seiri-markdown`, `seiri-report`, core | 通常auditにhardcoded aggregate countが存在しない。 |
| X2 | local calibration provider | `seiri-core`, `seiri-calibration`, CLI boundary | local packは明示指定時だけreadされ、path/body/valueはpublic rendererへ渡らない。 |
| X3 | profile meaning split | `seiri-profiles`, core, report, codex | heuristic fit、evidence match、rank score、calibration priorを別fieldにする。 |
| X4 | route target roles | core、markdown、report | targetがrole、document、evidence、span、normalized targetを持つ。 |
| X5 | target relation/conflict | reportまたはnew core module | `Competes`だけがDocumentConflictになり、`SharedHub`/`Refines`/`Unknown`は別表現になる。 |
| X6 | privacy/compat regression | privacy tests、goldens | tracked/public outputにprivate tokenや固定集計値がなく、legacy schema keyが変わらない。 |

#### 6.3 Rust contract

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriorVisibility {
    PublicSynthetic,
    LocalOnly,
    Redacted,
}

#[derive(Clone, PartialEq, Eq)]
pub struct AggregatePrior {
    observed: u64,
    sample_size: std::num::NonZeroU64,
    rank_weight_x100: u8,
    basis: PriorBasis,
}

pub trait CalibrationProvider {
    fn prior(&self, key: &CalibrationKey) -> CalibrationLookup;
    fn visibility(&self) -> Option<PriorVisibility>;
}

// Intentionally not Serialize/Deserialize/Debug-with-values.
pub struct LocalCalibrationProvider {
    priors: BTreeMap<CalibrationKey, AggregatePrior>,
    registry_fingerprint: Box<str>,
}
```

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum RouteTargetRole {
    Canonical,
    Detail,
    Example,
    Alternate,
    Migration,
    SharedHub,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetRelation {
    Equivalent,
    Refines,
    SharedHub,
    Competes,
    Unknown,
}

pub struct RouteTargetRef {
    pub route: RouteKind,
    pub document: DocumentId,
    pub evidence: EvidenceId,
    pub span: SourceSpan,
    pub role: RouteTargetRole,
    pub normalized_target: String,
}
```

#### 6.4 完了条件

- standard auditはcalibration packを暗黙探索しません。
- local pack指定なしではaggregate priorは`NotRequested`です。
- local packのsource path、raw body、exact count、tokenは`Serialize`可能なsnapshotへ入りません。
- `observed <= sample_size`、非zero denominator、registry fingerprint一致をloaderで検証します。
- `ProfileFit`は確率と呼びません。legacy `confidence_x100`はcompatibility projectionだけに残します。
- `Equivalent`はnormalized target一致など決定的規則だけで生成します。
- 異なるdetail linkをcanonical conflictへ昇格しません。
- relationを決定できない場合は`Unknown`とし、矛盾なしとは報告しません。
- RepoSeiri自己監査の既存conflict候補を分類し直し、true conflict fixtureは残します。
- `cargo test --test privacy_guard`がsynthetic tokenとpublic outputを検査します。

#### 6.5 downgrade条件

- private valueを保持しないとlegacy rendererが動かない場合、rendererを先にoptional prior対応へ変更します。
- target roleのheuristicがevidence spanを失う場合、X5を開始せずX4へ戻します。
- self-audit conflictが0になってもtrue conflict fixtureが消えた場合、成功とみなしません。

### 7. Block Y: Native-v3 / Codex Plugin Surface Completion

#### 7.1 目的

内部にある9種類のnative-v3 queryをCLI、report、Codex pluginから同じ名前とsupport matrixで要求可能にします。

#### 7.2 サブフェーズ

| Phase | Scope | 完了条件 |
| --- | --- | --- |
| Y0 | query slugのsingle source | `Summary`, `Routes`, `Evidence`, `Documents`, `Governance`, `Patches`, `Linter`, `Actions`, `Remote`が同一parserを通る。 |
| Y1 | schema support matrix | compatibility-v1/native-v2で未対応queryをtyped errorにし、silent fallbackしない。 |
| Y2 | CLI exposure | `--schema native-v3 --view query --query <all-nine>`がJSON/Markdownで動く。 |
| Y3 | renderer bounds | queryは要求collectionだけをborrowし、別query collectionをclone/materializeしない。 |
| Y4 | plugin update contract | skill command、query一覧、claim boundaryをnative-v3に同期する。 |
| Y5 | compatibility goldens | v1/v2 bytesとdefault command behaviorを維持する。 |

#### 7.3 Rust contract

```rust
impl std::str::FromStr for CodexNativeV3QueryKind {
    type Err = QueryKindParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "summary" => Ok(Self::Summary),
            "routes" => Ok(Self::Routes),
            "evidence" => Ok(Self::Evidence),
            "documents" => Ok(Self::Documents),
            "governance" => Ok(Self::Governance),
            "patches" => Ok(Self::Patches),
            "linter" => Ok(Self::Linter),
            "actions" => Ok(Self::Actions),
            "remote" => Ok(Self::Remote),
            _ => Err(QueryKindParseError),
        }
    }
}
```

#### 7.4 完了条件

- CLIがcore enumをparseし、5 variantのduplicate enumを持ちません。
- native-v3の9 queryすべてにJSON、Markdown、CLI integration testがあります。
- `Remote` queryはdefaultで`NotRequested`を返し、networkを開始しません。
- `Evidence`/`Documents` queryはscannerの既存上限を超えてmaterializeしません。
- pluginはcommandを実行せず、argvをreview contextとして渡すだけです。
- plugin cache更新、再install、restartはこのblockのcode completionとは別authorityです。

#### 7.5 実装状態（2026-07-11）

- **実装済み**: `CodexNativeV3QueryKind::ALL`、`slug()`、`FromStr`を単一定義とし、CLIの重複5 variant enumを削除しました。
- **実装済み**: schema/view/query対応表は未対応要求を`CodexRequestError`（`CodexError::Request`）として拒否し、別schemaやviewへfallbackしません。
- **実装済み**: native-v3の9 queryをJSON/Markdownの18 CLI経路で検証し、compatibility-v1既定呼び出しとのbyte一致を固定しました。
- **実装済み**: route/evidence/document/governance/patch/linter/remote queryの参照同一性を検証し、actionsは要求時だけargv review dataとして生成します。
- **実装済み**: plugin原本のcommand、9 query一覧、旧schema制限、network/command/claim boundaryを同期しました。cache更新、再install、restartは実行していません。

### 8. Block Z: Executable Pattern Pack / Private Calibration Overlay

#### 8.1 目的

PatternPackを「fixture種別名が揃う」状態から「fixture repositoryを実行して意味が一致する」状態へ進めます。

#### 8.2 サブフェーズ

| Phase | Scope | 完了条件 |
| --- | --- | --- |
| Z0 | executable fixture schema | fixture root、kind、budget、expected observationsをtyped dataにする。 |
| Z1 | bounded fixture loader | absolute path、`..`、symlink escape、過大file、過多expectationを拒否する。 |
| Z2 | in-process audit runner | CLI processやshellを起動せず、library audit entrypointを呼ぶ。 |
| Z3 | expectation comparator | outcomeだけでなくEvidenceId、CoverageStatus、GapKindを比較する。 |
| Z4 | data-only pattern loader | PredicateProgramとboundaryをloadし、invalid stack/arity/unknown atomをload時に拒否する。 |
| Z5 | local calibration overlay | explicit local packをinternal rankingへ適用し、自動adoptionしない。 |
| Z6 | adoption gate | candidateからbaselineへの移動にfixture pass、review record、schema compatibilityを必須化する。 |

#### 8.3 Rust contract

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FixtureExpectation {
    Pattern { pattern: PatternId, outcome: PatternOutcome },
    Coverage { scope: CoverageScope, status: CoverageStatus },
    Gap { kind: ReviewGapKind, minimum: u16, maximum: u16 },
    ClaimBoundary { kind: ClaimBoundaryKind, present: bool },
}

pub struct ExecutableFixtureSpec {
    pub id: FixtureId,
    pub kind: PatternFixtureKind,
    pub root: RelativeFixturePath,
    pub expectations: BoundedVec<FixtureExpectation, 64>,
    pub scan_budget: ScanBudget,
}
```

#### 8.4 完了条件

- 各PatternGroupがpositive、negative、ambiguous、partial、malformedの実行fixtureを持ちます。
- `partial` fixtureはAbsent/Missingを期待値にできません。
- malformed parser inputはpanicせず、typed diagnosticとUnknownを返します。
- fixture runnerはnetwork、GitHub API、Git hooks、subprocessを使いません。
- pattern pack fingerprintはdefinition、predicate bytecode、fixture expectation、versionを含みます。
- private packを使ったrunでもpublic reportはexact priorを表示しません。
- calibration suggestionは`PendingReview`のままで、registryを書き換えません。

#### 8.5 実装状態（2026-07-11）

- **実装済み**: `ExecutablePatternPack`、`ExecutableFixtureSpec`、`RelativeFixturePath`、`FixtureScanBudget`、5種のtyped expectationを追加しました。
- **実装済み**: loaderはpack/fixtureのbyte・entry・depth・count上限を検査し、absolute path、`..`、pack外root、symlink escape、過大fileをtyped errorで拒否します。
- **実装済み**: `run_executable_pattern_pack`はlibrary audit entrypointだけを呼び、13 group × positive/negative/ambiguous/partial/malformedの65 fixtureをsubprocess/networkなしで実行します。
- **実装済み**: outcome、EvidenceId、CoverageStatus、ReviewGapKind、ClaimBoundaryKind、diagnostic件数を比較し、partial fixtureのMissing期待をload時に拒否します。
- **実装済み**: serde対応predicate bytecodeはunknown atom、stack、arity、thresholdをload時に検証し、data-only definitionを`Candidate`と非自動adoption boundaryに固定します。
- **実装済み**: `PrivateCalibrationOverlay`は実行packとlocal priorのfingerprint一致を必須にし、exact priorをpublic JSON/Markdownへ出しません。
- **実装済み**: adoption gateはfixture全pass、明示review、schema/fingerprint一致を要求します。`EligibleForMaintainerAdoption`はregistry変更を実行せず、候補状態を維持します。

### 9. Block AA: Route Content Contract v2

#### 9.1 目的

14 routeについて「routeがあるか」だけでなく「どのcontent slotが観測されたか」を、正しさや十分性を断定せずに表します。

#### 9.2 固定slot

| Route | Content slots |
| --- | --- |
| Identity | purpose、audience、status、scope boundary |
| Docs | index、user guide、API/reference、developer guide、version route |
| Quickstart | prerequisites、install/build、first action、minimal example、expected output |
| Support | channel、question route、support scope、response-expectation statement |
| Intake | bug、feature、docs、question、security redirect、reproduction、version/environment |
| Contributing | developer setup、test command、review prerequisites、acceptance boundary |
| Security | private disclosure、supported versions statement、response-expectation statement、automation route |
| Release | changelog、release channel、compatibility、migration、deprecation policy statement |
| Lifecycle | active/deprecated/moved/archive signal、successor、migration route |
| Governance | decision path、proposal/RFC、roles/maintainers、decision record |
| License | repository-local file、README route、scope statement、additional artifact license route |
| Automation | triggers、job classes、permissions statement、action dependencies、release/docs/security classes |
| Ownership | CODEOWNERS rules、critical-path coverage、owner token syntax、uncovered scope |
| Hygiene | ignored artifacts、large files、generated output、vendored content、artifact storage route |

#### 9.3 サブフェーズ

| Phase | Scope | 完了条件 |
| --- | --- | --- |
| AA0 | Markdown event adapter | reference link、autolink、HTML anchor、image-linkをbyte span付きeventへ変換する。 |
| AA1 | bounded HTML attribute scan | inline HTML全体をDOM化せず、`href`等の必要属性だけを上限付きで読む。 |
| AA2 | content slot registry | 全14 routeのslot、facet condition、policy sensitivityを登録する。 |
| AA3 | slot evaluator | `Present`, `Absent`, `Unknown`, `Conflict`をCoverageStatus付きで生成する。 |
| AA4 | limited meaning | 各slotに`indicates`と`does_not_indicate`を割り当てる。 |
| AA5 | bilingual structural pairing | JA/EN heading pairと同一target集合を`StructurallyParallel`候補として扱う。 |
| AA6 | content gap priority | route gapとcontent gapを混同せず、facet obligationからpriorityを作る。 |
| AA7 | renderer/linter | report、Codex、wording linterが同一contract registryを使う。 |

#### 9.4 Rust contract

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContentSlotKind {
    Purpose,
    Audience,
    Status,
    Prerequisite,
    FirstAction,
    ExpectedOutput,
    DisclosureRoute,
    Compatibility,
    Migration,
    Permission,
    OwnershipCoverage,
    ArtifactStorage,
}

pub struct ContentSlotSpec {
    pub id: ContentSlotId,
    pub route: RouteKind,
    pub kind: ContentSlotKind,
    pub sensitivity: PolicySensitivity,
    pub enabled_by: FacetCondition,
    pub boundaries: BoundedVec<ClaimBoundaryKind, 16>,
}

pub struct ContentSlotAssessment {
    pub slot: ContentSlotId,
    pub observation: Observation<MeaningAtomSet>,
    pub evidence: EvidenceSet,
}
```

#### 9.5 完了条件

- Markdown parser backendはoffset iterator相当のbyte rangeを提供します。
- source text全体をcanonical snapshotへ複製しません。
- unsupported HTML、invalid UTF-8、event budget超過はUnknownです。
- `Absent`は該当document scopeがCompleteの場合だけ生成します。
- Security slotのPresentはsecurity保証へ昇格しません。
- Expected output slotのPresentはcommand実行成功へ昇格しません。
- JA/ENは構造的parallel候補までとし、翻訳同値や意味同一を自動断定しません。
- 同一normalized targetを持つparallel sectionはconflict countを増やしません。
- route content contractの全sentenceはtyped MeaningAtomとClaimBoundaryKindからrenderします。

### 10. Block AB: Git-local / Repository Scope Graph

#### 10.1 目的

GitHub repository root、worktree、monorepo package、subtreeを分離し、link reachabilityとは別のlocal temporal evidenceをread-onlyで収集します。

#### 10.2 サブフェーズ

| Phase | Scope | 完了条件 |
| --- | --- | --- |
| AB0 | `seiri-git-local` crate | core typeだけに依存するread-only adapter境界を作る。 |
| AB1 | repository discovery | `.git` directoryとgitfileを先に解決し、explicit subtree modeを追加する。 |
| AB2 | backend trait / `gix` adapter | object/refs decodeをbackendへ隔離し、shellとhookを使わない。 |
| AB3 | bounded temporal scan | HEAD、refs、tags、bounded commit timestampsをCoverage付きで収集する。 |
| AB4 | workspace manifest scan | Cargo workspace、npm workspace、pyproject、go.workをbyte budget付きで読む。 |
| AB5 | scope graph | repository、workspace、package、docs、example、fixture nodeをtyped edgeで結ぶ。 |
| AB6 | ignored shallow evidence | ignored directoryの存在、kind、reasonだけを保持し、内部へ降りない。 |
| AB7 | freshness split | target reachability、temporal activity、lifecycle signalを別reportにする。 |
| AB8 | worktree/submodule/shallow fixtures | external gitdir、shallow repo、malformed refs、nested packageを回帰固定する。 |

#### 10.3 初期budget

| Budget | Initial limit | Limit result |
| --- | ---: | --- |
| refs | 4,096 | Partial / LimitExceeded |
| tags | 2,048 | Partial / LimitExceeded |
| commit headers | 10,000 | Partial / LimitExceeded |
| workspace nodes | 4,096 | Partial / LimitExceeded |
| manifest bytes per file | 2 MiB | Unknown / SourceTooLarge |
| ignored shallow records | 4,096 | Partial / LimitExceeded |
| submodule recursion | 0 by default | NotRequested |

#### 10.4 Rust contract

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisScope {
    Repository,
    Workspace,
    Subtree,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GitReadBudget {
    pub max_refs: u32,
    pub max_tags: u32,
    pub max_commit_headers: u32,
}

pub trait GitReadBackend {
    fn observe(
        &self,
        root: &RepositoryRoot,
        budget: GitReadBudget,
    ) -> Result<GitObservation, GitReadError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GitTimestamp {
    pub seconds_since_epoch: i64,
    pub offset_minutes: i16,
}
```

#### 10.5 完了条件

- default rootはnearest containing Git repositoryです。
- fixture/subprojectをroot扱いする場合は`--scope subtree`を明示します。
- gitfileのrelative `gitdir:`をcanonicalizeし、discovered git metadata boundary外のarbitrary pathを読まないよう検査します。
- alternate object directory、credential helper、filter process、hook、remote fetchは既定で無効です。
- shallow/partial/malformed repositoryはUnknownまたはPartialであり、inactive/staleとは断定しません。
- timestampは観測値として出し、maintained、abandoned、healthyへ変換しません。
- package scopeのpolicy fileをrepository root policyへ自動昇格しません。
- ignored pathは存在evidenceだけを残し、child countやsizeを推測しません。

### 11. Block AC: Structured GitHub Semantics v2

#### 11.1 目的

workflow、issue form、CODEOWNERS、dependency botを「fileがある」から「静的に何が書かれているか」へ進めます。configurationを実行せず、dynamic expressionはUnknownとして保持します。

#### 11.2 サブフェーズ

| Phase | Scope | 完了条件 |
| --- | --- | --- |
| AC0 | static/dynamic value IR | literal、expression、unsupported、unknownを区別する。 |
| AC1 | workflow triggers/jobs | event、job、step、reusable workflow referenceをbounded IRへ変換する。 |
| AC2 | permission lattice | workflow/job `permissions`をNone/Read/Write/DefaultUnknownで表す。 |
| AC3 | action reference | local、docker、full object id、tag/branch ambiguous、dynamicを分類する。 |
| AC4 | job classifier | test/build/lint/docs/release/security/deploy候補をevidence-linkedに出す。 |
| AC5 | issue form semantics | required top-level、input type、validation、security/question routeを読む。 |
| AC6 | CODEOWNERS compiler | supported patternをbounded opcodeへcompileし、invalid syntaxをdiagnosticにする。 |
| AC7 | critical-path coverage | workflows、security、docs、manifest等のsyntax coverageをscope graph上で計算する。 |
| AC8 | dependency bot semantics | ecosystem、directory、schedule、open limit等のstatic fieldを読む。 |
| AC9 | renderer/fixture | official syntax fixture、unsupported syntax、budget、dynamic expressionを固定する。 |

GitHub issue formsはpublic previewで変更され得るため、schema versionとunsupported fieldを保持します。CODEOWNERSはgitignoreと完全同一ではなく、negationとcharacter range等を同じ意味で受理しません。仕様参照は[Issue forms syntax](https://docs.github.com/en/communities/using-templates-to-encourage-useful-issues-and-pull-requests/syntax-for-issue-forms)、[CODEOWNERS](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-code-owners)、[GitHub Actions secure use](https://docs.github.com/en/actions/reference/security/secure-use)を正とします。

#### 11.3 Rust contract

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StaticValue<T> {
    Literal(T),
    Dynamic { span: EvidenceSpan },
    Unsupported { span: EvidenceSpan },
    Unknown(ObservationUnknownReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenPermission {
    None,
    Read,
    Write,
    DefaultOrInheritedUnknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionReferenceKind {
    LocalPath(Box<str>),
    Docker(Box<str>),
    FullObjectId(Box<str>),
    TagOrBranch(Box<str>),
    Dynamic,
    Malformed,
}
```

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodeownersOp {
    Root,
    Slash,
    Literal(Box<str>),
    Star,
    DoubleStar,
}

pub struct CodeownersPatternProgram {
    pub ops: BoundedVec<CodeownersOp, 256>,
    pub owners: BoundedVec<OwnerToken, 64>,
    pub source: EvidenceSpan,
}
```

#### 11.4 完了条件

- workflow expression `${{ ... }}`を評価しません。
- omitted permissionは`DefaultOrInheritedUnknown`であり、NoneやReadと推測しません。
- full object idらしい文字列はsyntax observationであり、remote repository内の実在commitとは断定しません。
- job classifierはjob成功、test十分性、deployment安全性を主張しません。
- issue formのlabel、assignee、project存在をlocal scanだけで検証済みにしません。
- CODEOWNERS matcherはinvalid lineをskipした事実とspanを残します。
- CODEOWNERS coverageはpattern syntax coverageであり、owner実在、write権限、branch protectionを保証しません。
- dynamic/unsupported syntaxをMissingへ変換しません。
- official syntax変更はfixture updateとschema reviewを経て採用します。

### 12. Block AD: Audit Delta / Planner v5

#### 12.1 目的

同じscope/config/schemaの二つの監査を比較し、route regressionsと改善候補をevidence-linkedに表示します。その上で既存targetへの汎用routing patchをdry-runで生成します。

#### 12.2 サブフェーズ

| Phase | Scope | 完了条件 |
| --- | --- | --- |
| AD0 | analysis configuration fingerprint | schema、scope、budgets、pattern fingerprint、visibilityをdigest inputにする。 |
| AD1 | portable audit snapshot | source text/private priorを含まないcanonical digest viewを定義する。 |
| AD2 | compatibility gate | schema/scope/config不一致をUnknown comparisonとして止める。 |
| AD3 | delta engine | route、content slot、coverage、conflict、facet obligationのAdded/Removed/Changedを出す。 |
| AD4 | regression gate | complete-to-complete比較だけをregression candidateへ昇格する。 |
| AD5 | generic existing-target patch | `AddExistingRouteLink`をtarget role、base digest、anchor、language sectionへbindする。 |
| AD6 | paired-language guard | JA/EN major documentの片側だけを変えるproposalをHoldする。 |
| AD7 | CLI/report | `seiri diff`または同等viewをJSON/Markdownで出し、file writeしない。 |
| AD8 | whole-system regression | stale base、changed anchor、scope mismatch、partial scan、private overlay差を固定する。 |

#### 12.3 Rust contract

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeltaState {
    Added,
    Removed,
    Changed,
    Unchanged,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditSnapshotDigest {
    pub schema: SchemaVersion,
    pub configuration: Digest32,
    pub evidence: Digest32,
    pub routes: Digest32,
    pub documents: Digest32,
}

pub struct RouteDelta {
    pub route: RouteKind,
    pub state: DeltaState,
    pub before: Observation<RouteDigest>,
    pub after: Observation<RouteDigest>,
}
```

```rust
pub struct AddExistingRouteLink {
    pub route: RouteKind,
    pub target: ExistingTargetId,
    pub target_role: RouteTargetRole,
    pub document: DocumentId,
    pub insertion_anchor: PatchAnchorContext,
    pub analysis_run: PatchAnalysisRun,
}
```

#### 12.4 完了条件

- digest algorithmはdeterministic 256-bit implementationをpinし、Rust 1.76とdependency licenseを確認します。
- digestはstale/comparison guardであり、authenticity、signature、security proofとは呼びません。
- private calibration raw valueをsnapshot digest inputへ入れません。必要ならredacted pack fingerprintだけをconfig compatibilityへ使います。
- PartialからAbsentへの変化をregressionとして報告しません。
- targetがrepository-local presentでない場合、Safe proposalを生成しません。
- canonical target conflictまたはUnknown relationがある場合、proposalはHoldです。
- Security、Support、Governance、Ownership、License本文を生成しません。既存fileへのlink previewだけがSafe候補です。
- patchはbase bytes、encoding、EOL、span、anchor、analysis runを再検証します。
- JA/EN paired sectionの一方しか安全に更新できない場合、両方を変更せずHoldします。
- plannerはwrite、branch、commit、push、PR、GitHub API callを行いません。

### 13. Block単位の実装境界

| Block | 一括実装してよい範囲 | 同じblockへ混ぜないもの |
| --- | --- | --- |
| X | X0-X6。schema compatibilityとprivacy修正を同時に閉じる | Git parser、new planner operation |
| Y | query parser、CLI、renderer、plugin source、golden tests | plugin install/restart、GitHub action |
| Z | fixture schema、runner、pack loader、adoption gate | content slot追加、remote crawl |
| AA | Markdown adapter、slot registry、assessment、renderer/linter | Git temporal scan、policy text generation |
| AB | new git-local crate、root/scope、temporal evidence、workspace graph | workflow semantics、delta planner |
| AC | workflow/intake/CODEOWNERS/dependency semantic pass | config execution、remote owner validation |
| AD | snapshot、delta、generic route patch、whole regression | file apply、commit、push、PR creation |

各blockは、そのblockの全exit gateが通るまで次blockのcanonical schemaへ依存させません。実装中のcompatibility shimはblock完了時に「残す理由」と削除条件を文書化します。

#### 13.1 Block別counterexample gate

| Block | 必須counterexample | 失敗時の扱い |
| --- | --- | --- |
| X | 同一routeにcanonicalとdetail linkが共存する。private packにsentinel path/valueが入る。 | detailをconflictにした場合、またはsentinelがpublic surfaceへ出た場合はX未完了。 |
| Y | unsupported schema/query組み合わせと`remote`未要求run。 | silent fallbackまたはnetwork開始があればY未完了。 |
| Z | symlink escape、過大fixture、invalid predicate stack、Partial coverage。 | panic、escape、PartialからMissingへの変換があればZ未完了。 |
| AA | unsupported HTML、invalid UTF-8、同一targetの日英section、Security markerだけの文書。 | correctness/translation/security保証へ昇格した場合はAA未完了。 |
| AB | gitfile escape、shallow repository、malformed ref、nested workspace、ignored large subtree。 | arbitrary path read、stale断定、ignored subtree descentがあればAB未完了。 |
| AC | dynamic workflow expression、omitted permission、unsupported CODEOWNERS syntax、unknown issue-form field。 | expression評価、permission推測、unsupported syntaxのsilent acceptanceがあればAC未完了。 |
| AD | config mismatch、Partial-to-Absent、stale base、片側だけのJA/EN anchor、Unknown target relation。 | regressionまたはSafe patchへ昇格した場合はAD未完了。 |

### 14. 全体完了条件

#### 14.1 必須command

```powershell
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo +1.76.0 check --workspace --all-targets --locked
cargo test --test privacy_guard
cargo run --quiet -p seiri-cli -- audit --path . --profile library --format markdown
cargo run --quiet -p seiri-cli -- codex --path . --profile library --format markdown
cargo run --quiet -p seiri-cli -- codex --path . --profile library --schema native-v2 --format json
cargo run --quiet -p seiri-cli -- codex --path . --profile library --schema native-v3 --view query --query summary --format json
git diff --check
```

Y完了後はnative-v3の9 queryをJSONとMarkdownで全件matrix実行します。AD完了後はdeltaとplannerのstale/partial/scope mismatch fixtureを追加します。

#### 14.2 invariant

- 全IDはnon-zero、deterministic、canonical orderです。
- 全Absent/MissingはComplete coverageを必要とします。
- 全Conflictは両側のDocumentId、EvidenceId、spanを持ちます。
- 全ContentClaimはevidenceまたはBlocked理由を持ちます。
- private source dataはtracked file、golden、panic、Debug、JSON、Markdown、Codex outputへ出ません。
- compatibility-v1/native-v2 goldenは意図しないkey/value driftがありません。
- native-v3 queryはrequestされたview以外を再判断しません。
- plannerはdry-runで、policy-sensitive slotをliteral textへ変換しません。
- test passは外部品質、security、trust、performance proofとして表現しません。

### 15. 保留と非目標

- 単一trust scoreまたはquality score。
- LLMによるdocs correctness、maintainer intent、policy adequacy判定。
- private calibrationの自動発見、自動upload、自動公開。
- org-wide dashboard、default-on remote crawl、token persistence。
- GitHub workflow、issue form、CODEOWNERS、SECURITY、LICENSEの自動確定。
- GitHub Actions expression実行、shell実行、workflow simulation。
- owner実在、permission、branch protection、CI successのlocal-only断定。
- unsafe、SIMD、parallel scan、mmap、独自binary archiveの先行導入。
- file apply、branch、commit、push、merge、PR、visibility変更。

---

## English

### 0. Frozen Status

This document freezes the next RepoSeiri implementation after Q20-Q34 as Roadmap v4. Its scope is Blocks X-Y-Z-AA-AB-AC-AD.

- This roadmap freezes implementation order and completion criteria, but it does not guarantee implementation completion, performance gains, external evaluation, popularity, trust, safety, or quality.
- Freezing this roadmap does not authorize branch creation, commit, push, merge, PR creation, GitHub API mutation, or repository visibility changes.
- Private analysis is local calibration input. Its body, path, unique file names, and unpublished aggregate values must not enter tracked files, public reports, or Codex context.
- The 10,000-, 100,000-, and 1,000,000-scale values are stratified calibration models, not complete live crawls or statistical proof.
- Preserve existing compatibility-v1 and native-v2 wire output. Complete exposure of existing native-v3 capabilities.
- Rust 1.76 remains the minimum. Workspace crates prefer safe Rust, bounded input, deterministic order, and typed failures.

| Actor / object | Frozen authority boundary |
| --- | --- |
| Local developer or Codex | May change tracked RepoSeiri source, tests, fixtures, and docs only after a human explicitly requests implementation of the target block. |
| Roadmap text | Grants no authority for implementation, file mutation, calibration adoption, plugin installation, or GitHub actions. |
| Maintainer review | Decides private-calibration adoption, policy-sensitive wording, compatibility breaks, plugin installation, releases, and GitHub mutation. |
| Public audit object | Includes only local bytes inside the explicit repository root/scope and redacted observations from opt-in read-only adapters. |
| Private calibration object | Remains outside the tracked tree, goldens, Debug, panic text, reports, Codex context, and digest payloads. |
| Planner object | Ends at canonical audit IR and in-memory patch previews; it does not change the working tree, index, refs, or remote state. |

### 1. Premises Frozen From The Analysis

| Observed tendency | Roadmap response |
| --- | --- |
| README acts as a routing hub rather than a detailed manual | Separate route targets into canonical, detail, example, alternate, migration, and shared hub roles. |
| Common existence checks and profile-specific expectations are different | Keep common evidence, facets, conditional obligations, and local calibration priors in separate IR. |
| Route combinations matter more than isolated signals | Use co-occurrence as candidate Finding evidence without promoting fixed frequency to repository fact. |
| Route presence does not indicate content correctness | Add route-specific content contracts and claim boundaries. |
| GitHub structures have profile-dependent meaning | Retain multiple facets and heuristic fit instead of asserting one repository type. |
| Security, support, ownership, and governance cannot be decided automatically | Limit Safe to routing previews toward existing targets; policy content stays Guarded or Manual. |
| Maintenance/freshness differs from link existence | Separate target reachability, temporal activity, and lifecycle signals. |

### 2. Critique Of The Current Implementation And Repair Order

| Current state | Problem | Frozen repair |
| --- | --- | --- |
| Fixed aggregate priors exist in the public core | Values derived from private analysis can become public runtime defaults | Remove them from core and supply them only through an explicit local provider. |
| `profile_score_x100` and `confidence_x100` | They look like quality scores or statistical probabilities | Split meaning into `ProfileFit`, evidence match, rank score, and calibration prior. |
| Self-audit counts multiple document conflicts | Different related links for one route become immediate competition | Classify target role and relation first; only `Competes` becomes a conflict. |
| Native-v3 internal queries and CLI queries differ | Evidence/documents/governance/remote views cannot be requested through Codex | Centralize query parsing and expose all nine queries. |
| PatternPack fixtures mainly validate metadata coverage | They do not verify pattern behavior | Audit fixture repositories and compare expected observations. |
| Route content is marker-presence oriented | It cannot state precisely which content was observed | Freeze content slots for all 14 routes in a typed registry. |
| Freshness is derived from local target reachability | It can be mistaken for temporal freshness | Split it into `TargetReachability` and `TemporalActivity`. |
| Repository root resolution checks local markers first | A monorepo subdirectory can be mistaken for a GitHub repository root | Default to `.git`/gitfile root and move subtree analysis behind an explicit option. |

### 3. Low-level Implementation Policy

| Surface | Implementation layer | Reason |
| --- | --- | --- |
| Byte ranges, UTF-8 boundaries, path normalization, digest input | Low-level RepoSeiri Rust | Control spans, failure locations, and allocation limits. |
| Target relations, coverage algebra, delta, patch binding | RepoSeiri typed IR and pure functions | Enable property testing without repository mutation. |
| Markdown event stream | Thin adapter over a proven Rust parser that retains offsets | Avoid reimplementing CommonMark/GFM while retaining byte spans. |
| Git object database | Read-only pure-Rust backend such as `gix` behind a trait | Avoid custom pack/object decoding, shell, hooks, and network. |
| Workflow/issue-form YAML | Keep the bounded parser and add a static semantic pass | Avoid expression execution and a complete YAML implementation. |
| CODEOWNERS matching | Compile the restricted pattern language into bounded opcodes | Avoid regex backtracking and silent acceptance of unsupported syntax. |
| GitHub API/auth | High-level `seiri-remote` boundary | Isolate tokens, rate limits, and host compatibility from core. |

Do not introduce unsafe, SIMD, rayon, mmap, a custom binary store, or a custom allocator until all of the following measurements exist.

1. A before/after benchmark uses the same fixtures and configuration.
2. At least two of wall time, peak RSS, and allocation count are measured.
3. Deterministic output and Rust 1.76 compatibility remain intact.
4. A dedicated test and fallback cover each added dependency or unsafe boundary.

### 4. Dependency Order

```text
Q20-Q34 complete
  -> Block X: semantic integrity / private calibration boundary
       -> Block Y: native-v3 and plugin surface completion
       -> Block Z: executable pattern packs
       -> Block AB: git and repository scope substrate
  Block Z + Block X
       -> Block AA: route content contracts
  Block AA + Block AB
       -> Block AC: workflow / intake / ownership semantics
  Block Y + Block AA + Block AB + Block AC
       -> Block AD: audit delta and planner v5
```

- Blocks Y, Z, and AB may proceed independently after Block X.
- Block AA requires the executable fixture contract from Block Z.
- Block AC consumes the scope graph from Block AB and content slots from Block AA.
- Block AD begins only after all canonical IR has stabilized.

### 5. Shared Low-level Rust Contract

The following fragments freeze design contracts. Module placement may move during implementation, but variants, failure boundaries, and serialization boundaries must not be weakened.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapacityExceeded {
    pub attempted: usize,
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedVec<T, const N: usize>(Vec<T>);

impl<T, const N: usize> BoundedVec<T, N> {
    pub fn try_push(&mut self, value: T) -> Result<(), CapacityExceeded> {
        if self.0.len() >= N {
            return Err(CapacityExceeded {
                attempted: self.0.len().saturating_add(1),
                limit: N,
            });
        }
        self.0.push(value);
        Ok(())
    }

    pub fn as_slice(&self) -> &[T] {
        &self.0
    }
}
```

Shared rules:

- Reaching an allocation limit becomes `Partial(LimitExceeded)` or `Unknown`, never empty, Absent, or Missing.
- Do not conflate the repository-relative normalized UTF-8 path view with a raw platform path.
- Validate `start <= end <= source_len` for byte spans in constructors and deserialization.
- Renderers do not make canonical IR decisions again.
- Only compatibility projections generate legacy string/count fields.

### 6. Block X: Semantic Integrity / Privacy Boundary

#### 6.1 Goal

Remove private-derived fixed priors from the public core and repair score, target, and conflict semantics. This block is the meaning boundary for all later work.

Implementation status: X0-X6 were implemented in the local working tree on 2026-07-11. No commit, push, merge, or plugin reinstall was performed.

#### 6.2 Subphases

| Phase | Scope | Main write surface | Completion condition |
| --- | --- | --- | --- |
| X0 | Capture current goldens | tests and fixture snapshots | Freeze compatibility-v1/native-v2/native-v3 summary, self-audit, and pattern output. |
| X1 | Extract fixed priors | `seiri-markdown`, `seiri-report`, core | Normal audit has no hardcoded aggregate count. |
| X2 | Local calibration provider | `seiri-core`, `seiri-calibration`, CLI boundary | Local packs are read only when explicit, and path/body/value do not reach public renderers. |
| X3 | Split profile meaning | `seiri-profiles`, core, report, codex | Heuristic fit, evidence match, rank score, and calibration prior are separate fields. |
| X4 | Route target roles | core, markdown, report | A target carries role, document, evidence, span, and normalized target. |
| X5 | Target relation/conflict | report or new core module | Only `Competes` becomes DocumentConflict; `SharedHub`/`Refines`/`Unknown` remain separate. |
| X6 | Privacy/compatibility regression | privacy tests and goldens | No private token or fixed aggregate value appears in tracked/public output, and legacy schema keys remain unchanged. |

#### 6.3 Rust Contract

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriorVisibility {
    PublicSynthetic,
    LocalOnly,
    Redacted,
}

#[derive(Clone, PartialEq, Eq)]
pub struct AggregatePrior {
    observed: u64,
    sample_size: std::num::NonZeroU64,
    rank_weight_x100: u8,
    basis: PriorBasis,
}

pub trait CalibrationProvider {
    fn prior(&self, key: &CalibrationKey) -> CalibrationLookup;
    fn visibility(&self) -> Option<PriorVisibility>;
}

// Intentionally not Serialize/Deserialize/Debug-with-values.
pub struct LocalCalibrationProvider {
    priors: BTreeMap<CalibrationKey, AggregatePrior>,
    registry_fingerprint: Box<str>,
}
```

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum RouteTargetRole {
    Canonical,
    Detail,
    Example,
    Alternate,
    Migration,
    SharedHub,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetRelation {
    Equivalent,
    Refines,
    SharedHub,
    Competes,
    Unknown,
}

pub struct RouteTargetRef {
    pub route: RouteKind,
    pub document: DocumentId,
    pub evidence: EvidenceId,
    pub span: SourceSpan,
    pub role: RouteTargetRole,
    pub normalized_target: String,
}
```

#### 6.4 Completion Conditions

- Standard audit does not implicitly discover a calibration pack.
- Aggregate prior is `NotRequested` without an explicit local pack.
- Local source paths, raw bodies, exact counts, and tokens never enter serializable snapshots.
- Validate `observed <= sample_size`, nonzero denominator, and registry fingerprint at load time.
- Do not call `ProfileFit` a probability. Keep legacy `confidence_x100` only in compatibility projection.
- Generate `Equivalent` only through deterministic rules such as normalized target equality.
- Do not promote different detail links to canonical conflicts.
- If relation cannot be decided, return `Unknown` rather than claiming no contradiction.
- Reclassify current RepoSeiri self-audit conflict candidates while retaining true-conflict fixtures.
- `cargo test --test privacy_guard` checks synthetic tokens and public output.

#### 6.5 Downgrade Conditions

- If a legacy renderer cannot work without private values, first make its prior optional.
- If target-role heuristics lose evidence spans, return to X4 and do not begin X5.
- A self-audit conflict count of zero is not success if the true-conflict fixture disappears.

### 7. Block Y: Native-v3 / Codex Plugin Surface Completion

#### 7.1 Goal

Make all nine internal native-v3 queries requestable through CLI, report, and Codex plugin with the same names and support matrix.

#### 7.2 Subphases

| Phase | Scope | Completion condition |
| --- | --- | --- |
| Y0 | Single source for query slugs | `Summary`, `Routes`, `Evidence`, `Documents`, `Governance`, `Patches`, `Linter`, `Actions`, and `Remote` use one parser. |
| Y1 | Schema support matrix | Unsupported compatibility-v1/native-v2 queries return typed errors without silent fallback. |
| Y2 | CLI exposure | `--schema native-v3 --view query --query <all-nine>` works for JSON and Markdown. |
| Y3 | Renderer bounds | A query borrows only the requested collection and does not clone/materialize other query collections. |
| Y4 | Plugin update contract | Skill commands, query list, and claim boundary match native-v3. |
| Y5 | Compatibility goldens | Preserve v1/v2 bytes and default-command behavior. |

#### 7.3 Rust Contract

```rust
impl std::str::FromStr for CodexNativeV3QueryKind {
    type Err = QueryKindParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "summary" => Ok(Self::Summary),
            "routes" => Ok(Self::Routes),
            "evidence" => Ok(Self::Evidence),
            "documents" => Ok(Self::Documents),
            "governance" => Ok(Self::Governance),
            "patches" => Ok(Self::Patches),
            "linter" => Ok(Self::Linter),
            "actions" => Ok(Self::Actions),
            "remote" => Ok(Self::Remote),
            _ => Err(QueryKindParseError),
        }
    }
}
```

#### 7.4 Completion Conditions

- CLI parses the core enum and has no duplicate five-variant enum.
- All nine native-v3 queries have JSON, Markdown, and CLI integration tests.
- `Remote` returns `NotRequested` by default and starts no network activity.
- `Evidence` and `Documents` do not materialize beyond existing scanner limits.
- The plugin does not execute commands; it transports argv as review context.
- Plugin cache update, reinstall, and restart require authority separate from code completion.

#### 7.5 Implementation Status (2026-07-11)

- **Implemented**: `CodexNativeV3QueryKind::ALL`, `slug()`, and `FromStr` are the single definition; the duplicate five-variant CLI enum was removed.
- **Implemented**: the schema/view/query support matrix rejects unsupported requests as `CodexRequestError` (wrapped by `CodexError::Request`) without falling back to another schema or view.
- **Implemented**: all nine native-v3 queries are covered across 18 JSON/Markdown CLI routes, and byte equality with the default compatibility-v1 invocation is pinned.
- **Implemented**: pointer identity is checked for route/evidence/document/governance/patch/linter/remote queries, and actions generate argv review data only when requested.
- **Implemented**: the plugin source command, nine-query list, legacy-schema limits, and network/command/claim boundaries are synchronized. Cache update, reinstall, and restart were not performed.

### 8. Block Z: Executable Pattern Pack / Private Calibration Overlay

#### 8.1 Goal

Move PatternPack from “fixture class names exist” to “fixture repositories execute and semantic expectations match.”

#### 8.2 Subphases

| Phase | Scope | Completion condition |
| --- | --- | --- |
| Z0 | Executable fixture schema | Type fixture root, kind, budget, and expected observations. |
| Z1 | Bounded fixture loader | Reject absolute paths, `..`, symlink escapes, oversized files, and excessive expectations. |
| Z2 | In-process audit runner | Call the library audit entry point without CLI process or shell. |
| Z3 | Expectation comparator | Compare EvidenceId, CoverageStatus, and GapKind in addition to outcome. |
| Z4 | Data-only pattern loader | Load PredicateProgram and boundary; reject invalid stack, arity, and unknown atoms at load time. |
| Z5 | Local calibration overlay | Apply explicit local packs to internal ranking without automatic adoption. |
| Z6 | Adoption gate | Candidate-to-baseline promotion requires fixture pass, review record, and schema compatibility. |

#### 8.3 Rust Contract

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FixtureExpectation {
    Pattern { pattern: PatternId, outcome: PatternOutcome },
    Coverage { scope: CoverageScope, status: CoverageStatus },
    Gap { kind: ReviewGapKind, minimum: u16, maximum: u16 },
    ClaimBoundary { kind: ClaimBoundaryKind, present: bool },
}

pub struct ExecutableFixtureSpec {
    pub id: FixtureId,
    pub kind: PatternFixtureKind,
    pub root: RelativeFixturePath,
    pub expectations: BoundedVec<FixtureExpectation, 64>,
    pub scan_budget: ScanBudget,
}
```

#### 8.4 Completion Conditions

- Every PatternGroup has executable positive, negative, ambiguous, partial, and malformed fixtures.
- A `partial` fixture cannot expect Absent or Missing.
- Malformed parser input returns typed diagnostics and Unknown without panic.
- The fixture runner uses no network, GitHub API, Git hooks, or subprocesses.
- The pattern-pack fingerprint covers definition, predicate bytecode, fixture expectations, and version.
- Public reports do not reveal exact priors even when a private pack is active.
- Calibration suggestions stay `PendingReview` and do not rewrite the registry.

#### 8.5 Implementation Status (2026-07-11)

- **Implemented**: added `ExecutablePatternPack`, `ExecutableFixtureSpec`, `RelativeFixturePath`, `FixtureScanBudget`, and five typed expectation classes.
- **Implemented**: the loader enforces pack/fixture byte, entry, depth, and count bounds and rejects absolute paths, `..`, roots outside the pack, symlink escapes, and oversized files with typed errors.
- **Implemented**: `run_executable_pattern_pack` calls only the library audit entry point and executes 65 fixtures covering 13 groups × positive/negative/ambiguous/partial/malformed without subprocesses or network access.
- **Implemented**: comparison covers outcome, EvidenceId, CoverageStatus, ReviewGapKind, ClaimBoundaryKind, and diagnostic count; partial fixtures cannot expect Missing.
- **Implemented**: serde predicate bytecode validates unknown atoms, stack, arity, and thresholds at load time; data-only definitions are fixed to `Candidate` with non-automatic-adoption boundaries.
- **Implemented**: `PrivateCalibrationOverlay` requires matching executable-pack/local-prior fingerprints and does not expose exact priors through public JSON or Markdown.
- **Implemented**: the adoption gate requires all fixtures to pass plus explicit review and matching schema/fingerprint. `EligibleForMaintainerAdoption` does not mutate the registry and keeps definitions as candidates.

### 9. Block AA: Route Content Contract v2

#### 9.1 Goal

For all 14 routes, report which content slots were observed without claiming correctness or sufficiency.

#### 9.2 Frozen Slots

| Route | Content slots |
| --- | --- |
| Identity | purpose, audience, status, scope boundary |
| Docs | index, user guide, API/reference, developer guide, version route |
| Quickstart | prerequisites, install/build, first action, minimal example, expected output |
| Support | channel, question route, support scope, response-expectation statement |
| Intake | bug, feature, docs, question, security redirect, reproduction, version/environment |
| Contributing | developer setup, test command, review prerequisites, acceptance boundary |
| Security | private disclosure, supported-versions statement, response-expectation statement, automation route |
| Release | changelog, release channel, compatibility, migration, deprecation-policy statement |
| Lifecycle | active/deprecated/moved/archive signal, successor, migration route |
| Governance | decision path, proposal/RFC, roles/maintainers, decision record |
| License | repository-local file, README route, scope statement, additional artifact-license route |
| Automation | triggers, job classes, permissions statement, action dependencies, release/docs/security classes |
| Ownership | CODEOWNERS rules, critical-path coverage, owner-token syntax, uncovered scope |
| Hygiene | ignored artifacts, large files, generated output, vendored content, artifact-storage route |

#### 9.3 Subphases

| Phase | Scope | Completion condition |
| --- | --- | --- |
| AA0 | Markdown event adapter | Convert reference links, autolinks, HTML anchors, and image-links into byte-spanned events. |
| AA1 | Bounded HTML attribute scan | Read only needed attributes such as `href` without materializing a DOM. |
| AA2 | Content-slot registry | Register slot, facet condition, and policy sensitivity for all 14 routes. |
| AA3 | Slot evaluator | Generate `Present`, `Absent`, `Unknown`, or `Conflict` with CoverageStatus. |
| AA4 | Limited meaning | Attach `indicates` and `does_not_indicate` to each slot. |
| AA5 | Bilingual structural pairing | Treat JA/EN heading pairs with identical target sets as `StructurallyParallel` candidates. |
| AA6 | Content-gap priority | Keep route gaps separate from content gaps and derive priority from facet obligations. |
| AA7 | Renderer/linter | Report, Codex, and wording linter use the same contract registry. |

#### 9.4 Rust Contract

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContentSlotKind {
    Purpose,
    Audience,
    Status,
    Prerequisite,
    FirstAction,
    ExpectedOutput,
    DisclosureRoute,
    Compatibility,
    Migration,
    Permission,
    OwnershipCoverage,
    ArtifactStorage,
}

pub struct ContentSlotSpec {
    pub id: ContentSlotId,
    pub route: RouteKind,
    pub kind: ContentSlotKind,
    pub sensitivity: PolicySensitivity,
    pub enabled_by: FacetCondition,
    pub boundaries: BoundedVec<ClaimBoundaryKind, 16>,
}

pub struct ContentSlotAssessment {
    pub slot: ContentSlotId,
    pub observation: Observation<MeaningAtomSet>,
    pub evidence: EvidenceSet,
}
```

#### 9.5 Completion Conditions

- The Markdown backend provides byte ranges equivalent to an offset iterator.
- Do not duplicate the complete source text into the canonical snapshot.
- Unsupported HTML, invalid UTF-8, and event-budget exhaustion become Unknown.
- Generate `Absent` only when the relevant document scope is Complete.
- Present Security slots do not become security guarantees.
- Present ExpectedOutput slots do not become command-success claims.
- JA/EN pairing is structural only; do not claim translation or semantic equivalence automatically.
- Parallel sections with the same normalized target do not increase conflict counts.
- Every route-content sentence renders from typed MeaningAtom and ClaimBoundaryKind.

### 10. Block AB: Git-local / Repository Scope Graph

#### 10.1 Goal

Separate GitHub repository root, worktree, monorepo packages, and subtrees, and collect read-only local temporal evidence separately from link reachability.

#### 10.2 Subphases

| Phase | Scope | Completion condition |
| --- | --- | --- |
| AB0 | `seiri-git-local` crate | Create a read-only adapter boundary depending only on core types. |
| AB1 | Repository discovery | Resolve `.git` directories and gitfiles first and add explicit subtree mode. |
| AB2 | Backend trait / `gix` adapter | Isolate object/ref decoding and use no shell or hooks. |
| AB3 | Bounded temporal scan | Collect HEAD, refs, tags, and bounded commit timestamps with Coverage. |
| AB4 | Workspace manifest scan | Read Cargo workspaces, npm workspaces, pyproject, and go.work under byte budgets. |
| AB5 | Scope graph | Connect repository, workspace, package, docs, example, and fixture nodes through typed edges. |
| AB6 | Ignored shallow evidence | Retain only existence, kind, and reason for ignored directories without descent. |
| AB7 | Freshness split | Separate target reachability, temporal activity, and lifecycle signals. |
| AB8 | Worktree/submodule/shallow fixtures | Freeze external gitdir, shallow repo, malformed refs, and nested-package regressions. |

#### 10.3 Initial Budgets

| Budget | Initial limit | Limit result |
| --- | ---: | --- |
| refs | 4,096 | Partial / LimitExceeded |
| tags | 2,048 | Partial / LimitExceeded |
| commit headers | 10,000 | Partial / LimitExceeded |
| workspace nodes | 4,096 | Partial / LimitExceeded |
| manifest bytes per file | 2 MiB | Unknown / SourceTooLarge |
| ignored shallow records | 4,096 | Partial / LimitExceeded |
| submodule recursion | 0 by default | NotRequested |

#### 10.4 Rust Contract

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisScope {
    Repository,
    Workspace,
    Subtree,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GitReadBudget {
    pub max_refs: u32,
    pub max_tags: u32,
    pub max_commit_headers: u32,
}

pub trait GitReadBackend {
    fn observe(
        &self,
        root: &RepositoryRoot,
        budget: GitReadBudget,
    ) -> Result<GitObservation, GitReadError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GitTimestamp {
    pub seconds_since_epoch: i64,
    pub offset_minutes: i16,
}
```

#### 10.5 Completion Conditions

- Default root is the nearest containing Git repository.
- Treating a fixture/subproject as root requires explicit `--scope subtree`.
- Canonicalize relative `gitdir:` values and prevent arbitrary reads outside the discovered Git metadata boundary.
- Alternate object directories, credential helpers, filter processes, hooks, and remote fetch are disabled by default.
- Shallow, partial, or malformed repositories become Unknown or Partial, not inactive/stale.
- Report timestamps as observations without converting them to maintained, abandoned, or healthy.
- Do not promote package-scope policy files to repository-root policy automatically.
- Retain only ignored-path existence evidence without guessing child count or size.

### 11. Block AC: Structured GitHub Semantics v2

#### 11.1 Goal

Move workflows, issue forms, CODEOWNERS, and dependency bots from “file exists” toward “which static content was observed.” Never execute configuration; retain dynamic expressions as Unknown.

#### 11.2 Subphases

| Phase | Scope | Completion condition |
| --- | --- | --- |
| AC0 | Static/dynamic value IR | Separate literal, expression, unsupported, and unknown values. |
| AC1 | Workflow triggers/jobs | Convert events, jobs, steps, and reusable-workflow references into bounded IR. |
| AC2 | Permission lattice | Represent workflow/job `permissions` as None/Read/Write/DefaultUnknown. |
| AC3 | Action reference | Classify local, docker, full object id, tag/branch ambiguous, and dynamic references. |
| AC4 | Job classifier | Emit evidence-linked test/build/lint/docs/release/security/deploy candidates. |
| AC5 | Issue-form semantics | Read required top-level fields, input types, validation, and security/question routes. |
| AC6 | CODEOWNERS compiler | Compile supported patterns into bounded opcodes and diagnose invalid syntax. |
| AC7 | Critical-path coverage | Compute syntax coverage for workflows, security, docs, manifests, and similar paths over the scope graph. |
| AC8 | Dependency-bot semantics | Read static ecosystem, directory, schedule, and open-limit fields. |
| AC9 | Renderer/fixtures | Freeze official syntax, unsupported syntax, budget, and dynamic-expression fixtures. |

GitHub issue forms are in public preview and may change, so retain schema version and unsupported fields. CODEOWNERS is not identical to gitignore; do not accept negation and character ranges with gitignore semantics. The authoritative references are [Issue forms syntax](https://docs.github.com/en/communities/using-templates-to-encourage-useful-issues-and-pull-requests/syntax-for-issue-forms), [CODEOWNERS](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-code-owners), and [GitHub Actions secure use](https://docs.github.com/en/actions/reference/security/secure-use).

#### 11.3 Rust Contract

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StaticValue<T> {
    Literal(T),
    Dynamic { span: EvidenceSpan },
    Unsupported { span: EvidenceSpan },
    Unknown(ObservationUnknownReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenPermission {
    None,
    Read,
    Write,
    DefaultOrInheritedUnknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionReferenceKind {
    LocalPath(Box<str>),
    Docker(Box<str>),
    FullObjectId(Box<str>),
    TagOrBranch(Box<str>),
    Dynamic,
    Malformed,
}
```

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodeownersOp {
    Root,
    Slash,
    Literal(Box<str>),
    Star,
    DoubleStar,
}

pub struct CodeownersPatternProgram {
    pub ops: BoundedVec<CodeownersOp, 256>,
    pub owners: BoundedVec<OwnerToken, 64>,
    pub source: EvidenceSpan,
}
```

#### 11.4 Completion Conditions

- Do not evaluate workflow expressions `${{ ... }}`.
- Omitted permissions become `DefaultOrInheritedUnknown`, not None or Read.
- A syntactic full object id is not proof that the commit exists in the remote action repository.
- Job classifiers do not claim job success, test sufficiency, or deployment safety.
- Local scan does not verify that issue-form labels, assignees, or projects exist.
- The CODEOWNERS matcher retains skipped invalid lines and spans.
- CODEOWNERS coverage is syntax coverage, not owner existence, write access, or branch protection.
- Dynamic/unsupported syntax never becomes Missing.
- Official syntax changes require fixture updates and schema review.

### 12. Block AD: Audit Delta / Planner v5

#### 12.1 Goal

Compare two audits with the same scope/config/schema and show evidence-linked route regressions and improvement candidates. Then generate dry-run generic routing patches toward existing targets.

#### 12.2 Subphases

| Phase | Scope | Completion condition |
| --- | --- | --- |
| AD0 | Analysis-configuration fingerprint | Include schema, scope, budgets, pattern fingerprint, and visibility in digest input. |
| AD1 | Portable audit snapshot | Define a canonical digest view without source text or private priors. |
| AD2 | Compatibility gate | Stop schema/scope/config mismatch as Unknown comparison. |
| AD3 | Delta engine | Emit Added/Removed/Changed for routes, content slots, coverage, conflicts, and facet obligations. |
| AD4 | Regression gate | Promote only complete-to-complete comparisons to regression candidates. |
| AD5 | Generic existing-target patch | Bind `AddExistingRouteLink` to target role, base digest, anchor, and language section. |
| AD6 | Paired-language guard | Hold proposals that would change only one half of a major JA/EN document. |
| AD7 | CLI/report | Emit `seiri diff` or equivalent JSON/Markdown view without file writes. |
| AD8 | Whole-system regression | Freeze stale base, changed anchor, scope mismatch, partial scan, and private-overlay differences. |

#### 12.3 Rust Contract

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeltaState {
    Added,
    Removed,
    Changed,
    Unchanged,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditSnapshotDigest {
    pub schema: SchemaVersion,
    pub configuration: Digest32,
    pub evidence: Digest32,
    pub routes: Digest32,
    pub documents: Digest32,
}

pub struct RouteDelta {
    pub route: RouteKind,
    pub state: DeltaState,
    pub before: Observation<RouteDigest>,
    pub after: Observation<RouteDigest>,
}
```

```rust
pub struct AddExistingRouteLink {
    pub route: RouteKind,
    pub target: ExistingTargetId,
    pub target_role: RouteTargetRole,
    pub document: DocumentId,
    pub insertion_anchor: PatchAnchorContext,
    pub analysis_run: PatchAnalysisRun,
}
```

#### 12.4 Completion Conditions

- Pin a deterministic 256-bit digest implementation and verify Rust 1.76 plus dependency licenses.
- The digest is a stale/comparison guard, not authenticity, signature, or security proof.
- Do not include raw private-calibration values in snapshot digest input; use only a redacted pack fingerprint for config compatibility when necessary.
- Do not report a Partial-to-Absent transition as a regression.
- Do not generate a Safe proposal unless the target is repository-local present.
- Hold proposals when a canonical conflict or Unknown relation exists.
- Do not generate Security, Support, Governance, Ownership, or License body text. Only link previews toward existing files can be Safe candidates.
- Revalidate base bytes, encoding, EOL, span, anchor, and analysis run.
- Hold without changing either side when only one JA/EN paired section can be updated safely.
- The planner performs no write, branch, commit, push, PR, or GitHub API call.

### 13. Block Implementation Boundaries

| Block | Scope allowed in one implementation batch | Do not mix into the same block |
| --- | --- | --- |
| X | X0-X6; close schema compatibility and privacy repair together | Git parser, new planner operations |
| Y | Query parser, CLI, renderer, plugin source, golden tests | Plugin install/restart, GitHub actions |
| Z | Fixture schema, runner, pack loader, adoption gate | Content-slot additions, remote crawl |
| AA | Markdown adapter, slot registry, assessment, renderer/linter | Git temporal scan, policy-text generation |
| AB | New git-local crate, root/scope, temporal evidence, workspace graph | Workflow semantics, delta planner |
| AC | Workflow/intake/CODEOWNERS/dependency semantic passes | Config execution, remote owner validation |
| AD | Snapshot, delta, generic route patch, whole regression | File apply, commit, push, PR creation |

Do not let a later block depend on the canonical schema of a block whose exit gates have not passed. At block completion, document why each compatibility shim remains and its removal condition.

#### 13.1 Per-block Counterexample Gate

| Block | Required counterexample | Failure treatment |
| --- | --- | --- |
| X | A canonical and detail link coexist for one route; a private pack contains sentinel paths and values. | X is incomplete if the detail becomes a conflict or a sentinel reaches a public surface. |
| Y | An unsupported schema/query combination and a run where `remote` was not requested. | Y is incomplete if either silently falls back or starts network access. |
| Z | A symlink escape, oversized fixture, invalid predicate stack, and Partial coverage. | Z is incomplete on panic, escape, or conversion from Partial to Missing. |
| AA | Unsupported HTML, invalid UTF-8, JA/EN sections with one target, and a document containing only a Security marker. | AA is incomplete if any case becomes a correctness, translation-equivalence, or security guarantee. |
| AB | A gitfile escape, shallow repository, malformed ref, nested workspace, and ignored large subtree. | AB is incomplete on arbitrary path reads, stale assertions, or descent into the ignored subtree. |
| AC | A dynamic workflow expression, omitted permission, unsupported CODEOWNERS syntax, and unknown issue-form field. | AC is incomplete on expression evaluation, permission inference, or silent acceptance of unsupported syntax. |
| AD | Config mismatch, Partial-to-Absent, stale base, one-sided JA/EN anchor, and Unknown target relation. | AD is incomplete if any case becomes a regression or Safe patch. |

### 14. Whole-roadmap Completion Conditions

#### 14.1 Required Commands

```powershell
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo +1.76.0 check --workspace --all-targets --locked
cargo test --test privacy_guard
cargo run --quiet -p seiri-cli -- audit --path . --profile library --format markdown
cargo run --quiet -p seiri-cli -- codex --path . --profile library --format markdown
cargo run --quiet -p seiri-cli -- codex --path . --profile library --schema native-v2 --format json
cargo run --quiet -p seiri-cli -- codex --path . --profile library --schema native-v3 --view query --query summary --format json
git diff --check
```

After Y, execute all nine native-v3 queries in JSON and Markdown. After AD, add stale/partial/scope-mismatch fixtures for delta and planner.

#### 14.2 Invariants

- Every ID is nonzero, deterministic, and canonically ordered.
- Every Absent/Missing requires Complete coverage.
- Every Conflict has DocumentId, EvidenceId, and span on both sides.
- Every ContentClaim has evidence or a Blocked reason.
- Private source data never enters tracked files, goldens, panic text, Debug, JSON, Markdown, or Codex output.
- compatibility-v1/native-v2 goldens have no unintended key/value drift.
- A native-v3 query does not re-decide views that were not requested.
- The planner remains dry-run and never converts policy-sensitive slots into literal text.
- Test passes are not described as proof of external quality, security, trust, or performance.

### 15. Held Work And Non-goals

- A single trust or quality score.
- LLM judgment of documentation correctness, maintainer intent, or policy adequacy.
- Automatic discovery, upload, or publication of private calibration.
- Organization-wide dashboards, default-on remote crawling, or token persistence.
- Automatic decisions for workflows, issue forms, CODEOWNERS, SECURITY, or LICENSE.
- GitHub Actions expression execution, shell execution, or workflow simulation.
- Local-only claims about owner existence, permissions, branch protection, or CI success.
- Early unsafe, SIMD, parallel scanning, mmap, or custom binary archives.
- File application, branches, commits, pushes, merges, PRs, or visibility changes.
