# Migration v3

## 日本語

Roadmap v9-v10は、意味上の互換性がないportable/completion/runtime contractを新しいschemaへ移し、wire名を維持する意味変更をclosed semantic revisionで識別します。`seiri.analysis.v2`、`seiri.patch-plan.v2`、`seiri.codex.v2`、`seiri.error.v1`は維持します。

| 旧contract | 現行contract | 必要な変更 |
| --- | --- | --- |
| `seiri.portable-audit.v1` | `seiri.portable-audit.v2` | portable recordの`evidence_ids`を`evidence`内のidentity/state/occurrence fingerprintへ置換する |
| `seiri.audit-delta.v1` | `seiri.audit-delta.v2` | regression/improvement evidenceをstable fingerprintとして読む |
| `seiri.completion.v2` | `seiri.completion.v3` | pre/post source binding、source-bound host manifest、holdout report、5種のclaim matrixを要求する |
| `reposeiri.runtime-manifest.v1/v2` | `reposeiri.runtime-manifest.v3` | bundle metadata、source digest、Cargo.lock digest、binary SHA-256、contract、全semantic revisionを検証する |
| `seiri.patch-planner.v3` | `seiri.patch-planner.v4` | 3層evidence fingerprintをdecision basisとして読む |

stable identityはdomainとfield countを持つframed SHA-256へ移りました。patch baseだけは過去のFNV digestをdecodeできますが、新しいFNV digestは生成しません。portable document identityはrepository-relative pathとsemantic contentから作り、host absolute path、document ordinal、走査順序を含めません。

Markdown parserはvisible proseをcode fence、indented code、inline code、HTML comment、raw codeから分離します。wording/route analyzerはraw source substringを直接探さず、この同じevent streamを使います。意味変更を取り込むconsumerは`contract.semantic_revisions`の`markdown_parser`、`document_consistency`、`calibration`を含む全keyを検証してください。

private calibrationのraw content hashは公開されません。比較が必要な入力は、秘密を含まない`opaque_revision`を入力所有者が明示します。revisionがないlocal-private snapshotは`unknown_private_binding`で比較不能です。calibration outputの旧confidence bucketは`local_support_tier`、`sample_size`、明示したintervalへ置き換わりました。

calibration semantic revision v4は`seiri.calibration-corpus.v1`と`seiri.calibration-holdout.v1`を追加します。公開synthetic holdoutが最低sample数を満たさない場合、metricsが完全一致しても`insufficient_sample`です。completionは`ready_for_git`をrequired local verificationだけに限定し、host verification、calibration、manual policy、`evidence_complete`を別fieldで扱います。

旧schema、旧field、旧semantic revisionへのfallbackはありません。移行時にsource body、exact prior、private pathを公開artifactへコピーしないでください。

---

## English

Roadmaps v9-v10 move semantically incompatible portable, completion, and runtime contracts to new schemas and identify meaning changes inside retained wire names through a closed semantic-revision set. `seiri.analysis.v2`, `seiri.patch-plan.v2`, `seiri.codex.v2`, and `seiri.error.v1` remain unchanged.

| Previous contract | Current contract | Required change |
| --- | --- | --- |
| `seiri.portable-audit.v1` | `seiri.portable-audit.v2` | Replace portable `evidence_ids` with identity/state/occurrence fingerprints under `evidence` |
| `seiri.audit-delta.v1` | `seiri.audit-delta.v2` | Read regression and improvement evidence as stable fingerprints |
| `seiri.completion.v2` | `seiri.completion.v3` | Require pre/post source binding, source-bound host manifests, a holdout report, and the five-kind claim matrix |
| `reposeiri.runtime-manifest.v1/v2` | `reposeiri.runtime-manifest.v3` | Validate bundle metadata, source digest, Cargo.lock digest, binary SHA-256, contract, and every semantic revision |
| `seiri.patch-planner.v3` | `seiri.patch-planner.v4` | Read three-layer evidence fingerprints in the decision basis |

Stable identities now use framed SHA-256 with a domain and field count. Only patch bases can decode a historical FNV digest; new FNV digests are not produced. Portable document identity derives from repository-relative paths and semantic content, without host absolute paths, document ordinals, or traversal order.

The Markdown parser separates visible prose from code fences, indented code, inline code, HTML comments, and raw code. Wording and route analyzers consume that shared event stream instead of scanning raw source substrings. Consumers adopting the meaning change must validate every `contract.semantic_revisions` key, including `markdown_parser`, `document_consistency`, and `calibration`.

RepoSeiri no longer exposes a raw-content hash for private calibration. When comparison is required, the input owner supplies a non-secret `opaque_revision`. A local-private snapshot without that revision is incomparable with `unknown_private_binding`. Calibration output replaces the old confidence bucket with `local_support_tier`, `sample_size`, and an explicitly named interval.

Calibration semantic revision v4 adds `seiri.calibration-corpus.v1` and `seiri.calibration-holdout.v1`. A public synthetic holdout below its minimum sample count remains `insufficient_sample` even when its fixture metrics are perfect. Completion limits `ready_for_git` to required local verification and reports host verification, calibration, manual policy, and `evidence_complete` separately.

There is no fallback to previous schemas, fields, or semantic revisions. Do not copy source bodies, exact priors, or private paths into public artifacts during migration.
