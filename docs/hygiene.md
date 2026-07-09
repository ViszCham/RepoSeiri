# Repository Hygiene

## 日本語

RepoSeiri の repository hygiene route は、source tree を読みやすく保つための保守境界です。`.gitignore`、`.gitattributes`、docs の hygiene 方針、self-audit loop を証拠として扱います。

この route は、リポジトリが高品質であること、不要 file がないこと、secret が混入していないこと、または large file policy が正しいことを保証しません。RepoSeiri は観測できる file と README route から review aid を出します。

### Root hygiene files

| File | Role | Boundary |
| --- | --- | --- |
| `.gitignore` | build output、一時 file、local report を追跡対象から外します。 | source、docs、fixture、lockfile を隠すために使いません。 |
| `.gitattributes` | text / binary と line ending の既定を固定します。 | binary policy、LFS 採用、release artifact 配布を自動決定しません。 |
| `docs/hygiene.md` | hygiene 方針の正です。 | security policy や ownership policy を再定義しません。 |
| `docs/self-audit.md` | RepoSeiri が RepoSeiri 自身を確認する loop の正です。 | CI が通ることを release 可否や品質保証に変換しません。 |

### Tracked / ignored boundary

| Keep tracked | Keep ignored |
| --- | --- |
| Rust source、Cargo manifests、Cargo.lock、docs、fixtures、plugin files | `target/`、coverage output、一時 file、local generated report |
| Policy files and GitHub templates | OS/editor artifacts and local cache |
| Small deterministic fixtures | large generated artifacts unless a maintainer explicitly decides otherwise |

### Review rules

- `.gitignore` の追加は、必要な source evidence を消していないか確認します。
- `.gitattributes` の変更は、text/binary 判定と line ending の影響を確認します。
- large file、generated artifact、dataset、model artifact、binary release artifact は自動採用せず、maintainer review に送ります。
- secret や credential は hygiene ではなく security incident として扱い、`SECURITY.md` の route を使います。
- self-audit output は review aid です。score や route state は人気、信頼、安全性、品質の保証ではありません。

---

## English

The RepoSeiri repository hygiene route is a maintenance boundary for keeping the source tree reviewable. It treats `.gitignore`, `.gitattributes`, the hygiene docs, and the self-audit loop as observable evidence.

This route does not guarantee that the repository is high quality, free of unnecessary files, free of secrets, or governed by the right large-file policy. RepoSeiri emits review aids from observable files and README routes.

### Root hygiene files

| File | Role | Boundary |
| --- | --- | --- |
| `.gitignore` | Keeps build output, temporary files, and local reports out of tracking. | Do not use it to hide source, docs, fixtures, or lockfiles. |
| `.gitattributes` | Fixes default text / binary handling and line endings. | Does not automatically decide binary policy, LFS adoption, or release artifact distribution. |
| `docs/hygiene.md` | Authoritative hygiene policy. | Does not redefine security policy or ownership policy. |
| `docs/self-audit.md` | Authoritative loop for RepoSeiri checking RepoSeiri itself. | Does not turn passing CI into release approval or a quality guarantee. |

### Tracked / ignored boundary

| Keep tracked | Keep ignored |
| --- | --- |
| Rust source, Cargo manifests, Cargo.lock, docs, fixtures, plugin files | `target/`, coverage output, temporary files, local generated reports |
| Policy files and GitHub templates | OS/editor artifacts and local cache |
| Small deterministic fixtures | Large generated artifacts unless a maintainer explicitly decides otherwise |

### Review rules

- When changing `.gitignore`, check that required source evidence is not hidden.
- When changing `.gitattributes`, check text/binary classification and line-ending impact.
- Large files, generated artifacts, datasets, model artifacts, and binary release artifacts are not adopted automatically; route them to maintainer review.
- Secrets and credentials are security incidents, not hygiene work; use the `SECURITY.md` route.
- Self-audit output is a review aid. Scores and route states do not guarantee popularity, trust, safety, or quality.
