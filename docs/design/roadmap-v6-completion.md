# Roadmap v6: Completion

## 日本語

実装状態: CF0-CF7は1.0.0 source contractとして現在のworktreeへ実装されています。`ready_for_git`は`seiri.completion.v1`の全blocking checkが同一worktreeで通過した場合だけ記録し、Git/release/plugin操作の権限は付与しません。

### 1. 状態と権限

Roadmap v6 は、RepoSeiri 0.2.0 から local-first personal-use CLI / Codex plugin 1.0.0 へ進むための、次期実装に関する唯一の正本です。Roadmap v5 は 0.2.0 の canonical architecture と legacy removal の実装記録として残りますが、新しい実装順序は所有しません。

この roadmap の承認は、source変更、Git操作、releaseを自動では許可しません。RCBP-v1 triggerが出た場合だけ CF0-CF7 の `MutationAuthority` と `TestAuthority` が有効になります。commit、push、merge、release、plugin再インストール、Codex再起動、visibility変更には、それぞれ明示的な追加指示が必要です。

### 2. 完成の定義

RepoSeiri 1.0.0 は、repository organizationを支援する、ローカル優先・preview-onlyの個人利用向けCLI / Codex pluginです。完成とは、次を同時に満たす状態です。

1. bounded local evidenceからcanonical analysisを一度だけ構築する。
2. routeの観測有無、route condition、content adequacy、profile affinityを混同しない。
3. partial coverageをabsenceへ昇格しない。
4. Safe、Guarded、Manualを分離し、policy本文を自動採用しない。
5. private calibrationを明示的local overlayに限定し、public surfaceへ出さない。
6. RepoSeiri workspace外からstandalone pluginを実行できる。
7. public schema、CLI、exit behavior、plugin contract、compatibility policyを固定する。
8. completion gateを同一commitで通過できる。

完成は、人気、信頼、安全性、品質、法的妥当性、保守状態、production readiness、publication readinessの保証ではありません。

### 3. 固定する境界

- 必須hostは Windows x86_64 と Linux x86_64 です。macOSは1.0.0の必須範囲外です。
- `Common`はbaselineです。domain branchは Library、CLI、Infra、Product、Runtime、Docs、Tutorial、ML、Research、Template の10種類です。
- MLとResearchの分離は実装上のextensionであり、private benchmarkが独立stratumを証明したという主張ではありません。
- 0.3.0で最後のbreaking semantic migrationを行い、analysis、patch-plan、Codex wireをv2 generationへ揃えます。
- 0.1/0.2 wireのfallback、serde alias、silent conversionは追加しません。
- 0.4.0はhardening release candidate、1.0.0はcompletion gate通過版です。
- remote mutation、automatic Git/GitHub action、automatic policy adoption、package registryへの自動公開は範囲外です。

### 4. 現在観測されているblocker

- plugin launcherは`cargo run -p seiri-cli`に依存し、RepoSeiri workspace外で失敗します。
- launcherはnative command failureを成功終了として隠す場合があります。
- DocumentIndexはpath順のbounded selectionにより、重要文書より先にbudgetを消費できます。
- content diagnosticとmarker searchがslotのdocument scopeを越えて影響できます。
- `Overloaded` routeがco-occurrenceでmissing routeとして扱われます。
- 日本語と英語の同一routeがraw candidate countを増やし、Overloaded判定へ影響します。
- profile branchは広いsubstring signalと共通routeに引かれ、無関係profileを同点にできます。
- parser、predicate、patch、streaming calibrationのdeterministic regressionはありますが、完成用fuzz/resource gateは未固定です。

これらは現行実装の観測であり、一般的な品質や安全性の評価ではありません。

### 5. CF0: Completion Contract Freeze

実装slice:

- CF0.1: scope、non-goals、authority、supported hostを固定する。
- CF0.2: `seiri.analysis.v2`、`seiri.patch-plan.v2`、`seiri.codex.v2`、`seiri.error.v1`、`seiri.completion.v1`のownerを固定する。
- CF0.3: stdout、stderr、exit code、Unknown、partial coverageのcompatibility policyを固定する。
- CF0.4: removed v1 inputを拒否するnegative testとmigration noteを追加する。

完了条件:

- active docs、CLI、plugin skill、schema snapshotが同じversionとboundaryを示す。
- success時だけrequested formatをstdoutへ出し、failureはstderrとnon-zero exitで示す。
- 0.2 compatibility shimが存在しない。

### 6. CF1: Standalone Plugin Runtime

実装slice:

