# Roadmap v10: Closure And Product Integrity

## 日本語

### 1. 目的

Roadmap v10 は、RepoSeiri が「finding 0」を「十分に観測できた」と取り違えず、同じ bounded source から再現可能な evidence、delta、plan、summary、completion を生成するための最終改善契約です。Roadmap v9 の semantic identity と completion v3 を土台にし、coverage、bounded I/O、stable identity、extension 実行、Markdown semantics、製品入口、release evidence、empirical calibration を閉じます。

この roadmap は private analysis 本文、exact prior、secret 由来 fingerprint、host absolute path を tracked artifact へ移しません。人気、信頼、安全性、品質、法的適合性、公開準備完了、一般性能を証明しません。

### 2. 全体不変条件

1. `finding_count == 0` だけでは completion にしません。coverage、Unknown、budget exhaustion、conflict coverage、source binding を同時に判定します。
2. 一つの Codex query は一つの canonical source session だけを使います。audit 後の再走査から別時点の linter、plan、action を作りません。
3. filesystem と source read は allocation 前に上限を適用し、上限到達を typed `Partial` / `Unknown` evidence として残します。
4. portable identity に run-local ID、document ordinal、host path、列挙順を入れません。
5. integrity fingerprint は domain-separated、length-delimited、fixed-endian SHA-256 を使います。非暗号 FNV は integrity、binding、private revision に使いません。
6. semantic change は schema 名の据え置きで隠さず、`contract.semantic_revisions` と machine schema に記録します。
7. Markdown code、inline code、HTML comment、raw HTML、fixture、test、example は primary repository claim と区別します。
8. private calibration は public output へ semantic digest を出しません。比較可能性は input owner が渡す opaque revision だけで表します。
9. low-level Rust は boundedness、stable layout、typed state、subprocess control、hash framingの不変条件が必要な場所に限定します。CLI glue と prose rendering を低レイヤ化しません。
10. local pass、Windows receipt、Linux receipt、release/publication authority は別の状態です。

### 3. 実装順序

| Unit | 主責務 | Unit gate |
| --- | --- | --- |
| C0 | Truthful Closure | primary coverage と Unknown を summary/completion が正しく所有する |
| C1 | Bounded Source Session | 全 query が一つの bounded snapshot から生成される |
| C2 | Stable Identity | path insertion と列挙順で portable identity が変わらない |
| C3 | Contract Closure | semantics、schema、launcher validation が同じ revision set に結合する |
| C4 | Extension Completion | executable pack と private overlay が通常 audit の意味へ実際に参加する |
| C5 | Semantic Markdown And Consistency | parser event、route、wording、文書間命題が同じ semantic source を使う |
| C6 | Product Surface | README、docs、planner claim、release metadata が実装事実と一致する |
| C7 | Verification And Release Engineering | timeout、host receipt、CI、fuzz、bundle integrity が source-bound になる |
| C8 | Empirical Calibration And Completion | holdout 指標と最終 completion matrix が揃う |

### 4. C0: Truthful Closure

#### 実装

- document role coverage を `Primary`, `Test`, `Fixture`, `Example`, `Generated`, `Vendored`, `Submodule` ごとに分離します。
- role aggregate は primary document だけで primary route state を決め、除外領域は別の diagnostic count として保持します。
- `CodexSummary` に selected/skipped document、source bytes、coverage state、Unknown count、Absent count、budget exhaustion、conflict coverage を追加します。
- completion gate は finding count だけでなく、必須 route、coverage、unacknowledged Unknown、limit exhaustion、conflict evaluation、source binding を判定します。
- budget により読めなかった文書を `Absent` へ変換せず、理由付き `Unknown` にします。

#### Rust 方針

- `PathClassification` と `DocumentRole` を key にした typed counter を使い、異なる evidence region の暗黙合算を禁止します。
- completion input は summary 用文字列を再解析せず、typed `CoverageLedger` を直接受け取ります。

#### 完了条件

- skipped fixture の `SECURITY.md` が primary `SECURITY.md` の coverage を下げない regression test が通ります。
- finding 0 かつ Unknown > 0 の fixture は completion になりません。
- summary の count と governance projection の count が同じ canonical analysis から一致します。

### 5. C1: Bounded Source Session

#### 実装

