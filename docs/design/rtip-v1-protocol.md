# RTIP-v1 Execution Protocol

## 日本語

RTIP-v1 は「プロトコルを実行してください」という明示指示を、Roadmap v8 の TI0-TI11 へ順次分解する実装契約です。

### Authority

| Authority | 既定値 | 内容 |
| --- | --- | --- |
| AnalysisAuthority | true | source、test、docs、local state の読取 |
| MutationAuthority | true | Roadmap v8 の範囲内の file edit |
| VerificationAuthority | true | local build、test、lint、audit |
| CommitAuthority | false | stage / commit は別指示が必要 |
| PushAuthority | false | push は別指示が必要 |
| MergeAuthority | false | merge は別指示が必要 |
| ReleaseAuthority | false | tag / release / publish は別指示が必要 |
| InstallAuthority | false | plugin install / cache update は別指示が必要 |
| VisibilityAuthority | false | public / private 変更は別指示が必要 |

### Execution

1. TI0でHEAD、branch、dirty state、toolchain、baseline testを記録します。
2. TI1-TI11を依存順に小さなsliceへ分け、sliceごとにformat / targeted test / compileを実行します。
3. fail-closed、privacy、no-write、no-network、authority separationを回帰条件として保ちます。
4. 意味変更はschema名だけで隠さず、semantic revisionとmachine projectionへ記録します。
5. 最終検証に失敗した場合は`incomplete`とし、成功を推測しません。
6. Git / GitHub / install操作は実装完了後も自動実行しません。

### Stop Conditions

- private analysis本文、local source path、exact prior、credentialをtracked artifactへ移す必要がある。
- 公開schemaの破壊的変更が、固定されたversion方針では表現できない。
- repository外write、network、GitHub mutationが実装に必要になる。
- user変更と両立しない差分があり、安全に統合できない。

---

## English

RTIP-v1 is the implementation contract that decomposes an explicit “execute the protocol” instruction into Roadmap v8 blocks TI0-TI11.

### Authority

| Authority | Default | Meaning |
| --- | --- | --- |
| AnalysisAuthority | true | Read source, tests, docs, and local state |
| MutationAuthority | true | Edit files within Roadmap v8 scope |
| VerificationAuthority | true | Run local builds, tests, lint, and audit |
| CommitAuthority | false | Staging and commits require a separate instruction |
| PushAuthority | false | Push requires a separate instruction |
| MergeAuthority | false | Merge requires a separate instruction |
| ReleaseAuthority | false | Tags, releases, and publication require a separate instruction |
| InstallAuthority | false | Plugin installation and cache updates require a separate instruction |
| VisibilityAuthority | false | Public/private changes require a separate instruction |

### Execution

1. TI0 records HEAD, branch, dirty state, toolchains, and baseline tests.
2. Split TI1-TI11 into small dependency-ordered slices and run formatting, targeted tests, and compilation for each slice.
3. Preserve fail-closed, privacy, no-write, no-network, and authority-separation regression conditions.
4. Record semantic changes in semantic revisions and machine projections instead of hiding them behind an unchanged schema name.
5. Report `incomplete` when final verification fails; do not infer success.
6. Do not automatically perform Git, GitHub, or installation operations after implementation.

### Stop Conditions

- Implementation would require moving private analysis bodies, local source paths, exact priors, or credentials into tracked artifacts.
- A breaking public-schema change cannot be represented within the fixed version policy.
- Implementation requires an outside-repository write, network access, or GitHub mutation.
- Existing user changes cannot be integrated safely.
