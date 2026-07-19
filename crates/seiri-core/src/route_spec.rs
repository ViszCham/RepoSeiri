use crate::{DocumentLanguage, RouteKind, RoutePolicyBoundary};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteSpec {
    pub kind: RouteKind,
    pub slug: &'static str,
    pub label_ja: &'static str,
    pub label_en: &'static str,
    pub target_candidates: &'static [&'static str],
    pub policy: RoutePolicyBoundary,
}

impl RouteSpec {
    #[must_use]
    pub const fn label(self, language: DocumentLanguage) -> &'static str {
        match language {
            DocumentLanguage::Japanese => self.label_ja,
            DocumentLanguage::English => self.label_en,
        }
    }
}

const SUGGESTIBLE: RoutePolicyBoundary = RoutePolicyBoundary::Suggestible;
const MAINTAINER: RoutePolicyBoundary = RoutePolicyBoundary::MaintainerDecisionRequired;

pub const ROUTE_SPECS: [RouteSpec; 14] = [
    RouteSpec {
        kind: RouteKind::Identity,
        slug: "identity",
        label_ja: "リポジトリ情報",
        label_en: "Repository information",
        target_candidates: &[],
        policy: SUGGESTIBLE,
    },
    RouteSpec {
        kind: RouteKind::Docs,
        slug: "docs",
        label_ja: "ドキュメント",
        label_en: "Documentation",
        target_candidates: &["docs/", "docs/README.md"],
        policy: SUGGESTIBLE,
    },
    RouteSpec {
        kind: RouteKind::Quickstart,
        slug: "quickstart",
        label_ja: "クイックスタート",
        label_en: "Quickstart",
        target_candidates: &["docs/getting-started.md", "docs/quickstart.md"],
        policy: SUGGESTIBLE,
    },
    RouteSpec {
        kind: RouteKind::Support,
        slug: "support",
        label_ja: "サポート",
        label_en: "Support",
        target_candidates: &["SUPPORT.md"],
        policy: SUGGESTIBLE,
    },
    RouteSpec {
        kind: RouteKind::Intake,
        slug: "intake",
        label_ja: "Issue受付",
        label_en: "Issue intake",
        target_candidates: &[".github/ISSUE_TEMPLATE/", "SUPPORT.md"],
        policy: SUGGESTIBLE,
    },
    RouteSpec {
        kind: RouteKind::Contributing,
        slug: "contributing",
        label_ja: "コントリビューション",
        label_en: "Contributing",
        target_candidates: &["CONTRIBUTING.md"],
        policy: SUGGESTIBLE,
    },
    RouteSpec {
        kind: RouteKind::Security,
        slug: "security",
        label_ja: "セキュリティ方針",
        label_en: "Security policy",
        target_candidates: &["SECURITY.md"],
        policy: MAINTAINER,
    },
    RouteSpec {
        kind: RouteKind::Release,
        slug: "release",
        label_ja: "変更履歴とリリース",
        label_en: "Changes and releases",
        target_candidates: &["CHANGELOG.md", "docs/releases.md"],
        policy: SUGGESTIBLE,
    },
    RouteSpec {
        kind: RouteKind::Lifecycle,
        slug: "lifecycle",
        label_ja: "ライフサイクル",
        label_en: "Lifecycle",
        target_candidates: &["docs/releases.md", "CHANGELOG.md"],
        policy: MAINTAINER,
    },
    RouteSpec {
        kind: RouteKind::Governance,
        slug: "governance",
        label_ja: "ガバナンス",
        label_en: "Governance",
        target_candidates: &["GOVERNANCE.md"],
        policy: MAINTAINER,
    },
    RouteSpec {
        kind: RouteKind::License,
        slug: "license",
        label_ja: "ライセンス",
        label_en: "License",
        target_candidates: &["LICENSE", "LICENSE.md"],
        policy: MAINTAINER,
    },
    RouteSpec {
        kind: RouteKind::Automation,
        slug: "automation",
        label_ja: "自動化",
        label_en: "Automation",
        target_candidates: &[".github/workflows/"],
        policy: SUGGESTIBLE,
    },
    RouteSpec {
        kind: RouteKind::Ownership,
        slug: "ownership",
        label_ja: "所有と責任",
        label_en: "Ownership",
        target_candidates: &[".github/CODEOWNERS", "CODEOWNERS", "docs/CODEOWNERS"],
        policy: MAINTAINER,
    },
    RouteSpec {
        kind: RouteKind::Hygiene,
        slug: "hygiene",
        label_ja: "リポジトリ衛生",
        label_en: "Repository hygiene",
        target_candidates: &[".gitignore", ".gitattributes", ".editorconfig"],
        policy: SUGGESTIBLE,
    },
];

const UNKNOWN_ROUTE_SPEC: RouteSpec = RouteSpec {
    kind: RouteKind::Unknown,
    slug: "unknown",
    label_ja: "不明",
    label_en: "Unknown",
    target_candidates: &[],
    policy: SUGGESTIBLE,
};

impl RouteKind {
    pub const ALL: [Self; 15] = [
        Self::Identity,
        Self::Docs,
        Self::Quickstart,
        Self::Support,
        Self::Intake,
        Self::Contributing,
        Self::Security,
        Self::Release,
        Self::Lifecycle,
        Self::Governance,
        Self::License,
        Self::Automation,
        Self::Ownership,
        Self::Hygiene,
        Self::Unknown,
    ];

    #[must_use]
    pub const fn spec(self) -> &'static RouteSpec {
        match self {
            Self::Identity => &ROUTE_SPECS[0],
            Self::Docs => &ROUTE_SPECS[1],
            Self::Quickstart => &ROUTE_SPECS[2],
            Self::Support => &ROUTE_SPECS[3],
            Self::Intake => &ROUTE_SPECS[4],
            Self::Contributing => &ROUTE_SPECS[5],
            Self::Security => &ROUTE_SPECS[6],
            Self::Release => &ROUTE_SPECS[7],
            Self::Lifecycle => &ROUTE_SPECS[8],
            Self::Governance => &ROUTE_SPECS[9],
            Self::License => &ROUTE_SPECS[10],
            Self::Automation => &ROUTE_SPECS[11],
            Self::Ownership => &ROUTE_SPECS[12],
            Self::Hygiene => &ROUTE_SPECS[13],
            Self::Unknown => &UNKNOWN_ROUTE_SPEC,
        }
    }

    #[must_use]
    pub const fn slug(self) -> &'static str {
        self.spec().slug
    }

    #[must_use]
    pub const fn label(self, language: DocumentLanguage) -> &'static str {
        self.spec().label(language)
    }

    #[must_use]
    pub const fn target_candidates(self) -> &'static [&'static str] {
        self.spec().target_candidates
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn route_registry_is_unique_and_covers_every_non_unknown_route() {
        let kinds = ROUTE_SPECS
            .iter()
            .map(|spec| spec.kind)
            .collect::<BTreeSet<_>>();
        let slugs = ROUTE_SPECS
            .iter()
            .map(|spec| spec.slug)
            .collect::<BTreeSet<_>>();

        assert_eq!(kinds.len(), ROUTE_SPECS.len());
        assert_eq!(slugs.len(), ROUTE_SPECS.len());
        assert_eq!(RouteKind::ALL.len(), ROUTE_SPECS.len() + 1);
        assert!(!kinds.contains(&RouteKind::Unknown));
        for route in RouteKind::ALL {
            assert_eq!(route.spec().kind, route);
            assert!(!route.slug().is_empty());
        }
    }
}