- `AuditSourceSession` を導入し、filesystem index、selected documents、bounded bytes、Git-local state、source digest を一度だけ確定します。
- directory enumeration は `read_dir(...).collect::<Vec<_>>()` を廃止し、directory entry cap と repository entry cap を allocation 前に適用します。
- deterministic sort に必要な directory buffer は明示上限を持ち、超過 directory は部分内容を evidence に使わず `limit_exceeded` にします。
- source read は metadata check と `Read::take(max + 1)` を使い、`fs::read` / unbounded `read_to_string` 後の size check を禁止します。
- audit、routes、governance、linter、patches、actions、PR body は同じ session の immutable projections になります。

#### Rust 方針

- `AuditSourceSession<Indexed>` から `AuditSourceSession<Parsed>` への typestate transition を使います。
- bounded byte ownershipは `BoundedBytes` newtype、上限超過は `SourceLimitExceeded` で表します。
- `unsafe` は不要です。

#### 完了条件

- oversized file、oversized directory、deep tree、symlink cycle、changing worktree の hostile test が panic/OOM せず typed terminal state を返します。
- query 内の audit invocation は一回で、linter/patch/action が再走査しないことを counter test で固定します。
- source が途中変更された場合は mixed result を返さず `stale_source_binding` になります。

### 6. C2: Stable Identity

#### 実装

- identity、state、occurrence digest の field set を contract table として固定します。
- portable document identity から `DocumentId`、selection ordinal、path-order dependent field を除きます。
- enum discriminant の `as u8` hash を廃止し、stable tag byte/string を明示します。
- `PatchBaseDigest`、pattern registry fingerprint、executable pack fingerprint を SHA-256 framed digest へ移行します。
- old digest を読む必要がある場合は decode-only migration boundary とし、新規出力には使いません。

#### Rust 方針

- `StableHash` trait は domain tag、field count、length-prefixed bytes、big-endian integerを要求します。
- digest type を目的別 newtype にし、`EvidenceIdentityDigest` と `PrivateCalibrationDigest` の代入を型で防ぎます。

#### 完了条件

- lexicographically earlier path の挿入、filesystem enumeration order の反転、run-local ID の変更で既存 portable identity が変わりません。
- state だけの変更は identity digest を保ち、state digest だけを変えます。
- known-answer vector と cross-platform fixture が一致します。

### 7. C3: Contract Closure

#### 実装

- semantic revisions に Markdown parser、path classification、document selection、coverage、content slots、GitHub semantics、profiles、calibration、delta、planner、completion を追加します。
- portable、analysis、Codex、completion schema の nested array item、enum、required field、`additionalProperties` policy を固定します。
- launcher は schema version だけでなく、contract manifest、semantic revision set、binary digest、bundle metadata を検証します。
- wire meaning を変える場合は migration document、negative fixture、compatibility test を同じ slice で更新します。

#### Rust 方針

- revision key は free-form `String` map ではなく closed enum/newtype registry として所有します。
- deserialization 後に semantic compatibility gate を通し、未知 revision は fail-closed または typed unsupported state にします。

#### 完了条件

- nested malformed payload、unknown enum、missing revision、manifest/hash mismatch が非0終了になります。
- schema、Rust serializer、fixture、launcher の revision set が一つの contract test で一致します。

### 8. C4: Extension Completion

#### 実装

- executable pattern pack の definition を predicate/evaluator registry へ compile し、通常 audit の finding、route、priority へ参加させます。
- common baseline と overlay の precedence、conflict、disable、unsupported predicate を typed terminal state にします。
- private calibration overlay は bounded single read、non-serializable private digest、owner-supplied opaque revision、freshness state を持ちます。
- profile の現行 confidence label は統計的確率と誤読されない `LocalSupportTier` へ改名し、sample size と interval を別 field にします。

#### Rust 方針

- pack parse -> validate -> compile -> evaluate を typestate で分けます。
- evaluator は closed predicate opcode と bounded operand table を使い、dynamic code execution、shell、network、arbitrary path read を許しません。

#### 完了条件

- custom pack を変えると通常 audit の expected finding が変わり、pack fingerprint と provenance が追跡できます。
- unsupported predicate、conflicting overlay、stale private revision は silent fallback しません。
- serialized public output に private digest、exact prior、source path が存在しません。

### 9. C5: Semantic Markdown And Consistency

#### 実装

- route analyzer と wording linter は同じ offset-aware Markdown event stream と dead-zone mask を使います。
- wording linter は raw substring scan を廃止し、visible prose node と link labelだけを検査します。
- route classifier は token boundary、heading hierarchy、Japanese/English aliases、multi-label candidate を扱います。`ci` のような短い substring は単語境界なしで一致させません。
- README、docs、policy、GitHub template の命題を source span と modality 付き proposition graph へ投影し、version、support、security、release、capability の矛盾候補を検出します。
- fixture/test/example の命題は primary public claim と分離します。

