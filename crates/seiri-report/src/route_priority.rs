use seiri_core::{
    BaselineRequirement, BaselineRuleResult, BaselineStatus, CalibrationKey, CalibrationLookup,
    CalibrationPriorState, CalibrationProvider, CoverageScope, CoverageStatus, FileKind, GateKind,
    MissingRoutePriority, MissingRoutePriorityReport, MissingRoutePrioritySummary, ProfileKind,
    ProfilePriority, RepositoryAnalysis, ReviewGap, ReviewPriority, ReviewPriorityReport,
    RouteAvailability, RouteCoOccurrenceGap, RouteKind, RouteState, Severity,
};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn build_missing_route_priority_report(
    snapshot: &RepositoryAnalysis,
    calibration: &dyn CalibrationProvider,
) -> MissingRoutePriorityReport {
    let co_occurrence_gaps = build_co_occurrence_gaps(snapshot, calibration);
    let gap_ids_by_route = gap_ids_by_missing_route(&co_occurrence_gaps);
    let finding_gates = finding_gates(snapshot);
    let registry = seiri_patterns::common_registry();

    let mut priorities = Vec::new();
    for assessment in &snapshot.route_assessments {
        let route = assessment.route();
        let summary = assessment.summary_projection();
        let baseline_rules = missing_baseline_rules(snapshot, route);
        let candidate_patterns = missing_candidate_patterns(snapshot, &registry, route);
        let candidate_pattern_ids = candidate_patterns
            .iter()
            .map(|pattern| pattern.id.clone())
            .collect::<Vec<_>>();
        let co_occurrence_gap_ids = gap_ids_by_route.get(&route).cloned().unwrap_or_default();

        if !route_needs_priority(summary.state)
            && baseline_rules.is_empty()
            && candidate_pattern_ids.is_empty()
            && co_occurrence_gap_ids.is_empty()
        {
            continue;
        }

        let prior = route_gap_prior(calibration, route);
        let gate = strongest_gate_for_route(
            route,
            summary.state,
            &baseline_rules,
            &candidate_patterns,
            &co_occurrence_gaps,
            &co_occurrence_gap_ids,
            &finding_gates,
        );
        let profile_bonus = profile_route_bonus(snapshot, route);
        let priority_score_x100 = priority_score(PriorityScoreInput {
            state: summary.state,
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
            summary.state,
            prior,
            &baseline_pattern_ids,
            &candidate_pattern_ids,
            &co_occurrence_gap_ids,
            profile_bonus,
        );

        priorities.push(MissingRoutePriority {
            rank: 0,
            route,
            state: summary.state,
            gate,
            severity: severity_from_score(priority_score_x100),
            priority: priority_from_score(priority_score_x100),
            priority_score_x100,
            calibration_estimate: None,
            baseline_pattern_ids,
            candidate_pattern_ids,
            co_occurrence_gap_ids,
            evidence_ids: assessment.summary_evidence_ids(),
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

    let top_route_gap = priorities.iter().find(|priority| {
        route_needs_priority(priority.state) || !priority.baseline_pattern_ids.is_empty()
    });
    let summary = MissingRoutePrioritySummary {
        candidates: priorities.len(),
        co_occurrence_gaps: co_occurrence_gaps.len(),
        top_route: top_route_gap.map(|priority| priority.route),
        top_priority_x100: top_route_gap.map(|priority| priority.priority_score_x100),
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
        boundary: "This report retains route, candidate-pattern, and co-occurrence review items. Standard audit uses no aggregate prior. An explicitly supplied local prior may affect internal ranking, but its source path, exact values, and raw body are not serialized. Rankings do not guarantee popularity, trust, security, quality, or policy outcomes.".to_string(),
    }
}

pub(crate) fn build_review_priority_report(
    routes: &MissingRoutePriorityReport,
    content: &seiri_core::RouteContentReport,
) -> ReviewPriorityReport {
    let mut priorities = Vec::new();
    for item in &routes.priorities {
        if route_needs_priority(item.state) || !item.baseline_pattern_ids.is_empty() {
            priorities.push(review_priority(
                item,
                ReviewGap::Route {
                    route: item.route,
                    state: item.state,
                    baseline_pattern_ids: item.baseline_pattern_ids.clone(),
                },
            ));
        }
        if !item.candidate_pattern_ids.is_empty() {
            priorities.push(review_priority(
                item,
                ReviewGap::Content {
                    route: item.route,
                    candidate_pattern_ids: item.candidate_pattern_ids.clone(),
                },
            ));
        }
        if !item.co_occurrence_gap_ids.is_empty() {
            priorities.push(review_priority(
                item,
                ReviewGap::Consistency {
                    route: Some(item.route),
                    gap_ids: item.co_occurrence_gap_ids.clone(),
                },
            ));
        }
    }
    for assessment in &content.assessments {
        if assessment.enabled
            && matches!(
                assessment.observation,
                seiri_core::Observation::Absent { .. }
            )
        {
            let (gate, severity, priority, score) = content_slot_priority(assessment.sensitivity);
            priorities.push(ReviewPriority {
                rank: 0,
                gap: ReviewGap::ContentSlot {
                    route: assessment.route,
                    slot_ids: vec![assessment.slot],
                },
                gate,
                severity,
                priority,
                priority_score_x100: score,
                calibration_estimate: None,
                evidence_ids: assessment.condition_evidence_ids.clone(),
                reason: format!(
                    "Content slot '{}' is absent under complete bounded coverage; this is separate from route presence.",
                    assessment.code
                ),
            });
        }
    }
    priorities.sort_by(|left, right| {
        right
            .priority_score_x100
            .cmp(&left.priority_score_x100)
            .then_with(|| left.gap.route().cmp(&right.gap.route()))
    });
    for (index, priority) in priorities.iter_mut().enumerate() {
        priority.rank = index + 1;
    }
    ReviewPriorityReport::new(priorities)
}

fn content_slot_priority(
    sensitivity: seiri_core::PolicySensitivityWire,
) -> (GateKind, Severity, ProfilePriority, u8) {
    match sensitivity {
        seiri_core::PolicySensitivityWire::SecuritySensitive => (
            GateKind::Manual,
            Severity::High,
            ProfilePriority::Critical,
            95,
        ),
        seiri_core::PolicySensitivityWire::LegalSensitive => (
            GateKind::Manual,
            Severity::High,
            ProfilePriority::Critical,
            92,
        ),
        seiri_core::PolicySensitivityWire::MaintainerDecision => (
            GateKind::Manual,
            Severity::Medium,
            ProfilePriority::High,
            82,
        ),
        seiri_core::PolicySensitivityWire::ExecutionSensitive => (
            GateKind::Guarded,
            Severity::Medium,
            ProfilePriority::High,
            76,
        ),
        seiri_core::PolicySensitivityWire::EvidenceOnly => {
            (GateKind::Safe, Severity::Low, ProfilePriority::Normal, 55)
        }
    }
}

fn review_priority(item: &MissingRoutePriority, gap: ReviewGap) -> ReviewPriority {
    ReviewPriority {
        rank: 0,
        gap,
        gate: item.gate,
        severity: item.severity,
        priority: item.priority,
        priority_score_x100: item.priority_score_x100,
        calibration_estimate: item.calibration_estimate,
        evidence_ids: item.evidence_ids.clone(),
        reason: item.reason.clone(),
    }
}

#[derive(Debug, Clone, Copy)]
struct RouteGapPrior {
    leverage_x100: u8,
    state: CalibrationPriorState,
}

fn route_gap_prior(
    calibration: &dyn CalibrationProvider,
    route: RouteKind,
) -> Option<RouteGapPrior> {
    match calibration.prior(&CalibrationKey::RouteGap(route)) {
        CalibrationLookup::NotRequested => None,
        CalibrationLookup::Available(prior) => Some(RouteGapPrior {
            leverage_x100: prior.rank_weight_x100(),
            state: CalibrationPriorState::AppliedRedacted,
        }),
        CalibrationLookup::Unavailable(_) => Some(RouteGapPrior {
            leverage_x100: 0,
            state: CalibrationPriorState::Unavailable,
        }),
    }
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
            gate: GateKind::Manual,
            expected: BASELINE_OSS_ENTRY,
            reason: "README and LICENSE form a useful entry-boundary review rule; missing members still require human review for content or legal decisions.",
        },
        CoOccurrenceRule {
            id: "co-README-SUPPORT-ISSUE-FORMS",
            title: "README + Support + Issue forms intake control",
            gate: GateKind::Guarded,
            expected: SUPPORT_INTAKE_CONTROL,
            reason: "Support routes and structured issue forms explain how questions, bugs, and feature requests should enter the project without turning every question into a generic issue.",
        },
        CoOccurrenceRule {
            id: "co-README-SECURITY-CI-DEPENDENCY-BOT",
            title: "README + Security + CI + Dependency bot supply-chain route",
            gate: GateKind::Guarded,
            expected: SUPPLY_CHAIN_MINIMUM,
            reason: "The analysis identified README, security routing, CI, and dependency bot configuration as a useful supply-chain hygiene combination; RepoSeiri reports missing members as routing evidence only.",
        },
        CoOccurrenceRule {
            id: "co-CI-RELEASE-CHANGELOG",
            title: "CI + Release + Changelog update-risk route",
            gate: GateKind::Guarded,
            expected: RELEASE_READINESS,
            reason: "CI, release routing, and changelog evidence help users judge update risk; missing members should be reviewed against the repository purpose.",
        },
        CoOccurrenceRule {
            id: "co-CODEOWNERS-CI-PR-TEMPLATE",
            title: "CODEOWNERS + CI + PR template review route",
            gate: GateKind::Manual,
            expected: OWNERSHIP_REVIEW,
            reason: "Ownership, CI, and PR template signals often co-occur in mature review flows; owner assignment and decision rights remain manual.",
        },
    ]
}

fn build_co_occurrence_gaps(
    snapshot: &RepositoryAnalysis,
    calibration: &dyn CalibrationProvider,
) -> Vec<RouteCoOccurrenceGap> {
    let mut gaps = Vec::new();
    for rule in co_occurrence_rules() {
        let mut present_routes = BTreeSet::new();
        let mut degraded_routes = BTreeSet::new();
        let mut missing_routes = BTreeSet::new();
        let mut unknown_routes = BTreeSet::new();
        let mut present_signals = Vec::new();
        let mut missing_signals = Vec::new();

        for expected in rule.expected {
            match expected {
                ExpectedSignal::Route(route) => match route_availability(snapshot, *route) {
                    RouteAvailability::Present => {
                        present_routes.insert(*route);
                    }
                    RouteAvailability::Degraded => {
                        degraded_routes.insert(*route);
                    }
                    RouteAvailability::Absent => {
                        missing_routes.insert(*route);
                    }
                    RouteAvailability::Unknown => {
                        unknown_routes.insert(*route);
                    }
                },
                ExpectedSignal::Path(signal) => {
                    if path_signal_present(snapshot, *signal) {
                        present_signals.push(signal.label().to_string());
                    } else {
                        missing_signals.push(signal.label().to_string());
                    }
                }
            }
        }

        let present_count = present_routes.len() + degraded_routes.len() + present_signals.len();
        let missing_count = missing_routes.len() + missing_signals.len();
        if present_count == 0 || missing_count == 0 {
            continue;
        }

        let (rank_weight_x100, calibration_prior) =
            match calibration.prior(&CalibrationKey::CoOccurrence(rule.id.into())) {
                CalibrationLookup::NotRequested => (0, CalibrationPriorState::NotRequested),
                CalibrationLookup::Available(prior) => (
                    prior.rank_weight_x100(),
                    CalibrationPriorState::AppliedRedacted,
                ),
                CalibrationLookup::Unavailable(_) => (0, CalibrationPriorState::Unavailable),
            };
        gaps.push(RouteCoOccurrenceGap {
            id: rule.id.to_string(),
            title: rule.title.to_string(),
            calibration_estimate: None,
            support_x1000: 0,
            rank_weight_x100,
            calibration_prior,
            gate: rule.gate,
            priority: priority_from_score(rank_weight_x100),
            present_routes: present_routes.into_iter().collect(),
            degraded_routes: degraded_routes.into_iter().collect(),
            missing_routes: missing_routes.into_iter().collect(),
            unknown_routes: unknown_routes.into_iter().collect(),
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

fn missing_baseline_rules(
    snapshot: &RepositoryAnalysis,
    route: RouteKind,
) -> Vec<BaselineRuleResult> {
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
    snapshot: &RepositoryAnalysis,
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
            gate: definition.boundary.missing_gate,
        })
        .collect()
}

fn finding_gates(snapshot: &RepositoryAnalysis) -> BTreeMap<&str, GateKind> {
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
        match prior.state {
            CalibrationPriorState::AppliedRedacted => {
                parts.push("explicit local calibration prior applied with values redacted".into());
            }
            CalibrationPriorState::Unavailable => {
                parts.push("explicit local calibration prior unavailable for this route".into());
            }
            CalibrationPriorState::NotRequested => {}
        }
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

fn route_availability(snapshot: &RepositoryAnalysis, route: RouteKind) -> RouteAvailability {
    let Some(assessment) = snapshot
        .route_assessments
        .iter()
        .find(|assessment| assessment.route() == route)
    else {
        return RouteAvailability::Unknown;
    };
    let availability = assessment.condition().availability;
    if availability == RouteAvailability::Absent
        && snapshot
            .coverage
            .record(CoverageScope::RootReadme)
            .is_none_or(|record| record.status != CoverageStatus::Complete)
    {
        RouteAvailability::Unknown
    } else {
        availability
    }
}

fn path_signal_present(snapshot: &RepositoryAnalysis, signal: PathSignal) -> bool {
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
        .map(|gap| (gap.rank_weight_x100 / 5).min(18))
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
        | RouteKind::Lifecycle
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

fn profile_route_bonus(snapshot: &RepositoryAnalysis, route: RouteKind) -> u8 {
    let Some(profile) = snapshot.profile.as_ref() else {
        return 0;
    };
    let Some(top_profile) = profile.branch_summary.top_profile else {
        return 0;
    };
    let Some(top_branch) = profile
        .branches
        .iter()
        .find(|branch| branch.profile == top_profile)
    else {
        return 0;
    };
    if top_branch.semantics.rank_score.get() < 70 {
        return 0;
    }
    let matches_profile = match top_profile {
        ProfileKind::Library => matches!(
            route,
            RouteKind::Docs
                | RouteKind::Quickstart
                | RouteKind::Release
                | RouteKind::Lifecycle
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
                | RouteKind::Lifecycle
                | RouteKind::Release
        ),
        ProfileKind::Product => matches!(
            route,
            RouteKind::Support
                | RouteKind::Docs
                | RouteKind::Release
                | RouteKind::Lifecycle
                | RouteKind::Quickstart
                | RouteKind::Intake
        ),
        ProfileKind::Runtime => matches!(
            route,
            RouteKind::Security
                | RouteKind::Release
                | RouteKind::Lifecycle
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
                RouteKind::Quickstart
                    | RouteKind::Automation
                    | RouteKind::Release
                    | RouteKind::Lifecycle
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