- CF1.1: CLI errorをtyped exit classへ写像する。
- CF1.2: plugin launcherからimplicit `cargo run` fallbackを削除する。
- CF1.3: `REPOSEIRI_BIN`、bundle-local binary、PATHの明示順でruntimeを探索する。
- CF1.4: Windows / Linux bundle、version manifest、SHA-256 checksumを生成する。
- CF1.5: unrelated repository、spaceを含むpath、missing binary、schema mismatchのsmoke testを追加する。

完了条件:

- runtimeにRepoSeiri checkoutとRust toolchainを要求しない。
- missing/stale binaryはJSON成功出力を残さずnon-zeroで失敗する。
- launcherはrepositoryを変更せず、coreのstdout、stderr、exit codeを隠さない。

### 7. CF2: Priority DocumentIndex / Coverage v2

実装slice:

- CF2.1: Root README、root policy、README-linked target、docs index、remaining docsの決定的priorityを導入する。
- CF2.2: total budgetの中にcore role reserveを置き、path順だけで重要文書が飢餓状態にならないようにする。
- CF2.3: content slotへDocumentRoleMaskとscope-class filterを追加する。
- CF2.4: diagnosticとmarker searchを対象document scopeへ限定する。
- CF2.5: fixture、test、generated、nested packageをrepository-level contentから分離する。
- CF2.6: role別coverageとselected/skipped countをwireへ出す。

完了条件:

- 大量docsが存在してもcore roleが先に選択される。
- unrelated malformed documentが別routeのcontent slotをUnknownへ変えない。
- relevant coverageがpartialならabsenceを出さない。
- selection、event order、digestが同じinputで決定的である。

### 8. CF3: Route Semantics / Co-occurrence v2

実装slice:

- CF3.1: route evidence presenceとRouteState conditionを別のtyped fieldにする。
- CF3.2: co-occurrence memberをPresent、Degraded、Absent、Unknownで評価する。
- CF3.3: bilingual duplicateをlogical route candidateとして正規化する。
- CF3.4: Overloaded、Conflicting、Staleをroute absenceへ変換しない。
- CF3.5: planner candidateとholdをactual gapへ結び、既にinline routeがある対象へのspurious holdを除く。

完了条件:

- automation routeが観測されている状態で`missing_routes: [automation]`を出さない。
- bilingual duplicateだけではOverloadedにならない。
- Unknown memberはmissing priorityを増やさない。
- patch operationとholdはevidence、target、base digest、gateを持つ。

### 9. CF4: Profile Affinity / Facet Precision

実装slice:

- CF4.1: raw substringをexact filename、path segment、manifest、scope node、typed predicateへ置換する。
- CF4.2: common route coverageであるfitとrepository purpose affinityを分離する。
- CF4.3: fixture、test、generated、supporting exampleをprimary artifact evidenceとして数えない。
- CF4.4: positive、negative、ambiguous、mixed-purpose fixtureを10 domain branchへ追加する。
- CF4.5: multiple candidate、rank tie、low evidence、explicitly selected profileの表示契約を固定する。

完了条件:

- RepoSeiri self-auditではLibraryとCLIが上位集合となり、全branchが同点100にならない。
- profile rankはprobabilityやrepo type assertionとして表示されない。
- private calibrationなしでもbranch orderが決定的である。

### 10. CF5: Hostile Input / Resource Hardening

実装slice:

- CF5.1: stable deterministic property corpusを追加する。
- CF5.2: Markdown、GitHub YAML、CODEOWNERS、predicate VM、patch span、calibration JSONL、gitfile parserのfuzz targetを追加する。
- CF5.3: deep nesting、large scalar、invalid UTF-8、duplicate key、zero budget、span boundaryを固定caseにする。
- CF5.4: no-write、no-network、private-redaction、bounded output invariantをcompletion gateへ接続する。

完了条件:

- panic、out-of-range span、silent truncation、unbounded retained stateがない。
- `#![forbid(unsafe_code)]`を全workspace crateで維持する。
- timing値だけを根拠にperformance improvementを主張しない。

### 11. CF6: Calibration / Executable Pattern Closure

実装slice:

- CF6.1: built-in baselineとexternal executable packのownerを分離する。
- CF6.2: local-only calibration metadata、fingerprint、resource trace、redactionをv2 wireへ揃える。
- CF6.3: pattern group、route、state、profileにpositive、negative、partial、malformed coverageを持たせる。
- CF6.4: materialized ingestとstreaming ingestのdeterministic equivalenceを固定する。

完了条件:

- standard auditはaggregate priorを暗黙適用しない。
- private 100k/1M analysisの本文、path、exact priorをpublic artifactへ出さない。
- calibration suggestionはpattern/profile weightを自動変更しない。
- completionはbenchmark validityやgeneral performanceの証明を意味しない。

