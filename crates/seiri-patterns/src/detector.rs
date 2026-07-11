use seiri_core::{
    EvidenceAtom, EvidenceFact, EvidenceId, ImportantFileKind, MarkdownEvidenceKind,
    RepositoryAnalysis, RouteKind, SourceDomain,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternDetector {
    Evidence(EvidenceAtom),
    Route(RouteKind),
    ReadmeRoute(RouteKind),
    ImportantFile(ImportantFileKind),
}

impl PatternDetector {
    #[must_use]
    pub fn evidence_ids(self, analysis: &RepositoryAnalysis) -> Vec<EvidenceId> {
        analysis
            .evidence_kernel
            .facts()
            .iter()
            .filter(|fact| self.matches(analysis, fact))
            .map(|fact| fact.id)
            .collect()
    }

    #[must_use]
    pub fn basis(self) -> &'static str {
        match self {
            Self::Evidence(_) => "typed evidence atom",
            Self::Route(_) => "repository route",
            Self::ReadmeRoute(_) => "README route",
            Self::ImportantFile(_) => "important file",
        }
    }

    #[must_use]
    pub fn label(self) -> String {
        match self {
            Self::Evidence(atom) => format!("evidence atom:{atom:?}"),
            Self::Route(route) => format!("repository route:{route:?}"),
            Self::ReadmeRoute(route) => format!("README route:{route:?}"),
            Self::ImportantFile(kind) => format!("important file:{kind:?}"),
        }
    }

    fn matches(self, analysis: &RepositoryAnalysis, fact: &EvidenceFact) -> bool {
        if fact.provenance.domain != SourceDomain::RepositoryLocal {
            return false;
        }
        match self {
            Self::Evidence(expected) => fact.atom == expected,
            Self::Route(expected) => fact.atom.route() == Some(expected),
            Self::ReadmeRoute(expected) => {
                analysis
                    .evidence_kernel
                    .path_for_fact(fact)
                    .is_some_and(is_root_readme_path)
                    && fact.atom.route() == Some(expected)
                    && matches!(
                        fact.atom,
                        EvidenceAtom::Markdown {
                            event: MarkdownEvidenceKind::Heading
                                | MarkdownEvidenceKind::Link
                                | MarkdownEvidenceKind::RouteCandidate,
                            ..
                        }
                    )
            }
            Self::ImportantFile(expected) => {
                fact.atom == EvidenceAtom::ImportantFile(expected)
                    && analysis
                        .evidence_kernel
                        .path_for_fact(fact)
                        .is_some_and(|path| important_file_matches(expected, path))
            }
        }
    }
}

fn is_root_readme_path(path: &str) -> bool {
    matches!(path, "README.md" | "Readme.md" | "readme.md" | "README")
}

fn important_file_matches(kind: ImportantFileKind, path: &str) -> bool {
    match kind {
        ImportantFileKind::License => {
            !path.contains('/')
                && matches!(
                    path.to_ascii_lowercase().as_str(),
                    "license" | "license.md" | "copying"
                )
        }
        _ => true,
    }
}
