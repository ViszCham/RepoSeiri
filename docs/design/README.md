# Design Documentation

## 日本語

この subindex は RepoSeiri の設計文書を、長期前提、分析モデル、実装 roadmap に分けます。README はアプリの入口に限定し、低レイヤ Rust 契約と実装判断はここから辿ります。

### Reading order

| Step | Document | Owns |
| --- | --- | --- |
| 1 | [Repository Trust Graph](repository-trust-graph.md) | product boundary、repository route、claim boundary の長期前提 |
| 2 | [Baseline And Profiles](baseline-and-profiles.md) | common pattern、目的別 profile、calibration input の扱い |
| 3 | [Roadmap v5: Legacy Removal](roadmap-v5-legacy-removal.md) | 0.2.0 canonical Rust architecture、削除判断、実装 block、完了条件 |

### Authority

- 現行実装の schema、CLI、planner、Codex surface は Roadmap v5 を正とします。
- Trust Graph と Baseline And Profiles は前提と分析モデルを所有しますが、現行 symbol や command を上書きしません。
- Git history と changelog は変更履歴であり、現在の実装指示ではありません。
- private analysis data と private calibration body は設計 docs、fixture、report、commit に移しません。

### Update rules

1. 新しい設計文書を追加する前に、既存3文書の責務へ統合できないか確認します。
2. 実装 roadmap は同時に一つだけを正とします。
3. plan、implemented fact、manual decision、verification evidence を混在させません。
4. 主要文書は日本語前半、英語後半で同じ判断と境界を維持します。
5. 人気、信頼、安全性、品質、法的適合性、公開準備完了を RepoSeiri の出力から保証しません。

---

## English

This subindex separates RepoSeiri design documentation into long-term premises, the analysis model, and the implementation roadmap. The README remains the application entry point; low-level Rust contracts and implementation decisions are routed from here.

### Reading Order

| Step | Document | Owns |
| --- | --- | --- |
| 1 | [Repository Trust Graph](repository-trust-graph.md) | Long-term premises for product boundaries, repository routes, and claim boundaries |
| 2 | [Baseline And Profiles](baseline-and-profiles.md) | Common patterns, purpose-specific profiles, and calibration-input handling |
| 3 | [Roadmap v5: Legacy Removal](roadmap-v5-legacy-removal.md) | Canonical 0.2.0 Rust architecture, removal decisions, implementation blocks, and completion criteria |

### Authority

- Roadmap v5 is authoritative for the current implementation schema, CLI, planner, and Codex surface.
- Trust Graph and Baseline And Profiles own premises and the analysis model, but do not override current symbols or commands.
- Git history and the changelog record changes; they are not current implementation instructions.
- Private analysis data and private calibration bodies do not move into design docs, fixtures, reports, or commits.

### Update Rules

1. Before adding a design document, check whether its responsibility belongs in one of the existing three documents.
2. Keep exactly one implementation roadmap authoritative at a time.
3. Do not mix plans, implemented facts, manual decisions, and verification evidence.
4. Keep equivalent decisions and boundaries in the Japanese-first and English-second halves of major documents.
5. Do not turn RepoSeiri output into guarantees of popularity, trust, security, quality, legal fitness, or publication readiness.
