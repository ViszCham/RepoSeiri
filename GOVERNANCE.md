# Governance

## 日本語

RepoSeiri は個人使用目的の Rust コーディング練習リポジトリです。この文書は、公開前後に誤解されやすい governance 境界を明確にするためのものです。

### 意思決定

- 主要な方針、release、security route、license、repository visibility の変更は repository owner が判断します。
- RepoSeiri の audit、score、route state は判断材料であり、方針を自動決定しません。
- Codex plugin や自動化は、human review を置き換えません。

### 外部 contribution

- issue や PR を受け取る入口はありますが、採用、review、merge、release を保証しません。
- 大きな設計変更、public policy、security、license、ownership、automation の変更は、先に issue または明示確認を必要とします。
- 未修正の security detail は public issue / PR に書かず、`SECURITY.md` の route を使います。

### 公開前境界

- このリポジトリは private のまま整理します。visibility 変更は別の明示タスクです。
- 公開する場合も、個人使用・コーディング練習目的であることを維持します。
- GitHub description、topics、README、docs が過剰な製品主張になっていないかを確認します。

---

## English

RepoSeiri is a personal-use Rust coding practice repository. This document clarifies governance boundaries that could be misunderstood before or after publication.

### Decisions

- Major policy, release, security route, license, and repository visibility changes are decided by the repository owner.
- RepoSeiri audit results, scores, and route states are decision inputs, not automatic policy decisions.
- The Codex plugin and automation do not replace human review.

### External Contributions

- Issue and PR entry points exist, but acceptance, review, merge, and release are not guaranteed.
- Large design changes and changes to public policy, security, license, ownership, or automation require an issue or explicit confirmation first.
- Do not place unfixed security details in public issues or PRs; use the route in `SECURITY.md`.

### Pre-Publication Boundary

- This repository is organized while it remains private. Visibility changes are separate explicit tasks.
- If it is made public, it remains scoped as personal-use coding practice work.
- Check that the GitHub description, topics, README, and docs do not overstate the repository as a product.
