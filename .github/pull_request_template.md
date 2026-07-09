## 日本語

### 目的

<!-- 何を、なぜ変更するかを書いてください。 -->

### 対象 route

<!-- README / Docs / Quickstart / Support / Intake / Contributing / Security / Release / Governance / License / Automation / Ownership / Hygiene など。 -->

### 変更内容

- 

### 検証

実行したものに印を付け、必要なら結果を追記してください。

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo test --workspace`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo run --quiet -p seiri-cli -- audit --path . --profile library --format markdown`
- [ ] `cargo run --quiet -p seiri-cli -- codex --path . --profile library --format markdown`
- [ ] `git diff --check`

### RepoSeiri audit の差分

<!-- route state、profile score、safe/guarded/manual decision の変化を書いてください。 -->

### 境界

- [ ] 未修正のセキュリティ脆弱性の詳細を public PR に書いていません。
- [ ] RepoSeiri の出力を人気、信頼、安全性、品質、法務適合の保証として表現していません。
- [ ] 関係ない未コミット変更を巻き戻していません。
- [ ] 人間向け主要文書を変更した場合、日本語前半と英語後半に同じ内容を入れています。

---

## English

### Purpose

<!-- Describe what changes and why. -->

### Affected route

<!-- README / Docs / Quickstart / Support / Intake / Contributing / Security / Release / Governance / License / Automation / Ownership / Hygiene, or another route. -->

### Changes

- 

### Verification

Check what you ran and add results when useful.

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo test --workspace`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo run --quiet -p seiri-cli -- audit --path . --profile library --format markdown`
- [ ] `cargo run --quiet -p seiri-cli -- codex --path . --profile library --format markdown`
- [ ] `git diff --check`

### RepoSeiri audit delta

<!-- Describe route state, profile score, and safe/guarded/manual decision changes. -->

### Boundaries

- [ ] I did not include details of an unfixed security vulnerability in this public PR.
- [ ] I did not present RepoSeiri output as a guarantee of popularity, trust, safety, quality, or legal fitness.
- [ ] I did not revert unrelated uncommitted changes.
- [ ] If I changed major human-facing docs, I kept the same content in the Japanese first half and English second half.
