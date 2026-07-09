use seiri_core::{
    BaselineRequirement, BaselineRuleResult, BaselineStatus, FileKind, GateKind,
    MissingRoutePriority, MissingRoutePriorityReport, MissingRoutePrioritySummary, ProfileKind,
    ProfilePriority, RepoSnapshot, RouteCoOccurrenceGap, RouteKind, RouteState, Severity,
};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn build_missing_route_priority_report(
    snapshot: &RepoSnapshot,
) -> MissingRoutePriorityReport {
    let co_occurrence_gaps = build_co_occurrence_gaps(snapshot);
    let gap_ids_by_route = gap_ids_by_missing_route(&co_occurrence_gaps);
    let finding_gates = finding_gates(snapshot);
    let registry = seiri_patterns::common_registry();

    let mut priorities = Vec::new();
    for route_state in &snapshot.route_states {
        let baseline_rules = missing_baseline_rules(snapshot, route_state.route);
        let candidate_patterns = missing_candidate_patterns(snapshot, &registry, route_state.route);
        let candidate_pattern_ids = candidate_patterns
            .iter()
            .map(|pattern| pattern.id.clone())
            .collect::<Vec<_>>();
        let co_occurrence_gap_ids = gap_ids_by_route
            .get(&route_state.route)
            .cloned()
            .unwrap_or_default();

        if !route_needs_priority(route_state.state)
            && baseline_rules.is_empty()
            && candidate_pattern_ids.is_empty()
            && co_occurrence_gap_ids.is_empty()
        {
            continue;
        }

        let prior = route_gap_prior(route_state.route);
        let gate = strongest_gate_for_route(
            route_state.route,
            route_state.state,
            &baseline_rules,
            &candidate_patterns,
            &co_occurrence_gaps,
            &co_occurrence_gap_ids,
            &finding_gates,
        );
        let profile_bonus = profile_route_bonus(snapshot, route_state.route);
        let priority_score_x100 = priority_score(PriorityScoreInput {
            state: route_state.state,
            prior,
            baseline_rules: &baseline_rules,
            candidate_count: candidate_pattern_ids.len(),
            gaps: &co_occurrence_gaps,
            gap_ids: &co_occurrence_gap_ids,
            gate,
            profile_bonus,
        });
        let baseline_pattern_ids = baseline_rules
            .iter()
            .map(|rule| rule.pattern_id.clone())
            .collect::<Vec<_>>();
        let reason = priority_reason(
            route_state.state,
            prior,
            &baseline_pattern_ids,
            &candidate_pattern_ids,
            &co_occurrence_gap_ids,
            profile_bonus,
        );

        priorities.push(MissingRoutePriority {
            rank: 0,
            route: route_state.route,
            state: route_state.state,
            gate,
            severity: severity_from_score(priority_score_x100),
            priority: priority_from_score(priority_score_x100),
            priority_score_x100,
            observed_missing_repositories: prior.map(|prior| prior.observed_missing_repositories),
            observed_missing_x1000: prior.map(|prior| prior.observed_missing_x1000),
            baseline_pattern_ids,
            candidate_pattern_ids,
            co_occurrence_gap_ids,
            evidence_ids: route_state.evidence_ids.clone(),
            reason,
        });
    }

    priorities.sort_by(|left, right| {
        right
            .priority_score_x100
            .cmp(&left.priority_score_x100)
            .then_with(|| gate_rank(right.gate).cmp(&gate_rank(left.gate)))
            .then_with(|| left.route.cmp(&right.route))
    });
    for (index, priority) in priorities.iter_mut().enumerate() {
        priority.rank = index + 1;
    }

    let summary = MissingRoutePrioritySummary {
        candidates: priorities.len(),
        co_occurrence_gaps: co_occurrence_gaps.len(),
        top_route: priorities.first().map(|priority| priority.route),
        top_priority_x100: priorities
            .first()
            .map(|priority| priority.priority_score_x100),
        safe_gated: priorities
            .iter()
            .filter(|priority| priority.gate == GateKind::Safe)
            .count(),
        guarded_gated: priorities
            .iter()
            .filter(|priority| priority.gate == GateKind::Guarded)
            .count(),
        manual_gated: priorities
            .iter()
            .filter(|priority| priority.gate == GateKind::Manual)
            .count(),
    };

    MissingRoutePriorityReport {
        summary,
        priorities,
        co_occurrence_gaps,
        boundary: "Missing route priority is a deterministic routing hint from observed evidence, fixed 1,000,000-repository calibration priors, and route co-occurrence rules; it is not a popularity, trust, security, quality, or policy guarantee.".to_string(),
    }
}

