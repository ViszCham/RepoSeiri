# RepoSeiri Agent Instructions

## 日本語

このリポジトリでは、RepoSeiri の固定前提を保って作業します。

### 固定前提

- 名前は `RepoSeiri` とします。
- 実装言語は Rust とします。
- 人間が読む主要ドキュメントは、前半を日本語、後半を英語にします。
- 日本語部分と英語部分は、同じ意味、同じ判断、同じ制約、同じ警告を持つように更新します。
- 片方の言語だけに新しい仕様や作業手順を追加しないでください。

### 作業方針

- 小さく検証できる変更を優先します。
- 実装が追加されたら、該当する Rust の検証コマンドを実行します。
- benchmark aggregate の数値は、初期設計の重み付けとして扱い、実測済みの完全証明として扱いません。
- ユーザーや別作業の未関係な変更を戻さないでください。

---

## English

Work in this repository while preserving the fixed premises for RepoSeiri.

### Fixed Premises

- The name is `RepoSeiri`.
- The implementation language is Rust.
- Major human-facing documents use Japanese in the first half and English in the second half.
- Update the Japanese half and the English half so they carry the same meaning, decisions, constraints, and warnings.
- Do not add new specifications or work steps to only one language.

### Work Policy

- Prefer small changes that can be verified.
- After implementation is added, run the relevant Rust verification commands.
- Treat benchmark aggregate numbers as initial design weights, not as measured complete proof.
- Do not revert unrelated changes made by the user or by other work.