#### Rust 方針

- Markdown parser event を compact immutable IR とし、複数 analyzer が slice を共有します。
- classifier は interned token ID と deterministic rule table を使い、hot path で不要な `String` clone を避けます。

#### 完了条件

- fence、indented code、inline code、comment、raw HTML 内の語が route/wording evidence になりません。
- Japanese heading、punctuation、case、plural、short-token collision の regression matrix が通ります。
- contradiction output は両側の repository-relative source span と confidence boundary を持ちます。

### 10. C6: Product Surface

#### 実装

- README first viewport を「何を調べる CLI か」「何を出すか」「何を自動変更しないか」の三点へ絞ります。
- install/run path、実利用 quickstart、実 fixture から生成した example output、主要 query、限界を掲載します。
- architecture、roadmap、completion の詳細は design docs へ route し、README 内で再定義しません。
- planner の名称と説明を existing-target-only operation の実装能力に合わせます。能力を拡張する場合も write/network/policy boundary を維持します。
- docs index の現行正、Japanese/English parity、version、CHANGELOG、release docs を一致させます。

#### 完了条件

- 初見利用者が source checkout から一つの repository audit を再現できます。
- example output に placeholder がなく、fixture と snapshot test から更新されます。
- README、CLI help、schema、plugin skill、docs の capability/version statement が contract test で一致します。

### 11. C7: Verification And Release Engineering

#### 実装

- subprocess supervisor に timeout、output cap、kill、reap、typed failure class を実装します。
- tautological hostile assertion を結果別の具体的 invariant assertion へ置き換えます。
- fuzz target を workspace/release verification へ接続し、parser、bounded reader、pack compiler、schema decoder、delta を対象にします。
- CI action と toolchain の mutable reference を immutable digestまたは明示 version policyへ固定します。
- Windows/Linux host receipt は source digest、Cargo.lock digest、binary digest、command set に結合します。
- plugin bundle は manifest、binary、schema、semantic revision の整合を install 前後で検査します。

#### 完了条件

- hung child、stdout flood、non-zero、missing executable が時間/メモリ上限内で区別されます。
- fmt、workspace test、clippy、MSRV、schema、self-audit、privacy、determinism、fuzz smoke が同じ source binding で記録されます。
- host evidence が欠ける状態を local pass や release ready に昇格しません。

### 12. C8: Empirical Calibration And Completion

#### 実装

- route、wording、consistency、profile、planner に public/synthetic corpus と holdout split を用意します。
- precision、recall、false-positive/false-negative、coverage、runtime、peak allocation を task 別に記録します。
- `LocalSupportTier` には sample count と Wilson/Beta interval など明示した統計量を併記し、頻度 bucket を probability と呼びません。
- private corpus は tracked artifact へ移さず、opaque dataset revision と aggregate result だけを許可します。
- final claim matrix で implemented、locally verified、host verified、empirically calibrated、manual policy を分離します。

#### 完了条件

1. 必須 route の primary coverage が Complete です。
2. unacknowledged Unknown、budget exhaustion、stale source binding が0です。
3. conflict coverage が Complete で、unresolved high-severity conflict が0です。
4. audit、linter、planner、summary、completion が同じ source session digest を共有します。
5. portable identity が insertion/order/run invariant property test を通ります。
6. public schemas と semantic revisions が nested contract test を通ります。
7. executable pack と private overlay の positive/negative/privacy test が通ります。
8. Markdown hostile corpus と bilingual route corpus が通ります。
9. README、docs、CLI、plugin、release metadata が一致します。
10. required local checks が同じ source に対して pass します。
11. required Windows/Linux host receipt が同じ source に結合します。
12. calibration report が holdout と uncertainty を示し、一般性能や信頼の保証へ昇格していません。

### 13. 完成状態

- `IMPLEMENTED`: C0-C8 の source と test が存在します。
- `LOCALLY_VERIFIED`: required local checks が同じ source binding で通っています。
- `HOST_VERIFIED`: required Windows/Linux receipt が同じ source binding に結合しています。
- `CALIBRATED`:定義済み holdout 指標が最低 sample 条件を満たします。
- `READY_FOR_GIT`（machine state: `ready_for_git`）: implementation と required local verification が完了し、Git操作だけが未実行です。
- `EVIDENCE_COMPLETE`:この roadmap の required local/host/calibration evidence が揃っています。

