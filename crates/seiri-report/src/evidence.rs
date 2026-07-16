use seiri_core::{
    DocumentEvent, DocumentIndex, DocumentRole, EvidenceAtom, EvidenceConfidence, EvidenceDraft,
    EvidenceFact, EvidenceId, EvidenceKernel, EvidenceKernelError, EvidenceProducer,
    ImportantFileKind, MarkdownEvidenceKind, PathClassification, PatternOutcome, ReadmePresence,
    ReadmeRouteAssessment, RepositoryAnalysis, RepositoryScopeGraph, RouteAssessment,
    RouteAssessmentError, RouteKind, SourceDomain,
};
use seiri_fs::RepoFsScan;
use std::collections::BTreeSet;

pub(crate) fn build_evidence_kernel(
    fs_scan: &RepoFsScan,
    document_index: &DocumentIndex,
    scope: &RepositoryScopeGraph,
) -> Result<EvidenceKernel, EvidenceKernelError> {
    let mut drafts = Vec::new();

    for important in &fs_scan.important_files {
        drafts.push(file_draft(
            EvidenceAtom::ImportantFile(important.kind),
            important.path.clone(),
            confidence_for_path(&important.path, EvidenceConfidence::High, scope),
            scope,
        ));
    }

    for entry in document_index.entries() {
        drafts.push(file_draft(
            EvidenceAtom::FilePresent,
            entry.path.clone(),
            confidence_for_path(&entry.path, EvidenceConfidence::High, scope),
            scope,
        ));
    }

    for entry in document_index.scanned_documents() {
        let document = entry
            .scan
            .as_ref()
            .expect("scanned document index entries carry a scan payload");
        if entry.role == DocumentRole::RootReadme {
            drafts.push(markdown_draft(
                EvidenceAtom::Readme(ReadmePresence::Present),
                Some(document.path().to_string()),
                None,
                EvidenceConfidence::High,
                scope,
            ));
        }

        for event in document.events() {
            match event {
                DocumentEvent::Heading(heading) => {
                    let route = seiri_markdown::classify_route(&heading.text, None);
                    drafts.push(markdown_draft(
                        EvidenceAtom::Markdown {
                            event: MarkdownEvidenceKind::Heading,
                            route: (route != RouteKind::Unknown).then_some(route),
                        },
                        Some(document.path().to_string()),
                        heading.span,
                        EvidenceConfidence::Medium,
                        scope,
                    ));
                }
                DocumentEvent::Link(link) => drafts.push(markdown_draft(
                    EvidenceAtom::Markdown {
                        event: MarkdownEvidenceKind::Link,
                        route: link.route,
                    },
                    Some(document.path().to_string()),
                    link.span,
                    EvidenceConfidence::Medium,
                    scope,
                )),
                DocumentEvent::Badge(badge) => drafts.push(markdown_draft(
                    EvidenceAtom::Markdown {
                        event: MarkdownEvidenceKind::Badge,
                        route: Some(RouteKind::Automation),
                    },
                    Some(document.path().to_string()),
                    badge.span,
                    EvidenceConfidence::Medium,
                    scope,
                )),
                DocumentEvent::RouteCandidate(route) => drafts.push(markdown_draft(
                    EvidenceAtom::Markdown {
                        event: MarkdownEvidenceKind::RouteCandidate,
                        route: Some(route.route),
                    },
                    Some(document.path().to_string()),
                    route.span,
                    EvidenceConfidence::Medium,
                    scope,
                )),
            }
        }
    }

    if !document_index.has_root_readme_candidate()
        && document_index.coverage_for_role(DocumentRole::RootReadme)
            == Some(seiri_core::CoverageStatus::Complete)
    {
        drafts.push(markdown_draft(
            EvidenceAtom::Readme(ReadmePresence::Absent),
            None,
            None,
            EvidenceConfidence::High,
            scope,
        ));
    }

    let indexed_paths = document_index
        .entries()
        .iter()
        .map(|entry| entry.path.as_str())
        .collect::<BTreeSet<_>>();
    let mut facet_signal_paths = fs_scan
        .files
        .iter()
        .map(|file| file.path.replace('\\', "/"))
        .filter(|path| {
            seiri_core::is_facet_signal_path(path)
                && PathClassification::classify(path, Some(scope)).is_primary_repository_content()
                && !indexed_paths.contains(path.as_str())
        })
        .collect::<Vec<_>>();
    facet_signal_paths.sort();
    facet_signal_paths.dedup();
    for path in facet_signal_paths {
        let confidence = confidence_for_path(&path, EvidenceConfidence::High, scope);
        drafts.push(file_draft(
            EvidenceAtom::FilePresent,
            path,
            confidence,
            scope,
        ));
    }

    EvidenceKernel::from_drafts(drafts)
}

pub(crate) fn build_route_assessments(
    analysis: &RepositoryAnalysis,
) -> Result<Vec<RouteAssessment>, RouteAssessmentError> {
    route_routes()
        .iter()
        .map(|route| build_route_assessment(analysis, *route))
        .collect()
}

