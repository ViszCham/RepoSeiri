# Low-level Claim Boundary Roadmap

## 日本語

### 1. 固定する目的

この roadmap は、RepoSeiri を単なる repository 整理 CLI から、Rust の型で evidence、route state、claim boundary、wording、patch safety を分離する低レイヤな review engine へ寄せるための実装順序です。

現状の RepoSeiri は、file scan、Markdown route scan、pattern registry、profile、safe patch planner、Codex adapter を Rust core に寄せて実装済みです。一方で、claim boundary、report 文言、route meaning、patch operation の一部はまだ文字列中心です。次の実装では、出力文が直接自由文から出るのではなく、型付き evidence と型付き claim から生成される構造に寄せます。

### 2. 批判から固定する方針

| Risk | Design response |
| --- | --- |
| score や route state を品質保証のように読めてしまう | `ContentClaim` と `ClaimBoundaryKind` を導入し、許可された主張と禁止された主張を型で分ける。 |
| `reason: String` と `claim_boundary: String` が増えすぎる | 既存 JSON 互換は残しつつ、主要 report 文を claim id と evidence id に結びつける。 |
| `Verified` が security / quality guarantee のように誤読される | `RouteMeaningRule` で route state ごとに「示すこと」と「示さないこと」を登録する。 |
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
| Q2: RouteMeaning Registry | route/state ごとの meaning と non-claim を静的 registry にする。 | `crates/seiri-core`, `crates/seiri-report` | 全 `RouteKind` と主要 `RouteState` に meaning rule がある。`Verified` でも guarantee を返さない test がある。 |
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
- `ClaimBoundaryKind` は popularity、trust、security、quality、legal、maintenance、runtime verification、publication readiness の保証禁止を表現できる。
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
- lint は保証表現を検出するが、法務判断や security diagnosis はしない。

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
- 外部人気、trust、security、quality の保証。
- private analysis data の公開保存。
- Markdown 完全準拠 parser の実装。
- unsafe code による性能最適化。

### 8. 実装順序

```text
Q0 -> Q1 -> Q2 -> Q3 -> Q4 -> Q5 -> Q6 -> Q7 -> Q8 -> Q9 -> Q10 -> Q11
```

最初の実装単位は `Q0 + Q1 + Q2` までを一塊にしてよい。ここは型と意味境界だけなので、後続の renderer、linter、planner を壊さずに進められる。

`Q3` は match 更新が広いため単独 block にする。`Q4 + Q5` は claim 生成と report 表示を同時に行う。`Q6 + Q7` は span 精度が関係するため、Q6 を最小 line/column 版で先に入れ、Q7 で byte span を拡張してもよい。

---

## English

### 1. Fixed Purpose

This roadmap moves RepoSeiri from a repository organization CLI toward a lower-level Rust review engine that separates evidence, route state, claim boundaries, wording, and patch safety through typed structures.

RepoSeiri already keeps file scanning, Markdown route scanning, the pattern registry, profiles, the safe patch planner, and the Codex adapter close to the Rust core. The remaining weakness is that claim boundaries, report wording, route meanings, and some patch operations are still string-centered. The next implementation should make public output come from typed evidence and typed claims instead of free-form text.

### 2. Decisions Fixed From Critique

| Risk | Design response |
| --- | --- |
| Scores or route states can read like quality guarantees. | Add `ContentClaim` and `ClaimBoundaryKind` so allowed claims and blocked claims are separated by type. |
| `reason: String` and `claim_boundary: String` keep growing. | Preserve existing JSON compatibility, but bind major report sentences to claim ids and evidence ids. |
| `Verified` can be misread as a security or quality guarantee. | Register what each route state indicates and does not indicate through `RouteMeaningRule`. |
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
| Q2: RouteMeaning Registry | Add a static registry for meaning and non-claim boundaries per route/state. | `crates/seiri-core`, `crates/seiri-report` | Every `RouteKind` and major `RouteState` has a meaning rule. Tests prove `Verified` does not imply guarantees. |
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
- `ClaimBoundaryKind` can express blocked guarantees for popularity, trust, security, quality, legal fitness, maintenance, runtime verification, and publication readiness.
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
- The linter detects guarantee wording but does not make legal judgments or security diagnoses.

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
- Guarantees of external popularity, trust, security, or quality.
- Public storage of private analysis data.
- A fully compliant Markdown parser.
- Performance optimization through unsafe code.

### 8. Implementation Order

```text
Q0 -> Q1 -> Q2 -> Q3 -> Q4 -> Q5 -> Q6 -> Q7 -> Q8 -> Q9 -> Q10 -> Q11
```

The first implementation unit may combine `Q0 + Q1 + Q2`. These blocks only establish types and meaning boundaries, so they can proceed without disturbing later renderers, linters, and planners.

`Q3` should be its own block because it touches many exhaustive matches. `Q4 + Q5` can be paired because claim generation and report rendering are tightly connected. `Q6 + Q7` both depend on span precision, but Q6 can land first with minimal line/column support and Q7 can later extend it with byte spans.
