# Migration v4

## 日本語

R11は`seiri.analysis.v2`、`seiri.patch-plan.v2`、`seiri.codex.v2`、`seiri.error.v1`のwire名を維持しながら、source ownership、route content判定、planner表示の意味を変更します。この変更を黙って受理させないため、contract manifestを`seiri.contract.v4`へ上げ、semantic revision集合を17個から22個へ閉じました。

| 項目 | 旧revision | 現行revision |
| --- | --- | --- |
| source session | `seiri.source-session.v1` | `seiri.source-session.v2` |
| stable digest | `seiri.stable-digest.v2` | `seiri.stable-digest.v3` |
| content slots | `seiri.content-slots.v2` | `seiri.content-slots.v3` |
| audit delta | `seiri.audit-delta-semantics.v3` | `seiri.audit-delta-semantics.v4` |
| patch planner | `seiri.patch-planner.v4` | `seiri.patch-planner.v5` |
| completion | `seiri.completion-semantics.v4` | `seiri.completion-semantics.v5` |

新しい必須keyは`semantic_index`、`language_topology`、`route_assessment`、`rule_registry`、`review_projection`です。launcherとruntime manifestは全22 keyの名前、値、個数を検証します。旧contractまたは一部だけのrevision集合へのfallbackはありません。

`source-session.v2`ではMarkdownとGitHub構造文書parserが同じbounded bytesを使い、plannerのfilesystem再読込を廃止しました。`stable-digest.v3`はsource bodyの公開を行わず、明示的にframe化したrepository-relative file、source、scope、Git observationをhashします。

`content-slots.v3`ではvisible Markdown eventの単一semantic indexを使い、code fence、inline code、HTML commentをroute contentとして数えません。単語境界を検査するため、`licensed`や`sublicense`は`license` markerではありません。LICENSE fileの存在とREADME内のlicense routeも別判定です。

`audit-delta-semantics.v4`ではevidence identityとstateをsemantic record digestへ使い、line、column、byte offsetを含むoccurrence digestは位置情報として別に保持します。内容とstateが同じまま行だけ移動してもsemantic artifactを`Changed`へしません。source sessionとdocument digestによるsource変更の観測は維持します。

`patch-planner.v5`はREADMEの言語topologyを検査します。日英並列READMEには両言語のsource-bound editを提案し、曖昧なmixed-language構造はholdします。存在しないtargetは自動生成せず、suggestible skeleton reviewまたはmaintainer decisionとして分類します。

`completion-semantics.v5`ではlocal invocationの`ready_for_git`をrequired local checkに限定したまま、`--host-evidence`が明示された実行ではWindows/Linux両receiptをblocking requirementとして扱います。不足または不一致があれば`implemented_with_blocked_evidence`とnon-zero exitへ下げます。calibrationと`evidence_complete`は別claimのままです。

CLI error envelopeは`seiri.error.v1`のままですが、`audit_failed`一種類ではなく、`document_index_failed`、`analysis_integrity_failed`など原因別codeを返します。公開artifactへsource body、host absolute path、private calibration path、exact priorを追加してはいけません。

---

## English

R11 changes source ownership, route-content evaluation, and planner rendering while retaining the `seiri.analysis.v2`, `seiri.patch-plan.v2`, `seiri.codex.v2`, and `seiri.error.v1` wire names. To prevent silent adoption, the contract manifest moves to `seiri.contract.v4` and closes the semantic-revision set at 22 keys instead of 17.

| Area | Previous revision | Current revision |
| --- | --- | --- |
| source session | `seiri.source-session.v1` | `seiri.source-session.v2` |
| stable digest | `seiri.stable-digest.v2` | `seiri.stable-digest.v3` |
| content slots | `seiri.content-slots.v2` | `seiri.content-slots.v3` |
| audit delta | `seiri.audit-delta-semantics.v3` | `seiri.audit-delta-semantics.v4` |
| patch planner | `seiri.patch-planner.v4` | `seiri.patch-planner.v5` |
| completion | `seiri.completion-semantics.v4` | `seiri.completion-semantics.v5` |

The new required keys are `semantic_index`, `language_topology`, `route_assessment`, `rule_registry`, and `review_projection`. Launchers and runtime manifests validate the names, values, and count of all 22 keys. There is no fallback to an old contract or a partial revision set.

In `source-session.v2`, Markdown and GitHub structured-document parsers use the same bounded bytes and the planner no longer rereads the filesystem. `stable-digest.v3` does not publish source bodies; it hashes explicitly framed repository-relative file, source, scope, and Git observations.

`content-slots.v3` uses one semantic index over visible Markdown events and does not count code fences, inline code, or HTML comments as route content. Word boundaries mean that `licensed` and `sublicense` are not `license` markers. LICENSE-file presence and the README license route are separate assessments.

`audit-delta-semantics.v4` uses evidence identity and state in semantic record digests while retaining line, column, and byte offsets separately in occurrence digests. Moving unchanged content to another line no longer marks the semantic artifact as `Changed`. Source changes remain observable through source-session and document digests.

`patch-planner.v5` checks README language topology. It proposes source-bound edits in both languages for parallel Japanese/English READMEs and holds ambiguous mixed-language structures. It does not generate missing targets; those become suggestible skeleton reviews or maintainer decisions.

`completion-semantics.v5` keeps local `ready_for_git` scoped to required local checks, but an invocation that explicitly supplies `--host-evidence` treats both Windows and Linux receipts as blocking requirements. Missing or mismatched receipts downgrade the state to `implemented_with_blocked_evidence` and return a non-zero exit. Calibration and `evidence_complete` remain separate claims.

The CLI error envelope remains `seiri.error.v1`, but it now emits cause-specific codes such as `document_index_failed` and `analysis_integrity_failed` instead of one `audit_failed` code. Public artifacts must not gain source bodies, host absolute paths, private calibration paths, or exact priors.