fn build_route_assessment(
    analysis: &RepositoryAnalysis,
    route: RouteKind,
) -> Result<RouteAssessment, RouteAssessmentError> {
    let root_structural = route_evidence_ids(analysis, route, |fact| {
        is_repository_root_fact(analysis, fact) && fact.atom.is_structural()
    });
    let readme_route = route_evidence_ids(analysis, route, |fact| {
        fact.provenance.domain == SourceDomain::RepositoryLocal
            && analysis
                .evidence_kernel
                .path_for_fact(fact)
                .is_some_and(is_root_readme_path)
            && fact.atom.is_markdown_route()
    });
    let inherited = analysis
        .evidence_kernel
        .facts()
        .iter()
        .filter(|fact| {
            fact.provenance.domain == SourceDomain::OrganizationInherited
                && fact.atom.route() == Some(route)
                && route != RouteKind::License
        })
        .map(|fact| fact.id)
        .collect();
    let missing_pattern = analysis.pattern_matches.iter().any(|pattern_match| {
        pattern_match.route == Some(route) && pattern_match.outcome == PatternOutcome::Missing
    });
    let readme_assessment = analysis
        .readme_summary
        .as_ref()
        .and_then(|readme| {
            readme
                .route_map
                .entries
                .iter()
                .find(|entry| entry.route == route)
        })
        .map_or_else(ReadmeRouteAssessment::default, |entry| entry.assessment);

    RouteAssessment::new(
        route,
        readme_assessment,
        missing_pattern,
        root_structural,
        readme_route,
        inherited,
    )
}

fn route_evidence_ids(
    analysis: &RepositoryAnalysis,
    route: RouteKind,
    predicate: impl Fn(&EvidenceFact) -> bool,
) -> Vec<EvidenceId> {
    analysis
        .evidence_kernel
        .facts()
        .iter()
        .filter(|fact| fact.atom.route() == Some(route) && predicate(fact))
        .map(|fact| fact.id)
        .collect()
}

fn is_repository_root_fact(analysis: &RepositoryAnalysis, fact: &EvidenceFact) -> bool {
    if fact.provenance.domain != SourceDomain::RepositoryLocal {
        return false;
    }
    let Some(path) = analysis.evidence_kernel.path_for_fact(fact) else {
        return matches!(fact.atom, EvidenceAtom::Readme(ReadmePresence::Absent));
    };
    is_root_path(path, fact.atom)
}

fn file_draft(
    atom: EvidenceAtom,
    path: String,
    confidence: EvidenceConfidence,
    scope: &RepositoryScopeGraph,
) -> EvidenceDraft {
    EvidenceDraft {
        atom,
        domain: PathClassification::classify(&path, Some(scope)).source_domain(),
        producer: EvidenceProducer::FileWalker,
        path: Some(path),
        span: None,
        confidence,
    }
}

fn markdown_draft(
    atom: EvidenceAtom,
    path: Option<String>,
    span: Option<seiri_core::SourceSpan>,
    confidence: EvidenceConfidence,
    scope: &RepositoryScopeGraph,
) -> EvidenceDraft {
    let domain = path
        .as_deref()
        .map_or(SourceDomain::RepositoryLocal, |path| {
            PathClassification::classify(path, Some(scope)).source_domain()
        });
    EvidenceDraft {
        atom,
        domain,
        producer: EvidenceProducer::Markdown,
        path,
        span,
        confidence,
    }
}

fn confidence_for_path(
    path: &str,
    root: EvidenceConfidence,
    scope: &RepositoryScopeGraph,
) -> EvidenceConfidence {
    let classification = PathClassification::classify(path, Some(scope));
    if !classification.is_primary_repository_content() {
        EvidenceConfidence::Low
    } else {
        root
    }
}

fn is_root_path(path: &str, atom: EvidenceAtom) -> bool {
    let lower = path.replace('\\', "/").to_ascii_lowercase();
    if !lower.contains('/') || lower == "docs" {
        return true;
    }
    matches!(atom, EvidenceAtom::ImportantFile(kind) if is_root_github_operational_file(&lower, kind))
}

fn is_root_github_operational_file(path: &str, kind: ImportantFileKind) -> bool {
    matches!(
        kind,
        ImportantFileKind::IssueTemplate
            | ImportantFileKind::IssueForm
            | ImportantFileKind::PullRequestTemplate
            | ImportantFileKind::DependencyBot
            | ImportantFileKind::SecurityAutomation
            | ImportantFileKind::Workflow
            | ImportantFileKind::Codeowners
    ) && path.starts_with(".github/")
}

fn route_routes() -> &'static [RouteKind] {
    &[
        RouteKind::Identity,
        RouteKind::Docs,
        RouteKind::Quickstart,
        RouteKind::Support,
        RouteKind::Intake,
        RouteKind::Contributing,
        RouteKind::Security,
        RouteKind::Release,
        RouteKind::Lifecycle,
        RouteKind::Governance,
        RouteKind::License,
        RouteKind::Automation,
        RouteKind::Ownership,
        RouteKind::Hygiene,
    ]
}

fn is_root_readme_path(path: &str) -> bool {
    matches!(path, "README.md" | "Readme.md" | "readme.md" | "README")
}
