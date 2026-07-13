# v2移行 / v2 Migration

## 日本語

RepoSeiri 1.0は、`seiri.analysis.v2`、`seiri.patch-plan.v2`、`seiri.codex.v2`を唯一の現行wireとして使用します。v1入力、serde alias、schema selector、暗黙変換はありません。利用側はschema名と、DocumentIndex、route availability、profile purpose affinityのv2 fieldへ明示的に移行してください。

成功時は指定形式だけをstdoutへ出力します。実行時失敗は`seiri.error.v1`をstderrへ出力し、typed exit codeで終了します。部分coverageはabsenceとして扱いません。

## English

RepoSeiri 1.0 uses `seiri.analysis.v2`, `seiri.patch-plan.v2`, and `seiri.codex.v2` as its only active wires. There are no v1 inputs, serde aliases, schema selectors, or implicit conversions. Consumers must explicitly migrate the schema names and the v2 DocumentIndex, route-availability, and profile-purpose-affinity fields.

Success writes only the requested format to stdout. Runtime failures write `seiri.error.v1` to stderr and exit with a typed code. Partial coverage is never treated as absence.