#[derive(Debug, Clone, Copy)]
struct RouteGapPrior {
    observed_missing_repositories: u32,
    observed_missing_x1000: u16,
    leverage_x100: u8,
}

fn route_gap_prior(route: RouteKind) -> Option<RouteGapPrior> {
    let (observed_missing_repositories, observed_missing_x1000, leverage_x100) = match route {
        RouteKind::Identity => (14_000, 14, 52),
        RouteKind::Docs => (186_000, 186, 30),
        RouteKind::Quickstart => (438_000, 438, 34),
        RouteKind::Support => (503_000, 503, 43),
        RouteKind::Intake => (822_000, 822, 42),
        RouteKind::Contributing => (325_000, 325, 24),
        RouteKind::Security => (558_000, 558, 45),
        RouteKind::Release => (454_000, 454, 32),
        RouteKind::Governance => (787_000, 787, 20),
        RouteKind::License => (80_000, 80, 50),
        RouteKind::Automation => (229_000, 229, 28),
        RouteKind::Ownership => (605_000, 605, 40),
        RouteKind::Hygiene | RouteKind::Unknown => return None,
    };
    Some(RouteGapPrior {
        observed_missing_repositories,
        observed_missing_x1000,
        leverage_x100,
    })
}

#[derive(Debug, Clone, Copy)]
enum ExpectedSignal {
    Route(RouteKind),
    Path(PathSignal),
}

#[derive(Debug, Clone, Copy)]
enum PathSignal {
    IssueFormsYaml,
    IssueTemplates,
    PullRequestTemplate,
    DependencyBotConfig,
    SecurityAutomation,
    ChangelogFile,
}

impl PathSignal {
    fn label(self) -> &'static str {
        match self {
            Self::IssueFormsYaml => "issue_forms_yaml",
            Self::IssueTemplates => "issue_templates",
            Self::PullRequestTemplate => "pull_request_template",
            Self::DependencyBotConfig => "dependency_bot_config",
            Self::SecurityAutomation => "security_automation",
            Self::ChangelogFile => "changelog_file",
        }
    }
}

#[derive(Debug, Clone)]
struct CoOccurrenceRule {
    id: &'static str,
    title: &'static str,
    observed_repositories: u32,
    gate: GateKind,
    expected: &'static [ExpectedSignal],
    reason: &'static str,
}

const BASELINE_OSS_ENTRY: &[ExpectedSignal] = &[
    ExpectedSignal::Route(RouteKind::Identity),
    ExpectedSignal::Route(RouteKind::License),
];

const SUPPORT_INTAKE_CONTROL: &[ExpectedSignal] = &[
    ExpectedSignal::Route(RouteKind::Identity),
    ExpectedSignal::Route(RouteKind::Support),
    ExpectedSignal::Route(RouteKind::Intake),
    ExpectedSignal::Path(PathSignal::IssueTemplates),
    ExpectedSignal::Path(PathSignal::IssueFormsYaml),
];

const SUPPLY_CHAIN_MINIMUM: &[ExpectedSignal] = &[
    ExpectedSignal::Route(RouteKind::Identity),
    ExpectedSignal::Route(RouteKind::Security),
    ExpectedSignal::Route(RouteKind::Automation),
    ExpectedSignal::Path(PathSignal::DependencyBotConfig),
    ExpectedSignal::Path(PathSignal::SecurityAutomation),
];

const RELEASE_READINESS: &[ExpectedSignal] = &[
    ExpectedSignal::Route(RouteKind::Automation),
    ExpectedSignal::Route(RouteKind::Release),
    ExpectedSignal::Path(PathSignal::ChangelogFile),
];

const OWNERSHIP_REVIEW: &[ExpectedSignal] = &[
    ExpectedSignal::Route(RouteKind::Ownership),
    ExpectedSignal::Route(RouteKind::Automation),
    ExpectedSignal::Path(PathSignal::PullRequestTemplate),
];

