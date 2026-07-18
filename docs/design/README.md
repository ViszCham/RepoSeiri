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
| 6 | [Roadmap v8: Trust And Contract Integrity](roadmap-v8-trust-contract-integrity.md) | path、Markdown、claim、provenance、resource、completionの現行実装契約 |
| 7 | [RTIP-v1](rtip-v1-protocol.md) | Roadmap v8をTI0-TI11へ分解する実行契約 |
| 8 | [RTIP-v1 Template](rtip-v1-template.json) | TI block依存とauthority既定値の機械可読正本 |
| 9 | [RCBP-v1](completion-batch-protocol.md) | Roadmap v6完成batchの履歴実行契約 |
| 10 | [RCBP-v1 Template](rcbp-v1-template.json) | RCBP-v1の機械可読履歴 |
| 11 | [Roadmap v10: Closure And Product Integrity](roadmap-v10-closure-and-product-integrity.md) | coverage、bounded source、stable identity、extension、product、calibrationの現行改善契約 |
| 12 | [R10-SIP-v1](r10-sip-v1-protocol.md) | C0-C8を非対話で順次実装する現行実行契約 |
| 13 | [R10-SIP-v1 Template](r10-sip-v1-template.json) | unit依存、repair budget、authority、completionの機械可読正本 |

### Authority

- Roadmap v5は0.2.0 architecture/legacy removalの実装記録です。
- Roadmap v6は1.0.0 completion baselineの実装記録です。
- evidence-backed claimの強さ、claim-local boundary、underclaim lossはRoadmap v7を参照します。
- Roadmap v8、RTIP-v1、Roadmap v9、R9-SIP-v1は直前までの実装契約と履歴です。
- 現行の改善責務、completion条件、一括実装方法はRoadmap v10とR10-SIP-v1を参照します。
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

### Roadmap v9

- [Roadmap v9: Semantic Identity And Verification Closure](roadmap-v9-semantic-identity-verification-closure.md) は、semantic Markdown、path classification、private calibration、stable delta、completion v3の実装記録です。
- [R9-SIP-v1](r9-sip-v1-protocol.md) はRoadmap v9をSI0-SI12へ分割した履歴実行契約です。
- [R9-SIP-v1 Template](r9-sip-v1-template.json) はunit依存関係とauthority既定値の機械可読版です。
- Roadmap v8とRTIP-v1は直前のtrust contractと実装履歴です。Roadmap v9とR9-SIP-v1が同じ責任範囲の現行判断を上書きします。

### Roadmap v10

- [Roadmap v10: Closure And Product Integrity](roadmap-v10-closure-and-product-integrity.md) は、coverage、bounded source session、stable identity、contract、extension、Markdown、product surface、release evidence、calibrationの現行改善契約です。
- [R10-SIP-v1](r10-sip-v1-protocol.md) はC0-C8を内部sliceへ分解し、通常failureで対話停止せず最終reportまで進む現行実行契約です。
- [R10-SIP-v1 Template](r10-sip-v1-template.json) はunit依存、有限repair、blocked evidence、authority、completion predicateの機械可読正本です。
- Roadmap v9とR9-SIP-v1はsemantic identity/completion v3の実装記録です。Roadmap v10とR10-SIP-v1が重複する現行改善判断を上書きします。

---

## English

This subindex separates RepoSeiri design documentation into long-term premises, the analysis model, and the implementation roadmap. The README remains the application entry point; low-level Rust contracts and implementation decisions are routed from here.

### Roadmap v10

- [Roadmap v10: Closure And Product Integrity](roadmap-v10-closure-and-product-integrity.md) is the current improvement contract for coverage, bounded source sessions, stable identity, contracts, extensions, Markdown, the product surface, release evidence, and calibration.
- [R10-SIP-v1](r10-sip-v1-protocol.md) is the current execution contract that decomposes C0-C8 into internal slices and proceeds to the final report without pausing interactively for ordinary failures.
- [R10-SIP-v1 Template](r10-sip-v1-template.json) is the machine-readable authority for unit dependencies, bounded repairs, blocked evidence, authorities, and completion predicates.
- Roadmap v9 and R9-SIP-v1 remain the implementation record for semantic identity and completion v3. Roadmap v10 and R10-SIP-v1 override overlapping current improvement decisions.

### Roadmap v9

- [Roadmap v9: Semantic Identity And Verification Closure](roadmap-v9-semantic-identity-verification-closure.md) is the implementation record for semantic Markdown, path classification, private calibration, stable delta, and completion v3.
- [R9-SIP-v1](r9-sip-v1-protocol.md) is the historical execution contract that decomposed Roadmap v9 into SI0-SI12.
- [R9-SIP-v1 Template](r9-sip-v1-template.json) is the machine-readable unit dependency and authority-default contract.
- Roadmap v8 and RTIP-v1 remain the preceding trust contract and implementation record. Roadmap v9 and R9-SIP-v1 override current decisions in the same responsibility area.

### Reading Order

| Step | Document | Owns |
| --- | --- | --- |
| 1 | [Repository Trust Graph](repository-trust-graph.md) | Long-term premises for product boundaries, repository routes, and claim boundaries |
| 2 | [Baseline And Profiles](baseline-and-profiles.md) | Common patterns, purpose-specific profiles, and calibration-input handling |
| 3 | [Roadmap v5: Legacy Removal](roadmap-v5-legacy-removal.md) | Implementation record for the canonical 0.2.0 Rust architecture and legacy removal |
| 4 | [Roadmap v6: Completion](roadmap-v6-completion.md) | 1.0.0 CF0-CF7 implementation record, completion conditions, and stop conditions |
| 5 | [Roadmap v7: Calibrated Assertion](roadmap-v7-calibrated-assertion.md) | Current implementation contract for evidence-backed claims, boundary relevance, and underclaim loss |
| 6 | [Roadmap v8: Trust And Contract Integrity](roadmap-v8-trust-contract-integrity.md) | Current implementation contract for paths, Markdown, claims, provenance, resources, and completion |
| 7 | [RTIP-v1](rtip-v1-protocol.md) | Execution contract that decomposes Roadmap v8 into TI0-TI11 |
| 8 | [RTIP-v1 Template](rtip-v1-template.json) | Machine-readable TI dependencies and authority defaults |
| 9 | [RCBP-v1](completion-batch-protocol.md) | Historical execution contract for the Roadmap v6 completion batch |
| 10 | [RCBP-v1 Template](rcbp-v1-template.json) | Machine-readable RCBP-v1 history |
| 11 | [Roadmap v10: Closure And Product Integrity](roadmap-v10-closure-and-product-integrity.md) | Current improvement contract for coverage, bounded sources, stable identity, extensions, the product, and calibration |
| 12 | [R10-SIP-v1](r10-sip-v1-protocol.md) | Current noninteractive sequential implementation contract for C0-C8 |
| 13 | [R10-SIP-v1 Template](r10-sip-v1-template.json) | Machine-readable unit, repair-budget, authority, and completion authority |

### Authority

- Roadmap v5 is the implementation record for the 0.2.0 architecture and legacy removal.
- Roadmap v6 is the implementation record for the 1.0.0 completion baseline.
- Use Roadmap v7 for evidence-backed claim strength, claim-local boundaries, and underclaim loss.
- Roadmap v8, RTIP-v1, Roadmap v9, and R9-SIP-v1 are the preceding implementation contracts and records.
- Use Roadmap v10 and R10-SIP-v1 for current improvement responsibilities, completion conditions, and batch execution.
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
