use seiri_core::{
    stable_evidence_id, EvidenceId, EvidenceKind, EvidenceScope, ImportantFileKind, RepoSnapshot,
    RouteKind,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternDetector {
    EvidenceKind(EvidenceKind),
    Route(RouteKind),
    ReadmeRoute(RouteKind),
    ImportantFile(ImportantFileKind),
}

impl PatternDetector {
    #[must_use]
    pub fn evidence_ids(self, snapshot: &RepoSnapshot) -> Vec<EvidenceId> {
        if !snapshot.evidence_kernel.is_empty() {
            return snapshot
                .evidence_kernel
                .facts()
                .iter()
                .filter(|fact| self.matches(fact.scope, fact.kind, fact.route, &fact.value))
                .map(|fact| fact.id)
                .collect();
        }

        if !snapshot.evidence_ledger.is_empty() {
            return snapshot
                .evidence_ledger
                .iter()
                .filter(|record| {
                    self.matches(record.scope, record.kind, record.route, &record.value)
                })
                .map(|record| record.id)
                .collect();
        }

        snapshot
            .evidence
            .iter()
            .enumerate()
            .filter(|(_, evidence)| {
                self.matches(
                    EvidenceScope::Root,
                    evidence.kind,
                    evidence.route,
                    &evidence.value,
                )
            })
            .map(|(index, _)| stable_evidence_id(index + 1))
            .collect()
    }

    #[must_use]
    pub fn basis(self) -> &'static str {
        match self {
            Self::EvidenceKind(_) => "evidence kind",
            Self::Route(_) => "trust route",
            Self::ReadmeRoute(_) => "README trust route",
            Self::ImportantFile(_) => "important file",
        }
    }

    #[must_use]
    pub fn label(self) -> String {
        match self {
            Self::EvidenceKind(kind) => format!("evidence kind:{kind:?}"),
            Self::Route(route) => format!("trust route:{route:?}"),
            Self::ReadmeRoute(route) => format!("README route:{route:?}"),
            Self::ImportantFile(kind) => format!("important file:{kind:?}"),
        }
    }

    fn matches(
        self,
        scope: EvidenceScope,
        kind: EvidenceKind,
        route: Option<RouteKind>,
        value: &str,
    ) -> bool {
        if scope != EvidenceScope::Root {
            return false;
        }
        match self {
            Self::EvidenceKind(expected) => kind == expected,
            Self::Route(expected) => route == Some(expected),
            Self::ReadmeRoute(expected) => {
                route == Some(expected)
                    && matches!(
                        kind,
                        EvidenceKind::MarkdownHeading
                            | EvidenceKind::MarkdownLink
                            | EvidenceKind::RouteCandidate
                    )
            }
            Self::ImportantFile(expected) => {
                kind == EvidenceKind::ImportantFile && value == format!("{expected:?}")
            }
        }
    }
}