### 12. CF7: Completion Harness / Release Closure

実装slice:

- CF7.1: Rust `xtask completion --format json`と`seiri.completion.v1`を追加する。
- CF7.2: fmt、workspace test、clippy、MSRV、schema、privacy、self-audit、plugin smoke、fuzz corpusをblocking checkとして登録する。
- CF7.3: Windows / Linux bundleとchecksumをCI artifactとして生成する。
- CF7.4: README、docs topology、self-audit、release docs、CHANGELOG、plugin skillを同期する。
- CF7.5: fresh plugin cacheとnew Codex threadからunrelated repositoryを監査するmanual acceptanceを記録する。

完了条件:

- 同じcommitに対する全blocking checkがpassする。
- Windows/Linuxのruntime manifest、SHA-256、standalone smoke evidenceが揃わない場合は`incomplete`を維持する。
- blocking checkをskipしたcompletion recordを生成できない。
- final stateは`ready_for_git`または`incomplete`であり、releaseやGit操作を自動実行しない。

### 13. 依存順と停止条件

実装順は `CF0 -> CF1 -> CF2 -> CF3 -> CF4 -> CF5 -> CF6 -> CF7` です。CF2-CF4は同じsemantic surfaceを変更するため並行実装しません。

次の条件ではaffected sliceを停止します。

- public schemaのownerまたはmigrationが不明。
- user変更とowned fileが競合し、安全な統合ができない。
- partial coverageからfalse absenceが生成される。
- private dataまたはcredentialがpublic/ledger surfaceへ出る。
- hidden fallbackがruntime failureを成功へ変換する。
- blocking verificationが環境上実行できない。

停止は全変更の自動rollbackを意味しません。RCBP-v1は変更とresidualを記録し、`incomplete`として引き渡します。

### 14. Final acceptance

RepoSeiri 1.0.0は、以下が同一commitで成立した場合だけ完成候補です。

1. Windows / Linuxのstandalone plugin smokeがunrelated repositoryで成功する。
2. binary欠落とschema mismatchがnon-zeroで失敗する。
3. core document roleがdefault budgetで優先される。
4. route presenceとcondition、content coverage、profile affinityが分離される。
5. RepoSeiri self-auditに既知のautomation contradictionとall-profile tieがない。
6. full regression、clippy、MSRV、privacy、fuzz corpusが通る。
7. private analysisはrepo、bundle、log、reportへ含まれない。
8. 日英文書、CLI help、schema、plugin skillが同じversionとboundaryを示す。
9. completion recordにskipされたblocking checkがない。
10. maintainerがreleaseを手動で承認する。

---

## English

Implementation status: CF0-CF7 are implemented in the current worktree as the 1.0.0 source contract. `ready_for_git` is recorded only when every `seiri.completion.v1` blocking check passes against that same worktree; it grants no Git, release, or plugin authority.

### 1. Status And Authority

Roadmap v6 is the sole authority for the next implementation phase from RepoSeiri 0.2.0 to the local-first personal-use CLI / Codex plugin 1.0.0. Roadmap v5 remains the implementation record for the 0.2.0 canonical architecture and legacy removal, but it does not own the new implementation order.

Approving this roadmap does not automatically authorize source changes, Git operations, or a release. Only an RCBP-v1 trigger activates `MutationAuthority` and `TestAuthority` for CF0-CF7. Commit, push, merge, release, plugin reinstallation, Codex restart, and visibility changes each require an additional explicit instruction.

### 2. Definition Of Completion

RepoSeiri 1.0.0 is a local-first, preview-only CLI / Codex plugin for personal repository-organization review. Completion requires all of the following at the same time.

1. Build canonical analysis exactly once from bounded local evidence.
2. Do not conflate route observation, route condition, content adequacy, and profile affinity.
3. Never promote partial coverage to absence.
4. Separate Safe, Guarded, and Manual without automatically adopting policy text.
5. Keep private calibration in an explicit local overlay and out of public surfaces.
6. Run the standalone plugin outside the RepoSeiri workspace.
7. Freeze public schemas, CLI behavior, exit behavior, plugin contract, and compatibility policy.
8. Pass the completion gate on one commit.

Completion is not a guarantee of popularity, trust, security, quality, legal fitness, maintenance, production readiness, or publication readiness.

### 3. Frozen Boundaries

