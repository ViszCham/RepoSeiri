use seiri_core::{
    ReadmeRouteMap, ReadmeRouteMapEntry, ReadmeRouteMapSummary, ReadmeRouteTarget,
    ReadmeRouteTargetStatus, RouteCandidate, RouteKind, RouteSource, RouteState,
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
        let state = readme_route_state(
            route,
            candidate_count,
            target_count,
            stale_target_count,
            conflicting_target_count,
        );

        entries.push(ReadmeRouteMapEntry {
            route,
            state,
            observed_gap_count: observed_gap_count(route),
            candidate_count,
            heading_count,
            link_count,
            badge_count,
            target_count,
            stale_target_count,
            conflicting_target_count,
            evidence_lines,
            targets,
            reason: readme_route_reason(route, state).to_string(),
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
        RouteKind::Docs,
        RouteKind::Quickstart,
        RouteKind::Support,
        RouteKind::Intake,
        RouteKind::Security,
        RouteKind::Release,
        RouteKind::Governance,
        RouteKind::Contributing,
        RouteKind::License,
        RouteKind::Automation,
        RouteKind::Hygiene,
    ]
}

fn readme_route_state(
    route: RouteKind,
    candidate_count: usize,
    target_count: usize,
    stale_target_count: usize,
    conflicting_target_count: usize,
) -> RouteState {
    if candidate_count == 0 {
        RouteState::Absent
    } else if stale_target_count > 0 {
        RouteState::Stale
    } else if conflicting_target_count > 0 {
        RouteState::Conflicting
    } else if candidate_count >= 4 || target_count >= 4 {
        RouteState::Overloaded
    } else if target_count == 0 && !heading_only_route_is_actionable(route) {
        RouteState::Weak
    } else if target_count > 0 {
        RouteState::Verified
    } else {
        RouteState::Routed
    }
}

fn heading_only_route_is_actionable(route: RouteKind) -> bool {
    matches!(route, RouteKind::Quickstart)
}

fn observed_gap_count(route: RouteKind) -> Option<u32> {
    match route {
        RouteKind::Docs => Some(186_000),
        RouteKind::Quickstart => Some(438_000),
        RouteKind::Support => Some(503_000),
        RouteKind::Intake => Some(822_000),
        RouteKind::Release => Some(454_000),
        _ => None,
    }
}

fn readme_route_reason(route: RouteKind, state: RouteState) -> &'static str {
    match state {
        RouteState::Absent => "No README evidence was observed for this route.",
        RouteState::Weak => "README route evidence is visible but does not expose a target.",
        RouteState::Conflicting => {
            "README links reuse a target across multiple route kinds, so route intent is ambiguous."
        }
        RouteState::Overloaded => {
            "README exposes many entries for this route; users may need a clearer single path."
        }
        RouteState::Stale => "README links to a local target that was not found in the repository.",
        RouteState::Verified if route == RouteKind::Quickstart => {
            "README exposes a reachable first-run path."
        }
        RouteState::Verified => "README exposes a reachable route target.",
        RouteState::Routed => "README exposes this route inside the README.",
        _ => "README route map emitted this state from observed route evidence.",
    }
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