いずれの状態も commit、push、merge、release、publication、visibility change、plugin reinstall の権限を自動付与しません。

---

## English

### 1. Purpose

Roadmap v10 is the final improvement contract that prevents RepoSeiri from mistaking "zero findings" for "sufficient observation" and makes evidence, deltas, plans, summaries, and completion reproducible from one bounded source. It builds on Roadmap v9 semantic identity and completion v3, then closes coverage, bounded I/O, stable identity, extension execution, Markdown semantics, the product entry point, release evidence, and empirical calibration.

This roadmap does not move private-analysis bodies, exact priors, secret-derived fingerprints, or host absolute paths into tracked artifacts. It does not prove popularity, trust, security, quality, legal fitness, publication readiness, or general performance.

### 2. Global Invariants

1. `finding_count == 0` is not completion by itself. Coverage, Unknown states, budget exhaustion, conflict coverage, and source binding are evaluated together.
2. One Codex query uses exactly one canonical source session. Linter, plan, or action output is not produced by rescanning after the audit at another point in time.
3. Filesystem and source reads apply limits before allocation and retain limit hits as typed `Partial` / `Unknown` evidence.
4. Portable identity excludes run-local IDs, document ordinals, host paths, and enumeration order.
5. Integrity fingerprints use domain-separated, length-delimited, fixed-endian SHA-256. Non-cryptographic FNV is not used for integrity, binding, or private revisions.
6. Semantic changes are not hidden behind an unchanged schema name; they are recorded in `contract.semantic_revisions` and machine schemas.
7. Markdown code, inline code, HTML comments, raw HTML, fixtures, tests, and examples are distinguished from primary repository claims.
8. Private calibration emits no semantic digest to public output. Comparability is represented only by an opaque revision supplied by the input owner.
9. Low-level Rust is limited to invariants that require boundedness, stable layout, typed state, subprocess control, or hash framing. CLI glue and prose rendering are not made low-level.
10. Local pass, Windows receipt, Linux receipt, and release/publication authority remain separate states.

### 3. Implementation Order

| Unit | Primary responsibility | Unit gate |
| --- | --- | --- |
| C0 | Truthful Closure | Summary and completion correctly own primary coverage and Unknown states |
| C1 | Bounded Source Session | Every query is generated from one bounded snapshot |
| C2 | Stable Identity | Path insertion and enumeration order do not change portable identity |
| C3 | Contract Closure | Semantics, schemas, and launcher validation bind to one revision set |
| C4 | Extension Completion | Executable packs and private overlays actually participate in normal audit meaning |
| C5 | Semantic Markdown And Consistency | Parser events, routes, wording, and cross-document propositions use one semantic source |
| C6 | Product Surface | README, docs, planner claims, and release metadata match implemented facts |
| C7 | Verification And Release Engineering | Timeouts, host receipts, CI, fuzzing, and bundle integrity are source-bound |
| C8 | Empirical Calibration And Completion | Holdout metrics and the final completion matrix are present |

### 4. C0: Truthful Closure

#### Implementation

- Separate document-role coverage across `Primary`, `Test`, `Fixture`, `Example`, `Generated`, `Vendored`, and `Submodule`.
- Let only primary documents decide the primary route state; retain excluded regions as separate diagnostic counts.
- Add selected/skipped documents, source bytes, coverage state, Unknown count, Absent count, budget exhaustion, and conflict coverage to `CodexSummary`.
- Make the completion gate evaluate required routes, coverage, unacknowledged Unknown states, limit exhaustion, conflict evaluation, and source binding in addition to finding count.
- Preserve unread documents caused by a budget as reasoned `Unknown`, never converting them to `Absent`.

#### Rust Direction

- Use typed counters keyed by `PathClassification` and `DocumentRole`; prohibit implicit aggregation across evidence regions.
- Feed typed `CoverageLedger` directly into completion instead of reparsing summary prose.

#### Completion Conditions

- A skipped fixture `SECURITY.md` cannot reduce coverage for the primary `SECURITY.md`.
- A fixture with zero findings and Unknown > 0 cannot complete.
- Summary and governance counts match because they project the same canonical analysis.

### 5. C1: Bounded Source Session

#### Implementation