fn co_occurrence_rules() -> Vec<CoOccurrenceRule> {
    vec![
        CoOccurrenceRule {
            id: "co-README-LICENSE",
            title: "README + LICENSE entry boundary",
            observed_repositories: 905_000,
            gate: GateKind::Manual,
            expected: BASELINE_OSS_ENTRY,
            reason: "README and LICENSE co-occurred in 905,000 repositories in the 1,000,000-level analysis; missing members are high-leverage routing gaps but still require human review for content or legal decisions.",
        },
        CoOccurrenceRule {
            id: "co-README-SUPPORT-ISSUE-FORMS",
            title: "README + Support + Issue forms intake control",
            observed_repositories: 300_000,
            gate: GateKind::Guarded,
            expected: SUPPORT_INTAKE_CONTROL,
            reason: "Support routes and structured issue forms explain how questions, bugs, and feature requests should enter the project without turning every question into a generic issue.",
        },
        CoOccurrenceRule {
            id: "co-README-SECURITY-CI-DEPENDENCY-BOT",
            title: "README + Security + CI + Dependency bot supply-chain route",
            observed_repositories: 260_000,
            gate: GateKind::Guarded,
            expected: SUPPLY_CHAIN_MINIMUM,
            reason: "The analysis identified README, security routing, CI, and dependency bot configuration as a useful supply-chain hygiene combination; RepoSeiri reports missing members as routing evidence only.",
        },
        CoOccurrenceRule {
            id: "co-CI-RELEASE-CHANGELOG",
            title: "CI + Release + Changelog update-risk route",
            observed_repositories: 330_000,
            gate: GateKind::Guarded,
            expected: RELEASE_READINESS,
            reason: "CI, release routing, and changelog evidence help users judge update risk; missing members should be reviewed against the repository purpose.",
        },
        CoOccurrenceRule {
            id: "co-CODEOWNERS-CI-PR-TEMPLATE",
            title: "CODEOWNERS + CI + PR template review route",
            observed_repositories: 240_000,
            gate: GateKind::Manual,
            expected: OWNERSHIP_REVIEW,
            reason: "Ownership, CI, and PR template signals often co-occur in mature review flows; owner assignment and decision rights remain manual.",
        },
    ]
}

fn build_co_occurrence_gaps(snapshot: &RepoSnapshot) -> Vec<RouteCoOccurrenceGap> {
    let mut gaps = Vec::new();
    for rule in co_occurrence_rules() {
        let mut present_routes = BTreeSet::new();
        let mut missing_routes = BTreeSet::new();
        let mut present_signals = Vec::new();
        let mut missing_signals = Vec::new();

        for expected in rule.expected {
            match expected {
                ExpectedSignal::Route(route) => {
                    if route_is_present(snapshot, *route) {
                        present_routes.insert(*route);
                    } else {
                        missing_routes.insert(*route);
                    }
                }
                ExpectedSignal::Path(signal) => {
                    if path_signal_present(snapshot, *signal) {
                        present_signals.push(signal.label().to_string());
                    } else {
                        missing_signals.push(signal.label().to_string());
                    }
                }
            }
        }

        let present_count = present_routes.len() + present_signals.len();
        let missing_count = missing_routes.len() + missing_signals.len();
        if present_count == 0 || missing_count == 0 {
            continue;
        }

        let support_x1000 = ratio_x1000(rule.observed_repositories, 1_000_000);
        gaps.push(RouteCoOccurrenceGap {
            id: rule.id.to_string(),
            title: rule.title.to_string(),
            observed_repositories: rule.observed_repositories,
            support_x1000,
            gate: rule.gate,
            priority: priority_from_support(support_x1000),
            present_routes: present_routes.into_iter().collect(),
            missing_routes: missing_routes.into_iter().collect(),
            present_signals,
            missing_signals,
            reason: rule.reason.to_string(),
        });
    }
    gaps
}

fn gap_ids_by_missing_route(gaps: &[RouteCoOccurrenceGap]) -> BTreeMap<RouteKind, Vec<String>> {
    let mut map = BTreeMap::<RouteKind, Vec<String>>::new();
    for gap in gaps {
        for route in &gap.missing_routes {
            map.entry(*route).or_default().push(gap.id.clone());
        }
    }
    map
}

