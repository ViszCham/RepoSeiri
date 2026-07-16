# R9-SIP-v1: Roadmap v9 Sequential Implementation Protocol

## 日本語

R9-SIP-v1はRoadmap v9のSI0-SI12を依存順に実行する手順です。一括指示はfile mutationとlocal verificationを許可しますが、commit、push、merge、release、公開、plugin再インストールは許可しません。

1. SI0でbaseline、dirty state、既存failure、環境制約を記録します。
2. SI1-SI4でparser、path、classification、facetを実装し、targeted checkを行います。
3. SI5-SI9でprivate boundary、stable digest、portable delta、planner、schemaを実装します。
4. SI10でpre/post completion bindingとhost receipt bindingを実装します。
5. SI11でpositive、negative、hostile、privacy regressionを追加します。
6. SI12でfmt、workspace check、test、clippy、MSRV、self-audit、privacy、diff hygieneを順に実行します。
7. checkが環境に遮断された場合は`ENVIRONMENT_BLOCKED`としてcommand、OS error、未実行範囲を記録し、`EVIDENCE_COMPLETE`を禁止します。
8. 最終出力はimplemented facts、verification performed、remaining uncertainty、blocked checksを分離します。

---

## English

R9-SIP-v1 executes Roadmap v9 SI0-SI12 in dependency order. A batch instruction authorizes file mutation and local verification, but does not authorize commit, push, merge, release, publication, or plugin reinstallation.

1. SI0 records the baseline, dirty state, existing failures, and environment constraints.
2. SI1-SI4 implement parser, path, classification, and facet work, followed by targeted checks.
3. SI5-SI9 implement the private boundary, stable digests, portable delta, planner, and schemas.
4. SI10 implements pre/post completion binding and host-receipt binding.
5. SI11 adds positive, negative, hostile, and privacy regressions.
6. SI12 runs format, workspace check, tests, clippy, MSRV, self-audit, privacy, and diff hygiene in order.
7. When the environment blocks a check, record `ENVIRONMENT_BLOCKED` with the command, OS error, and unexecuted scope; prohibit `EVIDENCE_COMPLETE`.
8. The final output separates implemented facts, verification performed, remaining uncertainty, and blocked checks.