- Introduce `AuditSourceSession` to fix the filesystem index, selected documents, bounded bytes, Git-local state, and source digest exactly once.
- Remove `read_dir(...).collect::<Vec<_>>()`; apply directory-entry and repository-entry caps before allocation.
- Give the deterministic-sort directory buffer an explicit cap. When exceeded, do not use partial directory contents as evidence; emit `limit_exceeded`.
- Read source through a metadata check and `Read::take(max + 1)`; prohibit post-allocation checks after `fs::read` or unbounded `read_to_string`.
- Make audit, routes, governance, linter, patches, actions, and PR body immutable projections of the same session.

#### Rust Direction

- Use a typestate transition from `AuditSourceSession<Indexed>` to `AuditSourceSession<Parsed>`.
- Own bounded bytes through a `BoundedBytes` newtype and represent overflow as `SourceLimitExceeded`.
- No `unsafe` is required.

#### Completion Conditions

- Hostile oversized-file, oversized-directory, deep-tree, symlink-cycle, and changing-worktree tests return typed terminal states without panic or OOM.
- A counter test proves one audit invocation per query and no linter/patch/action rescan.
- A mid-query source change returns `stale_source_binding`, never a mixed result.

### 6. C2: Stable Identity

#### Implementation

- Freeze field sets for identity, state, and occurrence digests in a contract table.
- Remove `DocumentId`, selection ordinal, and path-order-dependent fields from portable document identity.
- Replace `as u8` enum hashing with explicit stable byte/string tags.
- Move `PatchBaseDigest`, pattern-registry fingerprints, and executable-pack fingerprints to framed SHA-256.
- Keep any required old digest support at a decode-only migration boundary; never emit it in new output.

#### Rust Direction

- Require a domain tag, field count, length-prefixed bytes, and big-endian integers in the `StableHash` trait.
- Use purpose-specific digest newtypes so `EvidenceIdentityDigest` cannot be substituted for `PrivateCalibrationDigest`.

#### Completion Conditions

- Inserting a lexicographically earlier path, reversing filesystem enumeration, or changing run-local IDs does not change existing portable identities.
- A state-only change preserves identity digest and changes only state digest.
- Known-answer vectors and cross-platform fixtures agree.

### 7. C3: Contract Closure

#### Implementation

- Add semantic revisions for Markdown parsing, path classification, document selection, coverage, content slots, GitHub semantics, profiles, calibration, delta, planner, and completion.
- Freeze nested array items, enums, required fields, and `additionalProperties` policy for portable, analysis, Codex, and completion schemas.
- Make the launcher validate the contract manifest, semantic-revision set, binary digest, and bundle metadata in addition to schema version.
- Update the migration document, negative fixture, and compatibility test in the same slice whenever wire meaning changes.

#### Rust Direction

- Own revision keys in a closed enum/newtype registry instead of a free-form `String` map.
- Run a semantic compatibility gate after deserialization; unknown revisions fail closed or return a typed unsupported state.

#### Completion Conditions

- Nested malformed payloads, unknown enums, missing revisions, and manifest/hash mismatches exit non-zero.
- One contract test proves that schema, Rust serializer, fixtures, and launcher use the same revision set.

### 8. C4: Extension Completion

#### Implementation

- Compile executable pattern-pack definitions into the predicate/evaluator registry so they affect findings, routes, and priorities in normal audits.
- Represent common-baseline/overlay precedence, conflicts, disabling, and unsupported predicates as typed terminal states.
- Give private calibration overlays one bounded read, a non-serializable private digest, an owner-supplied opaque revision, and a freshness state.
- Rename the current profile confidence label to `LocalSupportTier` so it is not mistaken for statistical probability; expose sample size and interval separately.

#### Rust Direction

- Separate pack parse -> validate -> compile -> evaluate through typestate.
- Use closed predicate opcodes and a bounded operand table. Do not permit dynamic code execution, shell access, network access, or arbitrary path reads.

#### Completion Conditions

- Changing a custom pack changes the expected normal-audit finding, with pack fingerprint and provenance retained.
- Unsupported predicates, conflicting overlays, and stale private revisions never silently fall back.
- Serialized public output contains no private digest, exact prior, or source path.

### 9. C5: Semantic Markdown And Consistency

#### Implementation

- Make the route analyzer and wording linter share one offset-aware Markdown event stream and dead-zone mask.
- Replace raw-substring wording scans with checks over visible prose nodes and link labels.
- Make route classification understand token boundaries, heading hierarchy, Japanese/English aliases, and multi-label candidates. A short token such as `ci` never matches without a word boundary.
- Project propositions from README, docs, policies, and GitHub templates into a graph with source spans and modality; detect candidate conflicts in version, support, security, release, and capability claims.
- Separate fixture/test/example propositions from primary public claims.

