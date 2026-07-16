# Migration v3

## 日本語

Roadmap v9は意味上の互換性がないportable/completion contractを新しいschemaへ移します。`seiri.analysis.v2`、`seiri.patch-plan.v2`、`seiri.codex.v2`、`seiri.error.v1`は維持します。

| 旧contract | 現行contract | 必要な変更 |
| --- | --- | --- |
| `seiri.portable-audit.v1` | `seiri.portable-audit.v2` | portable recordの`evidence_ids`を`evidence`内のidentity/state/occurrence fingerprintへ置換する |
| `seiri.audit-delta.v1` | `seiri.audit-delta.v2` | regression/improvement evidenceをstable fingerprintとして読む |
| `seiri.completion.v2` | `seiri.completion.v3` | pre/post source bindingとsource-bound host manifestを要求する |
| `reposeiri.runtime-manifest.v1` | `reposeiri.runtime-manifest.v2` | `source_digest`と`cargo_lock_digest`を検証する |
| `seiri.patch-planner.v3` | `seiri.patch-planner.v4` | 3層evidence fingerprintをdecision basisとして読む |

private calibrationのraw content hashは公開されません。比較が必要な入力は、秘密を含まない`opaque_revision`を入力所有者が明示します。revisionがないlocal-private snapshotは`unknown_private_binding`で比較不能です。

旧schema、旧field、旧semantic revisionへのfallbackはありません。移行時にsource body、exact prior、private pathを公開artifactへコピーしないでください。

---

## English

Roadmap v9 moves semantically incompatible portable and completion contracts to new schemas. `seiri.analysis.v2`, `seiri.patch-plan.v2`, `seiri.codex.v2`, and `seiri.error.v1` remain unchanged.

| Previous contract | Current contract | Required change |
| --- | --- | --- |
| `seiri.portable-audit.v1` | `seiri.portable-audit.v2` | Replace portable `evidence_ids` with identity/state/occurrence fingerprints under `evidence` |
| `seiri.audit-delta.v1` | `seiri.audit-delta.v2` | Read regression and improvement evidence as stable fingerprints |
| `seiri.completion.v2` | `seiri.completion.v3` | Require pre/post source binding and source-bound host manifests |
| `reposeiri.runtime-manifest.v1` | `reposeiri.runtime-manifest.v2` | Validate `source_digest` and `cargo_lock_digest` |
| `seiri.patch-planner.v3` | `seiri.patch-planner.v4` | Read three-layer evidence fingerprints in the decision basis |

RepoSeiri no longer exposes a raw-content hash for private calibration. When comparison is required, the input owner supplies a non-secret `opaque_revision`. A local-private snapshot without that revision is incomparable with `unknown_private_binding`.

There is no fallback to previous schemas, fields, or semantic revisions. Do not copy source bodies, exact priors, or private paths into public artifacts during migration.
