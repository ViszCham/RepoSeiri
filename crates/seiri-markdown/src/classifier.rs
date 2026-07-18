use seiri_core::RouteKind;

#[must_use]
pub fn classify_route(text: &str, target: Option<&str>) -> RouteKind {
    classify_routes(text, target)
        .into_iter()
        .next()
        .unwrap_or(RouteKind::Unknown)
}

#[must_use]
pub fn classify_routes(text: &str, target: Option<&str>) -> Vec<RouteKind> {
    let routes = route_aliases(&text.to_lowercase());
    if !routes.is_empty() {
        return routes;
    }
    target
        .map(|target| route_aliases(&target.to_lowercase()))
        .unwrap_or_default()
}

fn route_aliases(value: &str) -> Vec<RouteKind> {
    const ROUTE_ALIASES: &[(RouteKind, &[&str])] = &[
        (
            RouteKind::Hygiene,
            &[
                "hygiene",
                "repository hygiene",
                "cleanup",
                "clean-up",
                "self-audit",
                "self audit",
                "リポジトリ整理",
                "衛生",
            ],
        ),
        (
            RouteKind::Quickstart,
            &[
                "quickstart",
                "quick start",
                "getting started",
                "install",
                "installation",
                "usage",
                "example",
                "examples",
                "クイックスタート",
                "はじめに",
                "始め方",
                "インストール",
                "使い方",
                "使用例",
            ],
        ),
        (
            RouteKind::Docs,
            &[
                "docs",
                "documentation",
                "guide",
                "guides",
                "manual",
                "reference",
                "ドキュメント",
                "文書",
                "ガイド",
                "手引き",
                "リファレンス",
            ],
        ),
        (
            RouteKind::Intake,
            &[
                "issue template",
                "issue form",
                "bug report",
                "feature request",
                "pull request template",
                "pr template",
                "triage",
                "intake",
                "issue テンプレート",
                "バグ報告",
                "機能要望",
                "受付",
            ],
        ),
        (
            RouteKind::Lifecycle,
            &[
                "lifecycle",
                "life cycle",
                "maintenance",
                "maintained",
                "deprecation",
                "deprecated",
                "end of life",
                "end-of-life",
                "eol",
                "lts",
                "long term support",
                "supported versions",
                "version support",
                "support matrix",
                "compatibility policy",
                "archive policy",
                "archival",
                "sunset",
                "ライフサイクル",
                "保守",
                "メンテナンス",
                "非推奨",
                "サポート対象バージョン",
            ],
        ),
        (
            RouteKind::Support,
            &[
                "support",
                "discussions",
                "discussion",
                "help",
                "contact",
                "questions",
                "question",
                "issues",
                "issue",
                "サポート",
                "問い合わせ",
                "質問",
                "相談",
            ],
        ),
        (
            RouteKind::Contributing,
            &[
                "contributing",
                "contribute",
                "contribution",
                "development",
                "コントリビューション",
                "貢献",
                "開発参加",
            ],
        ),
        (
            RouteKind::Security,
            &[
                "security",
                "vulnerability",
                "vulnerabilities",
                "disclosure",
                "セキュリティ",
                "脆弱性",
                "開示",
            ],
        ),
        (
            RouteKind::Release,
            &[
                "release",
                "releases",
                "changelog",
                "changes",
                "version",
                "versions",
                "versioning",
                "compatibility",
                "リリース",
                "変更履歴",
                "バージョン",
                "互換性",
            ],
        ),
        (
            RouteKind::Governance,
            &[
                "governance",
                "roadmap",
                "rfc",
                "proposal",
                "ガバナンス",
                "ロードマップ",
                "提案",
                "意思決定",
            ],
        ),
        (
            RouteKind::License,
            &["license", "licence", "copying", "ライセンス", "許諾"],
        ),
        (
            RouteKind::Ownership,
            &[
                "codeowners",
                "maintainer",
                "maintainers",
                "ownership",
                "owner",
                "owners",
                "メンテナー",
                "管理者",
                "所有者",
            ],
        ),
        (
            RouteKind::Automation,
            &[
                "workflow",
                "workflows",
                "actions",
                "ci",
                "build",
                "badge",
                "automation",
                "自動化",
                "ワークフロー",
                "ビルド",
            ],
        ),
        (
            RouteKind::Identity,
            &[
                "readme",
                "overview",
                "about",
                "概要",
                "このプロジェクトについて",
            ],
        ),
    ];
    let mut routes = Vec::new();
    for (route, aliases) in ROUTE_ALIASES {
        if aliases
            .iter()
            .any(|alias| contains_route_alias(value, alias))
        {
            routes.push(*route);
        }
    }
    if routes.contains(&RouteKind::Intake)
        && !contains_route_alias(value, "support")
        && !value.contains("サポート")
    {
        routes.retain(|route| *route != RouteKind::Support);
    }
    if routes.contains(&RouteKind::Lifecycle)
        && routes.contains(&RouteKind::Release)
        && !has_explicit_release_marker(value)
    {
        routes.retain(|route| *route != RouteKind::Release);
    }
    routes
}

fn has_explicit_release_marker(value: &str) -> bool {
    [
        "release",
        "releases",
        "changelog",
        "changes",
        "versioning",
        "リリース",
        "変更履歴",
    ]
    .iter()
    .any(|alias| contains_route_alias(value, alias))
}

fn contains_route_alias(value: &str, alias: &str) -> bool {
    if !alias.is_ascii() {
        return value.contains(alias);
    }
    value.match_indices(alias).any(|(start, matched)| {
        let end = start + matched.len();
        let bytes = value.as_bytes();
        !start
            .checked_sub(1)
            .and_then(|index| bytes.get(index))
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
            && !bytes
                .get(end)
                .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_specific_version_terms_do_not_create_release_conflicts() {
        assert_eq!(
            classify_routes("Supported versions", None),
            vec![RouteKind::Lifecycle]
        );
        assert_eq!(
            classify_routes("Compatibility policy", None),
            vec![RouteKind::Lifecycle]
        );
        assert_eq!(
            classify_routes("Release supported versions", None),
            vec![RouteKind::Lifecycle, RouteKind::Release]
        );
    }
}