#### Rust Direction

- Use a compact immutable Markdown event IR shared by analyzers.
- Use interned token IDs and deterministic rule tables in the classifier, avoiding unnecessary hot-path `String` clones.

#### Completion Conditions

- Words inside fences, indented code, inline code, comments, and raw HTML do not become route or wording evidence.
- Regression matrices cover Japanese headings, punctuation, case, plurals, and short-token collisions.
- Contradiction output carries repository-relative source spans for both sides and a confidence boundary.

### 10. C6: Product Surface

#### Implementation

- Limit the README first viewport to what the CLI checks, what it outputs, and what it never changes automatically.
- Include an install/run path, a real-use quickstart, example output generated from a real fixture, main queries, and limitations.
- Route architecture, roadmap, and completion details to design docs instead of redefining them in README.
- Align planner naming and descriptions with existing-target-only operations. Any expansion preserves write/network/policy boundaries.
- Align the current docs authority, Japanese/English parity, version, CHANGELOG, and release docs.

#### Completion Conditions

- A first-time user can reproduce one repository audit from a source checkout.
- Example output has no placeholders and is updated from a fixture and snapshot test.
- A contract test aligns capability/version statements across README, CLI help, schemas, plugin skill, and docs.

### 11. C7: Verification And Release Engineering

#### Implementation

- Add timeout, output cap, kill, reap, and typed failure classes to the subprocess supervisor.
- Replace tautological hostile assertions with concrete result-specific invariants.
- Connect fuzz targets to workspace/release verification for parsing, bounded reading, pack compilation, schema decoding, and deltas.
- Pin mutable CI action and toolchain references through immutable digests or an explicit version policy.
- Bind Windows/Linux host receipts to source digest, Cargo.lock digest, binary digest, and command set.
- Validate plugin manifest, binary, schema, and semantic revisions before and after installation.

#### Completion Conditions

- Hung children, stdout floods, non-zero exits, and missing executables are distinguished within time and memory bounds.
- Format, workspace tests, clippy, MSRV, schemas, self-audit, privacy, determinism, and fuzz smoke are recorded against one source binding.
- Missing host evidence is never promoted to local pass or release readiness.

### 12. C8: Empirical Calibration And Completion

#### Implementation

- Add public/synthetic corpora and holdout splits for routes, wording, consistency, profiles, and planning.
- Record precision, recall, false positives/negatives, coverage, runtime, and peak allocation by task.
- Pair `LocalSupportTier` with sample count and an explicitly named interval such as Wilson/Beta; never call a frequency bucket a probability.
- Keep private corpora out of tracked artifacts; allow only opaque dataset revisions and aggregate results.
- Separate implemented, locally verified, host verified, empirically calibrated, and manual-policy states in the final claim matrix.

#### Completion Conditions

1. Primary coverage for required routes is Complete.
2. Unacknowledged Unknown, budget exhaustion, and stale source binding are zero.
3. Conflict coverage is Complete with zero unresolved high-severity conflict.
4. Audit, linter, planner, summary, and completion share one source-session digest.
5. Portable identity passes insertion/order/run-invariance properties.
6. Public schemas and semantic revisions pass nested contract tests.
7. Executable packs and private overlays pass positive, negative, and privacy tests.
8. Markdown hostile and bilingual route corpora pass.
9. README, docs, CLI, plugin, and release metadata agree.
10. Required local checks pass against the same source.
11. Required Windows/Linux host receipts bind to the same source.
12. Calibration reports include holdout uncertainty without promotion to guarantees of general performance or trust.

### 13. Completion States

- `IMPLEMENTED`: C0-C8 source and tests exist.
- `LOCALLY_VERIFIED`: required local checks pass against the same source binding.
- `HOST_VERIFIED`: required Windows/Linux receipts bind to the same source.
- `CALIBRATED`: defined holdout metrics satisfy minimum sample requirements.
- `READY_FOR_GIT` (machine state: `ready_for_git`): implementation and required local verification are complete, with only Git operations unperformed.
- `EVIDENCE_COMPLETE`: all required local, host, and calibration evidence for this roadmap is present.

No state automatically grants authority to commit, push, merge, release, publish, change visibility, or reinstall the plugin.
