# Design Documentation

## 日本語

この subindex は、RepoSeiri の設計文書を読む順序と責務境界を示します。設計判断はここから各文書へ分岐し、README には戻しません。

### Reading order

| Step | Document | Use when |
| --- | --- | --- |
| 1 | [Repository Trust Graph](repository-trust-graph.md) | RepoSeiri の固定設計、trust route、graph 全体を読むとき。 |
| 2 | [Baseline And Profiles](baseline-and-profiles.md) | 分析データ、common baseline、目的別 profile の扱いを読むとき。 |
| 3 | [Repair, Implementation, And Verification](repair-implementation-and-verification.md) | Rust module 境界、safe/guarded/manual gate、verification を読むとき。 |
| 4 | [Roadmap And Implementation Blocks](roadmap-and-implementation-blocks.md) | 実装block、将来拡張、calibration ingest 準備を読むとき。 |

### Boundaries

| Topic | Owned by | Boundary |
| --- | --- | --- |
| Product design | `repository-trust-graph.md` | 人気、信頼、安全性、品質の保証として書かない。 |
| Analysis data and profiles | `baseline-and-profiles.md` | 分析データは calibration input であり、統計的証明ではない。 |
| Repair and gates | `repair-implementation-and-verification.md` | policy、license、security SLA、ownership は自動決定しない。 |
| Roadmap | `roadmap-and-implementation-blocks.md` | 実装順序を示すが、完成や性能を保証しない。 |

### Update rules

- 新しい設計文書を追加する場合は、この subindex に入口を追加します。
- 既存文書と責務が重なる場合は、新規文書ではなく既存文書を更新します。
- README へ戻すのは、first-read route として必要な短い入口だけにします。
- 実装済み事実、計画、仮説、manual decision を混ぜません。

---

## English

This subindex defines the reading order and responsibility boundaries for RepoSeiri design documents. Design decisions branch from here into individual documents and do not move back into the README.

### Reading order

| Step | Document | Use when |
| --- | --- | --- |
| 1 | [Repository Trust Graph](repository-trust-graph.md) | Reading the fixed RepoSeiri design, trust routes, and whole graph. |
| 2 | [Baseline And Profiles](baseline-and-profiles.md) | Reading analysis data handling, common baseline, and purpose-specific profiles. |
| 3 | [Repair, Implementation, And Verification](repair-implementation-and-verification.md) | Reading Rust module boundaries, safe/guarded/manual gates, and verification. |
| 4 | [Roadmap And Implementation Blocks](roadmap-and-implementation-blocks.md) | Reading implementation blocks, future extension, and calibration ingest preparation. |

### Boundaries

| Topic | Owned by | Boundary |
| --- | --- | --- |
| Product design | `repository-trust-graph.md` | Do not describe it as a guarantee of popularity, trust, safety, or quality. |
| Analysis data and profiles | `baseline-and-profiles.md` | Analysis data is calibration input, not statistical proof. |
| Repair and gates | `repair-implementation-and-verification.md` | Policy, license, security SLA, and ownership are not decided automatically. |
| Roadmap | `roadmap-and-implementation-blocks.md` | Shows implementation order but does not guarantee completion or performance. |

### Update rules

- Add an entry to this subindex when adding a new design document.
- If the responsibility overlaps an existing document, update the existing document instead of creating a new one.
- Move content back to the README only when a short first-read entry is necessary.
- Keep implemented facts, plans, hypotheses, and manual decisions separate.
