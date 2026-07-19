# Lifecycle Boundary

## 日本語

RepoSeiriの現行tool/package versionは`1.0.0`で、machine-readable source contractは`seiri.contract.v4`です。v2 wire内の意味は22個の`contract.semantic_revisions`が所有します。対応するCLI、schema、plugin bundleの変更は[Changelog](../CHANGELOG.md)、[Migration v4](migration-v4.md)、[Release Process](release.md)に記録します。

この文書は観測できる現行状態と人間の判断境界を示します。将来の保守期間、support SLA、次回release日、非推奨日、archive日を約束しません。

### 現行境界

- breaking changeはmigration noteまたはmajor version境界で扱います。
- release、deprecation、archive、後継版の判断はmaintainerの明示判断です。
- CI、RepoSeiri audit、completion recordはreview evidenceであり、継続保守やrelease可否を自動決定しません。
- 変更時はChangelogと該当migration documentを更新します。

---

## English

RepoSeiri's current tool/package version is `1.0.0`, while its machine-readable source contract is `seiri.contract.v4`. The 22 `contract.semantic_revisions` own meaning inside the v2 wires. Changes to the corresponding CLI, schemas, and plugin bundle are recorded in the [Changelog](../CHANGELOG.md), [Migration v4](migration-v4.md), and [Release Process](release.md).

This document states observable current status and the human decision boundary. It does not promise a future maintenance duration, support SLA, next release date, deprecation date, or archival date.

### Current Boundary

- Breaking changes use a migration note or a major-version boundary.
- Release, deprecation, archival, and successor decisions require an explicit maintainer decision.
- CI, RepoSeiri audits, and completion records are review evidence; they do not decide continued maintenance or release approval automatically.
- Changes update the Changelog and the relevant migration document.
