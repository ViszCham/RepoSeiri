use seiri_core::{
    stable_id, DocumentEvent, DocumentIndex, DocumentRole, Evidence, EvidenceConfidence,
    EvidenceDraft, EvidenceEvent, EvidenceFact, EvidenceId, EvidenceKernel, EvidenceKernelError,
    EvidenceKernelV2, EvidenceKind, EvidenceOrigin, EvidenceRecord, EvidenceScanner, EvidenceScope,
    EvidenceSource, EvidenceSpan, ImportantFileKind, PatternMatch, PatternOutcome,
    ReadmeRouteAssessment, ReadmeSummary, RouteAssessment, RouteAssessmentError, RouteKind,
    RouteStateReport, SourceDomain,
};
use seiri_fs::RepoFsScan;
use std::collections::BTreeSet;

pub(crate) fn build_evidence_kernel(
    fs_scan: &RepoFsScan,
    document_index: &DocumentIndex,
) -> Result<EvidenceKernel, EvidenceKernelError> {
    let mut drafts = Vec::new();

    for important in &fs_scan.important_files {
        drafts.push(evidence_draft(
            EvidenceKind::ImportantFile,
            Some(important.path.clone()),
            route_for_important_file(important.kind),
            format!("{:?}", important.kind),
            EvidenceOrigin {
                scanner: EvidenceScanner::FileSystem,
                event: EvidenceEvent::ImportantFileDetection,
            },
            None,
        ));
    }

    for entry in document_index.entries() {
        drafts.push(evidence_draft(
            EvidenceKind::FilePresent,
            Some(entry.path.clone()),
            None,
            "indexed document candidate".to_string(),
            EvidenceOrigin {
                scanner: EvidenceScanner::FileSystem,
                event: EvidenceEvent::ImportantFileDetection,
            },
            None,
        ));
    }

    for entry in document_index.scanned_documents() {
        let document = entry
            .scan
            .as_ref()
            .expect("scanned document index entries carry a scan payload");
        if entry.role == DocumentRole::RootReadme {
            drafts.push(evidence_draft(
                EvidenceKind::ReadmePresent,
                Some(document.path().to_string()),
                Some(RouteKind::Identity),
                "README detected".to_string(),
                EvidenceOrigin {
                    scanner: EvidenceScanner::Markdown,
                    event: EvidenceEvent::ReadmeDiscovery,
                },
                None,
            ));
        }

        for heading in document.events().iter().filter_map(|event| match event {
            DocumentEvent::Heading(heading) => Some(heading),
            _ => None,
        }) {
            let route = seiri_markdown::classify_route(&heading.text, None);
            drafts.push(evidence_draft(
                EvidenceKind::MarkdownHeading,
                Some(document.path().to_string()),
                (route != RouteKind::Unknown).then_some(route),
                heading.text.clone(),
                EvidenceOrigin {
                    scanner: EvidenceScanner::Markdown,
                    event: EvidenceEvent::MarkdownHeading,
                },
                heading.span,
            ));
        }

        for link in document.events().iter().filter_map(|event| match event {
            DocumentEvent::Link(link) => Some(link),
            _ => None,
        }) {
            drafts.push(evidence_draft(
                EvidenceKind::MarkdownLink,
                Some(document.path().to_string()),
                link.route,
                format!("{} -> {}", link.text, link.target),
                EvidenceOrigin {
                    scanner: EvidenceScanner::Markdown,
                    event: EvidenceEvent::MarkdownLink,
                },
                link.span,
            ));
        }

        for badge in document.events().iter().filter_map(|event| match event {
            DocumentEvent::Badge(badge) => Some(badge),
            _ => None,
        }) {
            drafts.push(evidence_draft(
                EvidenceKind::MarkdownBadge,
                Some(document.path().to_string()),
                Some(RouteKind::Automation),
                format!("{} -> {}", badge.alt, badge.target),
                EvidenceOrigin {
                    scanner: EvidenceScanner::Markdown,
                    event: EvidenceEvent::MarkdownBadge,
                },
                badge.span,
            ));
        }

        for route in document.events().iter().filter_map(|event| match event {
            DocumentEvent::RouteCandidate(route) => Some(route),
            _ => None,
        }) {
            drafts.push(evidence_draft(
                EvidenceKind::RouteCandidate,
                Some(document.path().to_string()),
                Some(route.route),
                route.target.as_ref().map_or_else(
                    || route.text.clone(),
                    |target| format!("{} -> {target}", route.text),
                ),
                EvidenceOrigin {
                    scanner: EvidenceScanner::Markdown,
                    event: EvidenceEvent::RouteCandidate {
                        source: route.source,
                    },
                },
                route.span,
            ));
        }
    }

    if !document_index.has_root_readme_candidate()
        && document_index.coverage_for_role(DocumentRole::RootReadme)
            == Some(seiri_core::CoverageStatus::Complete)
    {
        drafts.push(evidence_draft(
            EvidenceKind::ReadmeMissing,
            None,
            Some(RouteKind::Identity),
            "README not detected".to_string(),
            EvidenceOrigin {
                scanner: EvidenceScanner::Markdown,
                event: EvidenceEvent::ReadmeDiscovery,
            },
            None,
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
        .filter(|path| is_facet_signal_path(path) && !indexed_paths.contains(path.as_str()))
        .collect::<Vec<_>>();
    facet_signal_paths.sort();
    facet_signal_paths.dedup();
    for path in facet_signal_paths {
        drafts.push(evidence_draft(
            EvidenceKind::FilePresent,
            Some(path),
            None,
            "facet signal file candidate".to_string(),
            EvidenceOrigin {
                scanner: EvidenceScanner::FileSystem,
                event: EvidenceEvent::ImportantFileDetection,
            },
            None,
        ));
    }

    EvidenceKernel::from_drafts(drafts)
}

fn is_facet_signal_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "cargo.toml"
            | "package.json"
            | "pyproject.toml"
            | "go.mod"
            | "src/main.rs"
            | "main.go"
            | "main.py"
    ) || lower.starts_with("src/bin/")
        || lower.starts_with("cmd/")
        || lower.split('/').any(|segment| {
            matches!(
                segment,
                "infra"
                    | "infrastructure"
                    | "terraform"
                    | "k8s"
                    | "helm"
                    | "deploy"
                    | "deployments"
                    | "ops"
                    | "research"
                    | "paper"
                    | "papers"
                    | "dataset"
                    | "datasets"
                    | "notebook"
                    | "notebooks"
                    | "experiment"
                    | "experiments"
                    | "template"
                    | "templates"
                    | "cookiecutter"
                    | "scaffold"
                    | "app"
                    | "apps"
                    | "web"
                    | "frontend"
                    | "backend"
                    | "product"
            )
        })
}

pub(crate) fn legacy_evidence_view(kernel: &EvidenceKernel) -> Vec<Evidence> {
    kernel
        .facts()
        .iter()
        .map(|fact| Evidence {
            id: legacy_evidence_id(fact),
            kind: fact.kind,
            path: fact.path.clone(),
            route: fact.route,
            value: fact.value.clone(),
            source: compatibility_source(fact),
        })
        .collect()
}

pub(crate) fn legacy_evidence_ledger_view(kernel: &EvidenceKernel) -> Vec<EvidenceRecord> {
    kernel
        .facts()
        .iter()
        .map(|fact| EvidenceRecord {
            id: fact.id,
            legacy_evidence_id: Some(legacy_evidence_id(fact)),
            kind: fact.kind,
            path: fact.path.clone(),
            route: fact.route,
            value: fact.value.clone(),
            source: compatibility_source(fact),
            scope: fact.scope,
            confidence: fact.confidence,
            span: fact.span.map(EvidenceSpan::from),
        })
        .collect()
}

pub(crate) fn build_route_assessments(
    evidence_facts: &[EvidenceFact],
    evidence_v2: &EvidenceKernelV2,
    pattern_matches: &[PatternMatch],
    readme: Option<&ReadmeSummary>,
) -> Result<Vec<RouteAssessment>, RouteAssessmentError> {
    route_state_routes()
        .iter()
        .map(|route| {
            build_route_assessment(*route, evidence_facts, evidence_v2, pattern_matches, readme)
        })
        .collect()
}

pub(crate) fn legacy_route_state_views(assessments: &[RouteAssessment]) -> Vec<RouteStateReport> {
    assessments
        .iter()
        .map(|assessment| {
            let projection = assessment.legacy_projection();
            RouteStateReport {
                route: assessment.route(),
                state: projection.state,
                evidence_ids: assessment.legacy_evidence_ids(),
                confidence: projection.confidence,
                reason: projection.reason.to_string(),
            }
        })
        .collect()
}

fn evidence_draft(
    kind: EvidenceKind,
    path: Option<String>,
    route: Option<RouteKind>,
    value: String,
    origin: EvidenceOrigin,
    span: Option<seiri_core::SourceSpan>,
) -> EvidenceDraft {
    let scope = evidence_scope(kind, path.as_deref(), &value);
    EvidenceDraft {
        kind,
        path,
        route,
        value,
        origin,
        scope,
        confidence: evidence_confidence(kind, scope),
        span,
    }
}

fn evidence_scope(kind: EvidenceKind, path: Option<&str>, value: &str) -> EvidenceScope {
    let Some(path) = path else {
        return EvidenceScope::Root;
    };
    let normalized = path.replace('\\', "/");
    let lower = normalized.to_ascii_lowercase();
    let segments = lower.split('/').collect::<Vec<_>>();

    if segments
        .iter()
        .any(|segment| matches!(*segment, "fixtures" | "__fixtures__" | "fixture"))
    {
        return EvidenceScope::Fixture;
    }
    if segments
        .iter()
        .any(|segment| matches!(*segment, "target" | "dist" | "build" | "coverage"))
    {
        return EvidenceScope::Generated;
    }
    if !lower.contains('/')
        || lower == "docs"
        || (lower.starts_with(".github/workflows/")
            && kind == EvidenceKind::ImportantFile
            && value == "Workflow")
        || (lower == ".github/codeowners"
            && kind == EvidenceKind::ImportantFile
            && value == "Codeowners")
        || (kind == EvidenceKind::ImportantFile && root_github_operational_file(&lower, value))
    {
        return EvidenceScope::Root;
    }

    EvidenceScope::Nested
}

fn evidence_confidence(kind: EvidenceKind, scope: EvidenceScope) -> EvidenceConfidence {
    if !matches!(scope, EvidenceScope::Root) {
        return EvidenceConfidence::Low;
    }

    match kind {
        EvidenceKind::FilePresent
        | EvidenceKind::ImportantFile
        | EvidenceKind::ReadmePresent
        | EvidenceKind::ReadmeMissing => EvidenceConfidence::High,
        EvidenceKind::MarkdownHeading
        | EvidenceKind::MarkdownLink
        | EvidenceKind::MarkdownBadge
        | EvidenceKind::RouteCandidate => EvidenceConfidence::Medium,
    }
}

fn legacy_evidence_id(fact: &EvidenceFact) -> String {
    let prefix = match fact.kind {
        EvidenceKind::ImportantFile => "ev-important-file",
        EvidenceKind::ReadmePresent => "ev-readme-present",
        EvidenceKind::ReadmeMissing => "ev-readme-missing",
        EvidenceKind::MarkdownHeading => "ev-heading",
        EvidenceKind::MarkdownLink => "ev-link",
        EvidenceKind::MarkdownBadge => "ev-badge",
        EvidenceKind::RouteCandidate => "ev-route",
        EvidenceKind::FilePresent => "ev-file-present",
    };
    stable_id(prefix, fact.id.ordinal() as usize)
}

fn compatibility_source(fact: &EvidenceFact) -> EvidenceSource {
    let scanner = match fact.origin.scanner {
        EvidenceScanner::FileSystem => "seiri-fs",
        EvidenceScanner::Markdown => "seiri-markdown",
    };
    let line = fact.span.map(|span| span.line);
    let detail = match fact.origin.event {
        EvidenceEvent::ImportantFileDetection => "important file detection".to_string(),
        EvidenceEvent::ReadmeDiscovery => "readme discovery".to_string(),
        EvidenceEvent::MarkdownHeading => compatibility_line_detail("heading", line),
        EvidenceEvent::MarkdownLink => compatibility_line_detail("link", line),
        EvidenceEvent::MarkdownBadge => compatibility_line_detail("badge", line),
        EvidenceEvent::RouteCandidate { source } => {
            compatibility_line_detail(&format!("{source:?}"), line)
        }
    };
    EvidenceSource {
        scanner: scanner.to_string(),
        detail,
    }
}

fn compatibility_line_detail(label: &str, line: Option<usize>) -> String {
    line.map_or_else(|| label.to_string(), |line| format!("{label} line {line}"))
}

fn route_state_routes() -> &'static [RouteKind] {
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

fn build_route_assessment(
    route: RouteKind,
    evidence_facts: &[EvidenceFact],
    evidence_v2: &EvidenceKernelV2,
    pattern_matches: &[PatternMatch],
    readme: Option<&ReadmeSummary>,
) -> Result<RouteAssessment, RouteAssessmentError> {
    let root_structural = route_evidence_ids(evidence_facts, route, |fact| {
        fact.scope == EvidenceScope::Root && is_structural_route_evidence(fact.kind)
    });
    let readme_route = route_evidence_ids(evidence_facts, route, |fact| {
        fact.scope == EvidenceScope::Root
            && fact.path.as_deref().is_some_and(is_root_readme_path)
            && is_readme_route_evidence(fact.kind)
    });
    let inherited = if route == RouteKind::License {
        Vec::new()
    } else {
        evidence_v2
            .facts()
            .iter()
            .filter(|fact| {
                fact.provenance.domain == SourceDomain::OrganizationInherited
                    && fact.atom.route() == Some(route)
            })
            .map(|fact| fact.id)
            .collect()
    };
    let missing_pattern = pattern_matches.iter().any(|pattern_match| {
        pattern_match.route == Some(route) && pattern_match.outcome == PatternOutcome::Missing
    });
    let readme_assessment = readme
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
    evidence_facts: &[EvidenceFact],
    route: RouteKind,
    predicate: impl Fn(&EvidenceFact) -> bool,
) -> Vec<EvidenceId> {
    evidence_facts
        .iter()
        .filter(|fact| fact.route == Some(route) && predicate(fact))
        .map(|fact| fact.id)
        .collect()
}

fn is_structural_route_evidence(kind: EvidenceKind) -> bool {
    matches!(
        kind,
        EvidenceKind::ImportantFile | EvidenceKind::ReadmePresent | EvidenceKind::FilePresent
    )
}

fn is_readme_route_evidence(kind: EvidenceKind) -> bool {
    matches!(
        kind,
        EvidenceKind::MarkdownHeading
            | EvidenceKind::MarkdownLink
            | EvidenceKind::MarkdownBadge
            | EvidenceKind::RouteCandidate
    )
}

fn is_root_readme_path(path: &str) -> bool {
    matches!(path, "README.md" | "Readme.md" | "readme.md" | "README")
}

fn route_for_important_file(kind: ImportantFileKind) -> Option<RouteKind> {
    match kind {
        ImportantFileKind::Readme => Some(RouteKind::Identity),
        ImportantFileKind::License => Some(RouteKind::License),
        ImportantFileKind::Contributing => Some(RouteKind::Contributing),
        ImportantFileKind::Security => Some(RouteKind::Security),
        ImportantFileKind::Support => Some(RouteKind::Support),
        ImportantFileKind::IssueTemplate
        | ImportantFileKind::IssueForm
        | ImportantFileKind::PullRequestTemplate => Some(RouteKind::Intake),
        ImportantFileKind::Changelog => Some(RouteKind::Release),
        ImportantFileKind::Codeowners => Some(RouteKind::Ownership),
        ImportantFileKind::CargoToml => Some(RouteKind::Identity),
        ImportantFileKind::DocsDirectory => Some(RouteKind::Docs),
        ImportantFileKind::Workflow => Some(RouteKind::Automation),
        ImportantFileKind::DependencyBot | ImportantFileKind::SecurityAutomation => {
            Some(RouteKind::Automation)
        }
        ImportantFileKind::Gitignore
        | ImportantFileKind::Gitattributes
        | ImportantFileKind::EditorConfig => Some(RouteKind::Hygiene),
    }
}

fn root_github_operational_file(path: &str, value: &str) -> bool {
    let Ok(kind) = parse_important_file_kind(value) else {
        return false;
    };
    matches!(
        kind,
        ImportantFileKind::IssueTemplate
            | ImportantFileKind::IssueForm
            | ImportantFileKind::PullRequestTemplate
            | ImportantFileKind::DependencyBot
            | ImportantFileKind::SecurityAutomation
    ) && (path.starts_with(".github/") || !path.contains('/'))
}

fn parse_important_file_kind(value: &str) -> Result<ImportantFileKind, ()> {
    match value {
        "IssueTemplate" => Ok(ImportantFileKind::IssueTemplate),
        "IssueForm" => Ok(ImportantFileKind::IssueForm),
        "PullRequestTemplate" => Ok(ImportantFileKind::PullRequestTemplate),
        "DependencyBot" => Ok(ImportantFileKind::DependencyBot),
        "SecurityAutomation" => Ok(ImportantFileKind::SecurityAutomation),
        _ => Err(()),
    }
}
