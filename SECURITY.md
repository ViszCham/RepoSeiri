# Security Policy

## 日本語

RepoSeiri の security route は、脆弱性の疑いを公開 issue に直接書かず、まず非公開の報告経路へ送るためのものです。

### 報告方法

- 推奨経路は GitHub の private vulnerability reporting または GitHub Security Advisory です。
- GitHub の非公開報告ボタンが使えない場合は、公開 issue に脆弱性の詳細、再現手順、攻撃手順、秘密情報を書かないでください。
- 非公開経路が使えない場合は、公開 issue では「非公開で報告したい security concern がある」ことだけを伝え、詳細は maintainer から private route が示された後に共有してください。

### 対象範囲

- RepoSeiri の Rust crates、CLI、Codex plugin source、repository analysis logic が対象です。
- `fixtures/` 配下のファイルはテスト用の入力であり、実プロジェクトの security policy、license、support route として扱いません。
- 第三者サービス、GitHub platform 自体、またはユーザーが監査対象として渡した外部リポジトリの脆弱性は、それぞれの管理者へ報告してください。

### 期待値と境界

- 現時点では固定の response SLA を約束しません。
- RepoSeiri は監査補助ツールであり、security outcome、完全性、脆弱性不在を保証しません。
- Security policy の内容、受付範囲、SLA、連絡先は maintainer decision です。変更する場合は review を通してください。

## English

RepoSeiri's security route exists so suspected vulnerabilities are not posted directly in public issues and are sent to a private reporting path first.

### Reporting

- The preferred route is GitHub private vulnerability reporting or a GitHub Security Advisory.
- If the private GitHub reporting button is unavailable, do not post vulnerability details, reproduction steps, exploit steps, or secrets in a public issue.
- If no private route is available, use a public issue only to say that you have a security concern to report privately, then share details only after a maintainer provides a private route.

### Scope

- The scope includes RepoSeiri Rust crates, the CLI, the Codex plugin source, and repository analysis logic.
- Files under `fixtures/` are test inputs and are not treated as the real project's security policy, license, or support route.
- Vulnerabilities in third-party services, the GitHub platform itself, or external repositories passed to RepoSeiri as audit targets should be reported to their respective maintainers.

### Expectations And Boundaries

- No fixed response SLA is promised at this stage.
- RepoSeiri is an audit assistance tool and does not guarantee security outcomes, completeness, or the absence of vulnerabilities.
- Security policy content, intake scope, SLA, and contact routes are maintainer decisions. Changes should go through review.
