# Design Documentation

## 日本語

この subindex は RepoSeiri の設計文書を、長期前提、分析モデル、実装 roadmap に分けます。README はアプリの入口に限定し、低レイヤ Rust 契約と実装判断はここから辿ります。

### Reading order

| Step | Document | Owns |
| --- | --- | --- |
| 1 | [Repository Trust Graph](repository-trust-graph.md) | product boundary、repository route、claim boundary の長期前提 |
| 2 | [Baseline And Profiles](baseline-and-profiles.md) | common pattern、目的別 profile、calibration input の扱い |
| 3 | [Roadmap v5: Legacy Removal](roadmap-v5-legacy-removal.md) | 0.2.0 canonical Rust architectureとlegacy removalの実装記録 |
| 4 | [Roadmap v6: Completion](roadmap-v6-completion.md) | 1.0.0 CF0-CF7実装記録、完成条件、停止条件 |
| 5 | [Roadmap v7: Calibrated Assertion](roadmap-v7-calibrated-assertion.md) | evidence-backed claim、boundary relevance、underclaim lossの現行実装契約 |
| 6 | [RCBP-v1](completion-batch-protocol.md) | 一括指示を内部sliceへ分解するCodex実行契約 |
| 7 | [RCBP-v1 Template](rcbp-v1-template.json) | block依存、authority既定値、stateの機械可読正本 |

### Authority

- Roadmap v5は0.2.0 architecture/legacy removalの実装記録です。
- 現行1.0.0 source contract、CF0-CF7の実装責務、完成条件はRoadmap v6を参照します。
- evidence-backed claimの強さ、claim-local boundary、underclaim lossはRoadmap v7を参照します。
- RCBP-v1はRoadmap v6の実行方法を所有し、製品semantics、Git権限、release判断を上書きしません。
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
| 3 | [Roadmap v5: Legacy Removal](roadmap-v5-legacy-removal.md) | Implementation record for the canonical 0.2.0 Rust architecture and legacy removal |
| 4 | [Roadmap v6: Completion](roadmap-v6-completion.md) | 1.0.0 CF0-CF7 implementation record, completion conditions, and stop conditions |
| 5 | [Roadmap v7: Calibrated Assertion](roadmap-v7-calibrated-assertion.md) | Current implementation contract for evidence-backed claims, boundary relevance, and underclaim loss |
| 6 | [RCBP-v1](completion-batch-protocol.md) | Codex execution contract that decomposes one batch instruction into internal slices |
| 7 | [RCBP-v1 Template](rcbp-v1-template.json) | Machine-readable authority for block dependencies, authority defaults, and states |

### Authority

- Roadmap v5 is the implementation record for the 0.2.0 architecture and legacy removal.
- Use Roadmap v6 for the current 1.0.0 source contract, CF0-CF7 implementation responsibilities, and completion conditions.
- Use Roadmap v7 for evidence-backed claim strength, claim-local boundaries, and underclaim loss.
- RCBP-v1 owns execution of Roadmap v6; it does not override product semantics, Git authority, or release decisions.
- Trust Graph and Baseline And Profiles own premises and the analysis model, but do not override current symbols or commands.
- Git history and the changelog record changes; they are not current implementation instructions.
- Private analysis data and private calibration bodies do not move into design docs, fixtures, reports, or commits.

### Update Rules

1. Before adding a design document, check whether its responsibility belongs in one of the existing three documents.
2. Keep exactly one implementation roadmap authoritative at a time.
3. Do not mix plans, implemented facts, manual decisions, and verification evidence.
4. Keep equivalent decisions and boundaries in the Japanese-first and English-second halves of major documents.
5. Do not turn RepoSeiri output into guarantees of popularity, trust, security, quality, legal fitness, or publication readiness.