fn missing_baseline_rules(snapshot: &RepoSnapshot, route: RouteKind) -> Vec<BaselineRuleResult> {
    snapshot
        .baseline
        .as_ref()
        .map(|baseline| {
            baseline
                .rules
                .iter()
                .filter(|rule| rule.route == Some(route) && rule.status == BaselineStatus::Missing)
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MissingCandidatePattern {
    id: String,
    gate: GateKind,
}

fn missing_candidate_patterns(
    snapshot: &RepoSnapshot,
    registry: &seiri_patterns::PatternRegistry,
    route: RouteKind,
) -> Vec<MissingCandidatePattern> {
    registry
        .definitions()
        .iter()
        .filter(|definition| {
            definition.route == Some(route)
                && definition.adoption_stage == seiri_patterns::PatternAdoptionStage::Candidate
                && seiri_patterns::evidence_ids_for_definition(snapshot, definition).is_empty()
        })
        .map(|definition| MissingCandidatePattern {
            id: definition.id.to_string(),
            gate: definition.missing_gate,
        })
        .collect()
}

fn finding_gates(snapshot: &RepoSnapshot) -> BTreeMap<&str, GateKind> {
    snapshot
        .findings
        .iter()
        .filter_map(|finding| {
            finding
                .recommendation
                .as_ref()
                .map(|recommendation| (finding.id.as_str(), recommendation.gate))
        })
        .collect()
}

fn strongest_gate_for_route(
    route: RouteKind,
    state: RouteState,
    baseline_rules: &[BaselineRuleResult],
    candidate_patterns: &[MissingCandidatePattern],
    gaps: &[RouteCoOccurrenceGap],
    gap_ids: &[String],
    finding_gates: &BTreeMap<&str, GateKind>,
) -> GateKind {
    let mut gate = if route_needs_priority(state) || !baseline_rules.is_empty() {
        default_route_gate(route)
    } else {
        GateKind::Safe
    };
    for rule in baseline_rules {
        if let Some(finding_id) = &rule.finding_id {
            if let Some(finding_gate) = finding_gates.get(finding_id.as_str()) {
                gate = strongest_gate(gate, *finding_gate);
            }
        }
    }
    for candidate in candidate_patterns {
        gate = strongest_gate(gate, candidate.gate);
    }
    for gap_id in gap_ids {
        if let Some(gap) = gaps.iter().find(|gap| &gap.id == gap_id) {
            gate = strongest_gate(gate, gap.gate);
        }
    }
    gate
}

struct PriorityScoreInput<'a> {
    state: RouteState,
    prior: Option<RouteGapPrior>,
    baseline_rules: &'a [BaselineRuleResult],
    candidate_count: usize,
    gaps: &'a [RouteCoOccurrenceGap],
    gap_ids: &'a [String],
    gate: GateKind,
    profile_bonus: u8,
}

fn priority_score(input: PriorityScoreInput<'_>) -> u8 {
    let leverage_component = prior_leverage_component(input.prior) as u16;
    let state_component = route_state_component(input.state) as u16;
    let requirement_component = requirement_component(input.baseline_rules) as u16;
    let candidate_component = ((input.candidate_count as u8).saturating_mul(4)).min(12) as u16;
    let co_component = co_occurrence_component(input.gaps, input.gap_ids) as u16;
    let gate_component = gate_component(input.gate) as u16;
    (leverage_component
        + state_component
        + requirement_component
        + candidate_component
        + co_component
        + gate_component
        + u16::from(input.profile_bonus))
    .min(100) as u8
}

fn priority_reason(
    state: RouteState,
    prior: Option<RouteGapPrior>,
    baseline_pattern_ids: &[String],
    candidate_pattern_ids: &[String],
    co_occurrence_gap_ids: &[String],
    profile_bonus: u8,
) -> String {
    let mut parts = vec![format!("route state {state:?}")];
    if let Some(prior) = prior {
        parts.push(format!(
            "analysis gap {} of 1,000,000 repositories",
            prior.observed_missing_repositories
        ));
    }
    if !baseline_pattern_ids.is_empty() {
        parts.push(format!(
            "missing baseline patterns {}",
            baseline_pattern_ids.join(", ")
        ));
    }
    if !candidate_pattern_ids.is_empty() {
        parts.push(format!(
            "missing candidate patterns {}",
            candidate_pattern_ids.join(", ")
        ));
    }
    if !co_occurrence_gap_ids.is_empty() {
        parts.push(format!(
            "co-occurrence gaps {}",
            co_occurrence_gap_ids.join(", ")
        ));
    }
    if profile_bonus > 0 {
        parts.push(format!("profile branch context bonus {profile_bonus}"));
    }
    parts.join("; ")
}

fn route_needs_priority(state: RouteState) -> bool {
    matches!(
        state,
        RouteState::Absent
            | RouteState::Weak
            | RouteState::Inherited
            | RouteState::Conflicting
            | RouteState::Overloaded
            | RouteState::Stale
            | RouteState::UnsafeToInvent
    )
}

fn route_is_present(snapshot: &RepoSnapshot, route: RouteKind) -> bool {
    snapshot.route_states.iter().any(|state| {
        state.route == route
            && matches!(
                state.state,
                RouteState::Routed
                    | RouteState::Structured
                    | RouteState::Verified
                    | RouteState::Overridden
            )
    })
}

fn path_signal_present(snapshot: &RepoSnapshot, signal: PathSignal) -> bool {
    snapshot.files.iter().any(|record| {
        if record.kind != FileKind::File {
            return false;
        }
        let path = record.path.replace('\\', "/").to_ascii_lowercase();
        match signal {
            PathSignal::IssueFormsYaml => {
                path.starts_with(".github/issue_template/")
                    && !path.ends_with("/config.yml")
                    && !path.ends_with("/config.yaml")
                    && (path.ends_with(".yml") || path.ends_with(".yaml"))
            }
            PathSignal::IssueTemplates => {
                path == "issue_template.md"
                    || path == ".github/issue_template.md"
                    || (path.starts_with(".github/issue_template/")
                        && !path.ends_with("/config.yml")
                        && !path.ends_with("/config.yaml")
                        && (path.ends_with(".md")
                            || path.ends_with(".yml")
                            || path.ends_with(".yaml")))
            }
            PathSignal::PullRequestTemplate => {
                path == "pull_request_template.md"
                    || path == ".github/pull_request_template.md"
                    || path.starts_with(".github/pull_request_template/")
            }
            PathSignal::DependencyBotConfig => {
                matches!(
                    path.as_str(),
                    ".github/dependabot.yml"
                        | ".github/dependabot.yaml"
                        | ".github/renovate.json"
                        | "renovate.json"
                        | ".renovaterc"
                        | ".renovaterc.json"
                )
            }
            PathSignal::SecurityAutomation => {
                path.starts_with(".github/workflows/")
                    && (path.ends_with(".yml") || path.ends_with(".yaml"))
                    && path.rsplit('/').next().is_some_and(|name| {
                        name.contains("codeql")
                            || name.contains("security")
                            || name.contains("scorecard")
                            || name.contains("sast")
                            || name.contains("govulncheck")
                            || name.contains("vuln")
                            || name.contains("fuzz")
                    })
            }
            PathSignal::ChangelogFile => {
                matches!(
                    path.as_str(),
                    "changelog" | "changelog.md" | "changes" | "changes.md"
                )
            }
        }
    })
}

fn route_state_component(state: RouteState) -> u8 {
    match state {
        RouteState::UnsafeToInvent => 28,
        RouteState::Absent => 24,
        RouteState::Conflicting | RouteState::Stale => 20,
        RouteState::Weak => 18,
        RouteState::Inherited | RouteState::Overloaded => 16,
        RouteState::Implicit => 12,
        RouteState::Routed
        | RouteState::Structured
        | RouteState::Verified
        | RouteState::Overridden => 0,
    }
}

fn prior_leverage_component(prior: Option<RouteGapPrior>) -> u8 {
    prior.map_or(0, |prior| prior.leverage_x100)
}

fn requirement_component(rules: &[BaselineRuleResult]) -> u8 {
    if rules
        .iter()
        .any(|rule| rule.requirement == BaselineRequirement::Required)
    {
        14
    } else if !rules.is_empty() {
        7
    } else {
        0
    }
}

fn co_occurrence_component(gaps: &[RouteCoOccurrenceGap], gap_ids: &[String]) -> u8 {
    gap_ids
        .iter()
        .filter_map(|gap_id| gaps.iter().find(|gap| &gap.id == gap_id))
        .map(|gap| (gap.support_x1000 / 25).min(18) as u8)
        .max()
        .unwrap_or(0)
}

fn gate_component(gate: GateKind) -> u8 {
    match gate {
        GateKind::Safe => 3,
        GateKind::Guarded => 6,
        GateKind::Manual => 8,
    }
}

fn default_route_gate(route: RouteKind) -> GateKind {
    match route {
        RouteKind::Identity
        | RouteKind::License
        | RouteKind::Security
        | RouteKind::Governance
        | RouteKind::Ownership => GateKind::Manual,
        RouteKind::Quickstart
        | RouteKind::Support
        | RouteKind::Intake
        | RouteKind::Contributing
        | RouteKind::Release
        | RouteKind::Automation
        | RouteKind::Hygiene => GateKind::Guarded,
        RouteKind::Docs | RouteKind::Unknown => GateKind::Safe,
    }
}

fn profile_route_bonus(snapshot: &RepoSnapshot, route: RouteKind) -> u8 {
    let Some(profile) = snapshot.profile.as_ref() else {
        return 0;
    };
    if profile.branch_summary.top_confidence_x100.unwrap_or(0) < 70 {
        return 0;
    }
    let Some(top_profile) = profile.branch_summary.top_profile else {
        return 0;
    };
    let matches_profile = match top_profile {
        ProfileKind::Library => matches!(
            route,
            RouteKind::Docs
                | RouteKind::Quickstart
                | RouteKind::Release
                | RouteKind::Security
                | RouteKind::License
        ),
        ProfileKind::Cli => matches!(
            route,
            RouteKind::Quickstart
                | RouteKind::Support
                | RouteKind::Security
                | RouteKind::Intake
                | RouteKind::Release
                | RouteKind::Automation
        ),
        ProfileKind::Infra => matches!(
            route,
            RouteKind::Docs
                | RouteKind::Support
                | RouteKind::Intake
                | RouteKind::Security
                | RouteKind::Automation
                | RouteKind::Ownership
                | RouteKind::Release
        ),
        ProfileKind::Product => matches!(
            route,
            RouteKind::Support
                | RouteKind::Docs
                | RouteKind::Release
                | RouteKind::Quickstart
                | RouteKind::Intake
        ),
        ProfileKind::Runtime => matches!(
            route,
            RouteKind::Security
                | RouteKind::Release
                | RouteKind::Governance
                | RouteKind::Automation
                | RouteKind::Ownership
        ),
        ProfileKind::Docs => {
            matches!(
                route,
                RouteKind::Docs | RouteKind::Contributing | RouteKind::Governance
            )
        }
        ProfileKind::Tutorial => {
            matches!(
                route,
                RouteKind::Quickstart | RouteKind::Docs | RouteKind::Support
            )
        }
        ProfileKind::Ml | ProfileKind::Research => {
            matches!(
                route,
                RouteKind::Docs | RouteKind::License | RouteKind::Quickstart
            )
        }
        ProfileKind::Template => {
            matches!(
                route,
                RouteKind::Quickstart | RouteKind::Automation | RouteKind::Release
            )
        }
        ProfileKind::Common => false,
    };
    if matches_profile {
        10
    } else {
        0
    }
}

fn priority_from_score(score: u8) -> ProfilePriority {
    match score {
        75..=100 => ProfilePriority::Critical,
        55..=74 => ProfilePriority::High,
        35..=54 => ProfilePriority::Normal,
        _ => ProfilePriority::Low,
    }
}

fn severity_from_score(score: u8) -> Severity {
    match score {
        75..=100 => Severity::High,
        55..=74 => Severity::Medium,
        35..=54 => Severity::Low,
        _ => Severity::Info,
    }
}

fn priority_from_support(support_x1000: u16) -> ProfilePriority {
    match support_x1000 {
        500..=1000 => ProfilePriority::Critical,
        300..=499 => ProfilePriority::High,
        150..=299 => ProfilePriority::Normal,
        _ => ProfilePriority::Low,
    }
}

fn strongest_gate(left: GateKind, right: GateKind) -> GateKind {
    if gate_rank(right) > gate_rank(left) {
        right
    } else {
        left
    }
}

fn gate_rank(gate: GateKind) -> u8 {
    match gate {
        GateKind::Safe => 0,
        GateKind::Guarded => 1,
        GateKind::Manual => 2,
    }
}

fn ratio_x1000(numerator: u32, denominator: u32) -> u16 {
    if denominator == 0 {
        return 0;
    }
    numerator
        .saturating_mul(1000)
        .checked_div(denominator)
        .unwrap_or(0)
        .min(1000) as u16
}
