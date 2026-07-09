# SUPPORT

## 日本語

RepoSeiri の support route は、質問、再現可能な不具合、機能提案、セキュリティ報告を分けて受け取るための入口です。この文書は、どこに何を書くかを決めるための routing surface であり、応答時間や解決を保証するものではありません。

### 入口

| 目的 | Route |
| --- | --- |
| 再現可能な不具合 | [.github/ISSUE_TEMPLATE/bug_report.yml](.github/ISSUE_TEMPLATE/bug_report.yml) |
| 機能提案または route 改善 | [.github/ISSUE_TEMPLATE/feature_request.yml](.github/ISSUE_TEMPLATE/feature_request.yml) |
| 質問、使い方、設計確認 | [.github/ISSUE_TEMPLATE/question.yml](.github/ISSUE_TEMPLATE/question.yml) |
| セキュリティ脆弱性 | [SECURITY.md](SECURITY.md) |
| contribution 手順 | [CONTRIBUTING.md](CONTRIBUTING.md) |

### Support 対象

- RepoSeiri CLI の実行方法、出力の読み方、route state の解釈。
- Rust workspace、fixture、pattern registry、calibration ingest、Codex adapter の挙動。
- README、support、contributing、security、release、automation などの repository route 設計。
- 分析データから導いた route priority の意味と限界。

### Support 対象外

- 公開 issue での未修正セキュリティ脆弱性の詳細共有。
- RepoSeiri のスコアを人気、信頼、安全性、品質、法務適合の保証として扱うこと。
- メンテナが確認していない自動方針決定、GitHub write、PR 作成、branch 作成。
- 外部サービス、GitHub 権限、Codex host の状態に依存する問題の保証付き解決。

### 情報の書き方

- 再現可能な問題は、実行した command、対象 repository path、期待結果、実際の結果を分けて書いてください。
- RepoSeiri の出力を貼る場合は、`seiri audit` または `seiri codex` の該当部分だけに絞ってください。
- セキュリティに関わる可能性がある場合は公開 issue を作らず、[SECURITY.md](SECURITY.md) の route を使ってください。
- 大きな設計変更は、先に issue で目的、対象 route、検証方法を確認してください。

### 応答境界

この repository は固定 SLA を置きません。Support route は triage を助けるための構造であり、回答、修正、採用、merge、release を約束するものではありません。

---

## English

The RepoSeiri support route separates questions, reproducible defects, feature proposals, and security reports. This document is a routing surface for deciding where each kind of request belongs; it does not guarantee response time or resolution.

### Entry Points

| Purpose | Route |
| --- | --- |
| Reproducible defect | [.github/ISSUE_TEMPLATE/bug_report.yml](.github/ISSUE_TEMPLATE/bug_report.yml) |
| Feature proposal or route improvement | [.github/ISSUE_TEMPLATE/feature_request.yml](.github/ISSUE_TEMPLATE/feature_request.yml) |
| Question, usage, or design check | [.github/ISSUE_TEMPLATE/question.yml](.github/ISSUE_TEMPLATE/question.yml) |
| Security vulnerability | [SECURITY.md](SECURITY.md) |
| Contribution process | [CONTRIBUTING.md](CONTRIBUTING.md) |

### In Scope

- How to run the RepoSeiri CLI, read output, and interpret route states.
- Behavior of the Rust workspace, fixtures, pattern registry, calibration ingest, and Codex adapter.
- Repository route design for README, support, contributing, security, release, and automation.
- The meaning and limits of route priorities derived from analysis data.

### Out of Scope

- Sharing details of an unfixed security vulnerability in a public issue.
- Treating a RepoSeiri score as a guarantee of popularity, trust, safety, quality, or legal fitness.
- Automatic policy decisions, GitHub writes, PR creation, or branch creation that maintainers have not confirmed.
- Guaranteed resolution for problems that depend on external services, GitHub permissions, or Codex host state.

### How To Write A Request

- For reproducible problems, separate the command you ran, target repository path, expected result, and actual result.
- When pasting RepoSeiri output, include only the relevant part of `seiri audit` or `seiri codex`.
- If the issue may involve security, do not open a public issue; use the route in [SECURITY.md](SECURITY.md).
- For large design changes, open an issue first and describe the goal, affected route, and verification method.

### Response Boundary

This repository does not set a fixed SLA. The support route structures triage, but it does not promise an answer, fix, acceptance, merge, or release.
