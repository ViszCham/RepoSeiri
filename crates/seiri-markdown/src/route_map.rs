use seiri_core::{
    ReadmeRouteAssessment, ReadmeRouteMap, ReadmeRouteMapEntry, ReadmeRouteMapSummary,
    ReadmeRouteTarget, ReadmeRouteTargetStatus, RouteCandidate, RouteKind, RouteSource, RouteState,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub(crate) fn build_route_map(
    route_candidates: &[RouteCandidate],
    repo_root: Option<&Path>,
) -> ReadmeRouteMap {
    let target_routes = target_routes(route_candidates);
    let mut entries = Vec::new();

    for &route in readme_hub_routes() {
        let route_candidates = route_candidates
            .iter()
            .filter(|candidate| candidate.route == route)
            .collect::<Vec<_>>();
        let heading_count = route_candidates
            .iter()
            .filter(|candidate| candidate.source == RouteSource::Heading)
            .count();
        let link_count = route_candidates
            .iter()
            .filter(|candidate| candidate.source == RouteSource::Link)
            .count();
        let badge_count = route_candidates
            .iter()
            .filter(|candidate| candidate.source == RouteSource::Badge)
            .count();
        let evidence_lines = route_candidates
            .iter()
            .map(|candidate| candidate.line)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let targets = route_targets(&route_candidates, &target_routes, repo_root);
        let target_count = targets
            .iter()
            .map(|target| target.target.as_str())
            .collect::<BTreeSet<_>>()
            .len();
        let stale_target_count = targets
            .iter()
            .filter(|target| target.status == ReadmeRouteTargetStatus::LocalMissing)
            .count();
        let conflicting_target_count = targets
            .iter()
            .filter(|target| target.routes.len() > 1)
            .count();
        let candidate_count = route_candidates.len();
        let assessment = ReadmeRouteAssessment::from_observations(
            candidate_count,
            heading_count,
            link_count,
            badge_count,
            target_count,
            &targets,
        )
        .expect("README route observations must form a valid assessment");
        let state = assessment.legacy_state(route);

        entries.push(ReadmeRouteMapEntry {
            route,
            assessment,
            state,
            gap_estimate: None,
            candidate_count,
            heading_count,
            link_count,
            badge_count,
            target_count,
            stale_target_count,
            conflicting_target_count,
            evidence_lines,
            targets,
            reason: assessment.legacy_reason(route).to_string(),
        });
    }

    let summary = ReadmeRouteMapSummary {
        routes: entries.len(),
        routed: entries
            .iter()
            .filter(|entry| matches!(entry.state, RouteState::Routed | RouteState::Verified))
            .count(),
        weak: entries
            .iter()
            .filter(|entry| entry.state == RouteState::Weak)
            .count(),
        conflicting: entries
            .iter()
            .filter(|entry| entry.state == RouteState::Conflicting)
            .count(),
        overloaded: entries
            .iter()
            .filter(|entry| entry.state == RouteState::Overloaded)
            .count(),
        stale: entries
            .iter()
            .filter(|entry| entry.state == RouteState::Stale)
            .count(),
        absent: entries
            .iter()
            .filter(|entry| entry.state == RouteState::Absent)
            .count(),
    };

    ReadmeRouteMap { summary, entries }
}

fn target_routes(route_candidates: &[RouteCandidate]) -> BTreeMap<String, Vec<RouteKind>> {
    let mut map = BTreeMap::<String, BTreeSet<RouteKind>>::new();
    for candidate in route_candidates {
        if let Some(target) = &candidate.target {
            map.entry(normalize_target_key(target))
                .or_default()
                .insert(candidate.route);
        }
    }
    map.into_iter()
        .map(|(target, routes)| (target, routes.into_iter().collect()))
        .collect()
}

fn route_targets(
    route_candidates: &[&RouteCandidate],
    target_routes: &BTreeMap<String, Vec<RouteKind>>,
    repo_root: Option<&Path>,
) -> Vec<ReadmeRouteTarget> {
    let mut targets = Vec::new();
    for candidate in route_candidates {
        let Some(target) = &candidate.target else {
            continue;
        };
        let target_key = normalize_target_key(target);
        targets.push(ReadmeRouteTarget {
            target: target.clone(),
            line: candidate.line,
            source: candidate.source,
            status: classify_target_status(target, repo_root),
            routes: target_routes.get(&target_key).cloned().unwrap_or_default(),
        });
    }
    targets.sort_by(|left, right| {
        left.target
            .cmp(&right.target)
            .then_with(|| left.line.cmp(&right.line))
            .then_with(|| left.source.cmp(&right.source))
    });
    targets
}

fn readme_hub_routes() -> &'static [RouteKind] {
    &[
        RouteKind::Identity,
        RouteKind::Docs,
        RouteKind::Quickstart,
        RouteKind::Support,
        RouteKind::Intake,
        RouteKind::Security,
        RouteKind::Release,
        RouteKind::Lifecycle,
        RouteKind::Governance,
        RouteKind::Contributing,
        RouteKind::License,
        RouteKind::Automation,
        RouteKind::Ownership,
        RouteKind::Hygiene,
    ]
}

fn classify_target_status(target: &str, repo_root: Option<&Path>) -> ReadmeRouteTargetStatus {
    let trimmed = target.trim();
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return ReadmeRouteTargetStatus::External;
    }
    if lower.starts_with("mailto:") {
        return ReadmeRouteTargetStatus::Mail;
    }
    if trimmed.starts_with('#') {
        return ReadmeRouteTargetStatus::Anchor;
    }

    let local = strip_target_fragment(trimmed);
    if local.is_empty() {
        return ReadmeRouteTargetStatus::Anchor;
    }
    let Some(repo_root) = repo_root else {
        return ReadmeRouteTargetStatus::Unknown;
    };
    if repo_root.join(local).exists() {
        ReadmeRouteTargetStatus::LocalPresent
    } else {
        ReadmeRouteTargetStatus::LocalMissing
    }
}

fn normalize_target_key(target: &str) -> String {
    strip_target_fragment(target.trim())
        .replace('\\', "/")
        .to_ascii_lowercase()
}

fn strip_target_fragment(target: &str) -> &str {
    target.split(['#', '?']).next().unwrap_or(target).trim()
}
