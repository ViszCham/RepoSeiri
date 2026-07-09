use seiri_core::{
    stable_id, Evidence, EvidenceConfidence, EvidenceKind, EvidenceRecord, EvidenceScope,
    EvidenceSource, EvidenceSpan, ImportantFileKind, PatternMatch, PatternOutcome, ReadmeSummary,
    RouteKind, RouteState, RouteStateReport,
};

pub(crate) fn build_evidence_ledger(evidence: &[Evidence]) -> Vec<EvidenceRecord> {
    evidence
        .iter()
        .enumerate()
        .map(|(index, legacy)| {
            let scope = evidence_scope(legacy);
            EvidenceRecord {
                id: stable_id("evrec", index + 1),
                legacy_evidence_id: Some(legacy.id.clone()),
                kind: legacy.kind,
                path: legacy.path.clone(),
                route: legacy.route,
                value: legacy.value.clone(),
                source: legacy.source.clone(),
                scope,
                confidence: evidence_confidence(legacy, scope),
                span: evidence_span(&legacy.source),
            }
        })
        .collect()
}

pub(crate) fn build_route_states(
    evidence_ledger: &[EvidenceRecord],
    pattern_matches: &[PatternMatch],
    readme: Option<&ReadmeSummary>,
) -> Vec<RouteStateReport> {
    route_state_routes()
        .iter()
        .map(|route| build_route_state(*route, evidence_ledger, pattern_matches, readme))
        .collect()
}

fn evidence_scope(evidence: &Evidence) -> EvidenceScope {
    let Some(path) = evidence.path.as_deref() else {
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
            && evidence.kind == EvidenceKind::ImportantFile
            && evidence.value == "Workflow")
        || (lower == ".github/codeowners"
            && evidence.kind == EvidenceKind::ImportantFile
            && evidence.value == "Codeowners")
        || (evidence.kind == EvidenceKind::ImportantFile
            && root_github_operational_file(&lower, &evidence.value))
    {
        return EvidenceScope::Root;
    }

    EvidenceScope::Nested
}

fn evidence_confidence(evidence: &Evidence, scope: EvidenceScope) -> EvidenceConfidence {
    if !matches!(scope, EvidenceScope::Root) {
        return EvidenceConfidence::Low;
    }

    match evidence.kind {
        EvidenceKind::FilePresent
        | EvidenceKind::ImportantFile
        | EvidenceKind::ReadmePresent
        | EvidenceKind::ReadmeMissing => EvidenceConfidence::High,
        EvidenceKind::MarkdownHeading
        | EvidenceKind::MarkdownLink
        | EvidenceKind::MarkdownBadge => EvidenceConfidence::Medium,
        EvidenceKind::RouteCandidate => EvidenceConfidence::Medium,
    }
}

fn evidence_span(source: &EvidenceSource) -> Option<EvidenceSpan> {
    let (_, tail) = source.detail.rsplit_once("line ")?;
    let digits = tail
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect::<String>();
    let line = digits.parse::<usize>().ok()?;
    Some(EvidenceSpan {
        start_line: line,
        end_line: line,
    })
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

fn build_route_state(
    route: RouteKind,
    evidence_ledger: &[EvidenceRecord],
    pattern_matches: &[PatternMatch],
    readme: Option<&ReadmeSummary>,
) -> RouteStateReport {
    let root_structural = route_evidence_ids(evidence_ledger, route, |record| {
        record.scope == EvidenceScope::Root && is_structural_route_evidence(record.kind)
    });
    let readme_route = route_evidence_ids(evidence_ledger, route, |record| {
        record.scope == EvidenceScope::Root && is_readme_route_evidence(record.kind)
    });
    let inherited = route_evidence_ids(evidence_ledger, route, |record| {
        !matches!(record.scope, EvidenceScope::Root) && record.route == Some(route)
    });
    let missing_pattern = pattern_matches.iter().any(|pattern_match| {
        pattern_match.route == Some(route) && pattern_match.outcome == PatternOutcome::Missing
    });
    let readme_map_state = readme
        .and_then(|readme| {
            readme
                .route_map
                .entries
                .iter()
                .find(|entry| entry.route == route)
        })
        .map(|entry| (entry.state, entry.reason.as_str()));

    let mut evidence_ids = Vec::new();
    evidence_ids.extend(root_structural.iter().cloned());
    evidence_ids.extend(readme_route.iter().cloned());
    evidence_ids.sort();
    evidence_ids.dedup();

    let (state, confidence, reason) = if let Some((RouteState::Stale, reason)) = readme_map_state {
        (RouteState::Stale, EvidenceConfidence::Medium, reason)
    } else if let Some((RouteState::Conflicting, reason)) = readme_map_state {
        (RouteState::Conflicting, EvidenceConfidence::Medium, reason)
    } else if let Some((RouteState::Overloaded, reason)) = readme_map_state {
        (RouteState::Overloaded, EvidenceConfidence::Medium, reason)
    } else if root_structural.is_empty() && matches!(readme_map_state, Some((RouteState::Weak, _)))
    {
        let reason = readme_map_state
            .map(|(_, reason)| reason)
            .unwrap_or("README route evidence is weak.");
        (RouteState::Weak, EvidenceConfidence::Low, reason)
    } else if !root_structural.is_empty() && !readme_route.is_empty() {
        (
            RouteState::Verified,
            EvidenceConfidence::High,
            "Root structured evidence and README routing evidence agree.",
        )
    } else if !root_structural.is_empty() {
        (
            RouteState::Structured,
            EvidenceConfidence::High,
            "Root structured evidence is present, but README routing is not explicit.",
        )
    } else if !readme_route.is_empty() {
        (
            RouteState::Routed,
            EvidenceConfidence::Medium,
            "README routing evidence is present.",
        )
    } else if !inherited.is_empty() {
        evidence_ids = inherited;
        (
            RouteState::Inherited,
            EvidenceConfidence::Low,
            "Only non-root or fixture evidence was observed; it is not credited as a root route.",
        )
    } else if missing_pattern && unsafe_to_invent_route(route) {
        (
            RouteState::UnsafeToInvent,
            EvidenceConfidence::Medium,
            "The route is missing and requires a maintainer policy or content decision.",
        )
    } else {
        (
            RouteState::Absent,
            EvidenceConfidence::Low,
            "No root route evidence was observed.",
        )
    };

    RouteStateReport {
        route,
        state,
        evidence_ids,
        confidence,
        reason: reason.to_string(),
    }
}

fn route_evidence_ids(
    evidence_ledger: &[EvidenceRecord],
    route: RouteKind,
    predicate: impl Fn(&EvidenceRecord) -> bool,
) -> Vec<String> {
    evidence_ledger
        .iter()
        .filter(|record| record.route == Some(route) && predicate(record))
        .map(|record| record.id.clone())
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

fn unsafe_to_invent_route(route: RouteKind) -> bool {
    matches!(
        route,
        RouteKind::License
            | RouteKind::Security
            | RouteKind::Lifecycle
            | RouteKind::Governance
            | RouteKind::Ownership
    )
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