- Required hosts are Windows x86_64 and Linux x86_64. macOS is outside the required 1.0.0 scope.
- `Common` is the baseline. The ten domain branches are Library, CLI, Infra, Product, Runtime, Docs, Tutorial, ML, Research, and Template.
- Splitting ML and Research is an implementation extension, not a claim that the private benchmark proved an independent stratum.
- Version 0.3.0 performs the final breaking semantic migration and moves analysis, patch-plan, and Codex wires to the v2 generation.
- Do not add 0.1/0.2 wire fallbacks, serde aliases, or silent conversions.
- Version 0.4.0 is the hardening release candidate; 1.0.0 is the completion-gate release.
- Remote mutation, automatic Git/GitHub actions, automatic policy adoption, and automatic package-registry publication are out of scope.

### 4. Currently Observed Blockers

- The plugin launcher depends on `cargo run -p seiri-cli` and fails outside the RepoSeiri workspace.
- The launcher can hide a native command failure behind a successful process exit.
- DocumentIndex uses bounded path-order selection that can spend its budget before selecting important documents.
- Content diagnostics and marker search can affect slots outside the relevant document scope.
- An `Overloaded` route is treated as a missing route by co-occurrence analysis.
- Equivalent Japanese and English routes increase raw candidate counts and can affect Overloaded classification.
- Profile branches depend on broad substring signals and common routes, allowing unrelated profiles to tie.
- Deterministic parser, predicate, patch, and streaming-calibration regressions exist, but the completion fuzz/resource gate is not frozen.

These are observations about the current implementation, not general quality or safety judgments.

### 5. CF0: Completion Contract Freeze

Implementation slices:

- CF0.1: Freeze scope, non-goals, authority, and supported hosts.
- CF0.2: Freeze owners for `seiri.analysis.v2`, `seiri.patch-plan.v2`, `seiri.codex.v2`, `seiri.error.v1`, and `seiri.completion.v1`.
- CF0.3: Freeze compatibility policies for stdout, stderr, exit codes, Unknown, and partial coverage.
- CF0.4: Add negative removed-v1 tests and migration notes.

Completion conditions:

- Active docs, CLI, plugin skill, and schema snapshots show the same versions and boundaries.
- Only successful execution writes the requested format to stdout; failures use stderr and a non-zero exit.
- No 0.2 compatibility shim exists.

### 6. CF1: Standalone Plugin Runtime

Implementation slices:

- CF1.1: Map CLI errors to typed exit classes.
- CF1.2: Remove the implicit `cargo run` fallback from the plugin launcher.
- CF1.3: Resolve runtime in the explicit order `REPOSEIRI_BIN`, bundle-local binary, then PATH.
- CF1.4: Generate Windows / Linux bundles, a version manifest, and SHA-256 checksums.
- CF1.5: Add smoke tests for unrelated repositories, paths with spaces, missing binaries, and schema mismatches.

Completion conditions:

- Runtime does not require a RepoSeiri checkout or Rust toolchain.
- Missing or stale binaries fail non-zero without leaving successful JSON output.
- The launcher does not modify the repository or hide core stdout, stderr, or exit codes.

### 7. CF2: Priority DocumentIndex / Coverage v2

Implementation slices:

- CF2.1: Add deterministic priority for Root README, root policies, README-linked targets, docs indexes, and remaining docs.
- CF2.2: Reserve core roles inside the total budget so path order cannot starve important documents.
- CF2.3: Add DocumentRoleMask and scope-class filters to content slots.
- CF2.4: Limit diagnostics and marker search to the target document scope.
- CF2.5: Separate fixture, test, generated, and nested-package content from repository-level content.
- CF2.6: Expose role-level coverage and selected/skipped counts on the wire.

Completion conditions:

- Core roles are selected before bulk docs.
- An unrelated malformed document cannot turn another route's content slot into Unknown.
- Partial relevant coverage never emits absence.
- Selection, event order, and digests are deterministic for the same input.

### 8. CF3: Route Semantics / Co-occurrence v2

Implementation slices:

- CF3.1: Separate route-evidence presence from RouteState condition.
- CF3.2: Evaluate co-occurrence members as Present, Degraded, Absent, or Unknown.
- CF3.3: Normalize bilingual duplicates into logical route candidates.
- CF3.4: Do not convert Overloaded, Conflicting, or Stale into route absence.
- CF3.5: Bind planner candidates and holds to actual gaps and remove spurious holds for existing inline routes.

Completion conditions:

- Do not emit `missing_routes: [automation]` while automation-route evidence is observed.
- Bilingual duplication alone does not produce Overloaded.
- Unknown members do not increase missing priority.
- Patch operations and holds carry evidence, target, base digest, and gate.

### 9. CF4: Profile Affinity / Facet Precision

Implementation slices:

- CF4.1: Replace raw substrings with exact filenames, path segments, manifests, scope nodes, and typed predicates.
- CF4.2: Separate common route-coverage fit from repository-purpose affinity.
- CF4.3: Do not count fixture, test, generated, or supporting-example paths as primary-artifact evidence.
- CF4.4: Add positive, negative, ambiguous, and mixed-purpose fixtures for all ten domain branches.
- CF4.5: Freeze rendering contracts for multiple candidates, ties, low evidence, and explicitly selected profiles.

Completion conditions:

- RepoSeiri self-audit places Library and CLI in the top set and does not score every branch at 100.
- Profile rank is not rendered as a probability or repository-type assertion.
- Branch order is deterministic without private calibration.

### 10. CF5: Hostile Input / Resource Hardening

Implementation slices:

- CF5.1: Add a stable deterministic property corpus.
- CF5.2: Add fuzz targets for Markdown, GitHub YAML, CODEOWNERS, predicate VM, patch spans, calibration JSONL, and gitfile parsing.
- CF5.3: Freeze cases for deep nesting, large scalars, invalid UTF-8, duplicate keys, zero budgets, and span boundaries.
- CF5.4: Connect no-write, no-network, private-redaction, and bounded-output invariants to the completion gate.

Completion conditions:

- No panic, out-of-range span, silent truncation, or unbounded retained state occurs.
- Keep `#![forbid(unsafe_code)]` across all workspace crates.
- Do not claim performance improvement from timing alone.

### 11. CF6: Calibration / Executable Pattern Closure

Implementation slices:

- CF6.1: Separate ownership of the built-in baseline from external executable packs.
- CF6.2: Align local-only calibration metadata, fingerprints, resource traces, and redaction with the v2 wire.
- CF6.3: Cover every pattern group, route, state, and profile with positive, negative, partial, and malformed cases.
- CF6.4: Freeze deterministic equivalence between materialized and streaming ingest.

Completion conditions:

- Standard audit never applies an aggregate prior implicitly.
- Private 100k/1M analysis bodies, paths, and exact priors never reach public artifacts.
- Calibration suggestions do not automatically change pattern or profile weights.
- Completion does not prove benchmark validity or general performance.

### 12. CF7: Completion Harness / Release Closure

Implementation slices:

- CF7.1: Add Rust `xtask completion --format json` and `seiri.completion.v1`.
- CF7.2: Register fmt, workspace tests, clippy, MSRV, schema, privacy, self-audit, plugin smoke, and fuzz corpus as blocking checks.
- CF7.3: Generate Windows / Linux bundles and checksums as CI artifacts.
- CF7.4: Synchronize README, docs topology, self-audit, release docs, CHANGELOG, and plugin skill.
- CF7.5: Record manual acceptance from a fresh plugin cache and new Codex thread against an unrelated repository.

Completion conditions:

- Every blocking check passes against the same commit.
- Keep the state `incomplete` unless Windows/Linux runtime manifests, SHA-256 values, and standalone-smoke evidence are all present.
- A completion record cannot be generated after skipping a blocking check.
- Final state is `ready_for_git` or `incomplete`; release and Git operations are not automatic.

### 13. Dependency Order And Stop Conditions

The implementation order is `CF0 -> CF1 -> CF2 -> CF3 -> CF4 -> CF5 -> CF6 -> CF7`. CF2-CF4 modify the same semantic surface and do not run in parallel.

Stop the affected slice when any of the following holds.

- The public-schema owner or migration is unresolved.
- User changes overlap owned files and cannot be integrated safely.
- Partial coverage produces false absence.
- Private data or credentials reach a public or ledger surface.
- A hidden fallback converts runtime failure into success.
- A blocking verification cannot run in the environment.

Stopping does not imply automatic rollback of every change. RCBP-v1 records changes and residuals and hands the run off as `incomplete`.

### 14. Final Acceptance

RepoSeiri 1.0.0 is a completion candidate only when all of the following hold on one commit.

1. Windows / Linux standalone plugin smoke succeeds against an unrelated repository.
2. Missing binaries and schema mismatches fail non-zero.
3. Core document roles receive priority under the default budget.
4. Route presence, condition, content coverage, and profile affinity remain separate.
5. RepoSeiri self-audit has neither the known automation contradiction nor the all-profile tie.
6. Full regression, clippy, MSRV, privacy, and fuzz corpus pass.
7. Private analysis is absent from the repository, bundle, logs, and reports.
8. Japanese/English docs, CLI help, schemas, and plugin skill show the same versions and boundaries.
9. The completion record has no skipped blocking check.
10. The maintainer approves release manually.
