use seiri_core::{
    BaselineReport, BaselineRuleResult, BaselineStatus, Finding, GateKind, ProfileKind,
    ProfilePriority, ProfileRecommendation, ProfileReport, ProfileRuleResult, ProfileScoreView,
    RepoSnapshot, Severity,
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileRuleDefinition {
    pub pattern_id: &'static str,
    pub weight: u32,
    pub priority: ProfilePriority,
    pub reason: &'static str,
}

#[must_use]
pub fn evaluate_profile(snapshot: &RepoSnapshot, profile: ProfileKind) -> Option<ProfileReport> {
    let baseline = snapshot.baseline.as_ref()?;
    Some(evaluate_profile_from_parts(
        baseline,
        &snapshot.findings,
        profile,
    ))
}

#[must_use]
pub fn evaluate_profile_from_parts(
    baseline: &BaselineReport,
    findings: &[Finding],
    profile: ProfileKind,
) -> ProfileReport {
    let baseline_by_pattern = baseline
        .rules
        .iter()
        .map(|rule| (rule.pattern_id.as_str(), rule))
        .collect::<BTreeMap<_, _>>();
    let findings_by_id = findings
        .iter()
        .map(|finding| (finding.id.as_str(), finding))
        .collect::<BTreeMap<_, _>>();

    let mut rules = Vec::new();
    for (index, definition) in profile_rules(profile).iter().enumerate() {
        let baseline_rule = baseline_by_pattern.get(definition.pattern_id).copied();
        rules.push(to_profile_rule_result(
            index + 1,
            profile,
            definition,
            baseline_rule,
        ));
    }

    let score = score_view(&rules);
    let recommendations = ordered_recommendations(&rules, &findings_by_id);

    ProfileReport {
        profile,
        score,
        rules,
        recommendations,
    }
}

#[must_use]
pub fn profile_rules(profile: ProfileKind) -> Vec<ProfileRuleDefinition> {
    match profile {
        ProfileKind::Common => vec![
            rule(
                "common.identity.readme_present",
                20,
                ProfilePriority::Critical,
                "Every profile needs a visible repository identity route.",
            ),
            rule(
                "common.docs.route_present",
                20,
                ProfilePriority::High,
                "Documentation routing is a common trust path.",
            ),
            rule(
                "common.quickstart.route_present",
                20,
                ProfilePriority::High,
                "A first-run route reduces evaluation friction.",
            ),
            rule(
                "common.license.file_present",
                20,
                ProfilePriority::High,
                "License visibility is a common reuse boundary.",
            ),
            rule(
                "common.support.route_present",
                5,
                ProfilePriority::Normal,
                "Support routing helps separate questions from defects.",
            ),
            rule(
                "common.contributing.route_present",
                5,
                ProfilePriority::Normal,
                "Contribution routing helps external changes stay reproducible.",
            ),
            rule(
                "common.security.route_present",
                5,
                ProfilePriority::Normal,
                "Security routing is a common trust signal.",
            ),
            rule(
                "common.release.route_present",
                3,
                ProfilePriority::Low,
                "Release routing helps users judge change risk.",
            ),
            rule(
                "common.automation.route_present",
                2,
                ProfilePriority::Low,
                "Automation signals make maintenance state easier to inspect.",
            ),
        ],
        ProfileKind::Library => vec![
            rule(
                "common.identity.readme_present",
                15,
                ProfilePriority::Critical,
                "A library needs immediate identity and scope.",
            ),
            rule(
                "common.docs.route_present",
                22,
                ProfilePriority::Critical,
                "Library users need API and usage documentation.",
            ),
            rule(
                "common.quickstart.route_present",
                18,
                ProfilePriority::High,
                "Library adoption depends on a small first example.",
            ),
            rule(
                "common.license.file_present",
                18,
                ProfilePriority::High,
                "Reuse requires a visible license boundary.",
            ),
            rule(
                "common.release.route_present",
                10,
                ProfilePriority::High,
                "Libraries need version and compatibility context.",
            ),
            rule(
                "common.security.route_present",
                7,
                ProfilePriority::Normal,
                "Security disclosure matters for downstream users.",
            ),
            rule(
                "common.contributing.route_present",
                6,
                ProfilePriority::Normal,
                "Libraries commonly benefit from contribution guidance.",
            ),
            rule(
                "common.support.route_present",
                3,
                ProfilePriority::Low,
                "Support routing helps users report integration issues.",
            ),
            rule(
                "common.automation.route_present",
                1,
                ProfilePriority::Low,
                "Automation is useful but secondary to user-facing routes.",
            ),
        ],
        ProfileKind::Cli => vec![
            rule(
                "common.identity.readme_present",
                15,
                ProfilePriority::Critical,
                "A CLI needs immediate command purpose.",
            ),
            rule(
                "common.quickstart.route_present",
                25,
                ProfilePriority::Critical,
                "A CLI should show install and first command quickly.",
            ),
            rule(
                "common.docs.route_present",
                18,
                ProfilePriority::High,
                "CLI flags and workflows need a documentation route.",
            ),
            rule(
                "common.license.file_present",
                14,
                ProfilePriority::High,
                "CLI reuse and packaging need license clarity.",
            ),
            rule(
                "common.release.route_present",
                10,
                ProfilePriority::High,
                "CLI users need update and compatibility notes.",
            ),
            rule(
                "common.support.route_present",
                8,
                ProfilePriority::Normal,
                "CLI users need a route for environment-specific failures.",
            ),
            rule(
                "common.automation.route_present",
                5,
                ProfilePriority::Normal,
                "A visible build or CI signal helps trust installability.",
            ),
            rule(
                "common.security.route_present",
                3,
                ProfilePriority::Low,
                "Security routing is still useful for shipped binaries.",
            ),
            rule(
                "common.contributing.route_present",
                2,
                ProfilePriority::Low,
                "Contribution routing is useful after user workflows are clear.",
            ),
        ],
        ProfileKind::Infra => vec![
            rule(
                "common.identity.readme_present",
                15,
                ProfilePriority::Critical,
                "Infrastructure repositories need clear blast-radius identity.",
            ),
            rule(
                "common.security.route_present",
                22,
                ProfilePriority::Critical,
                "Infrastructure changes need a visible security route.",
            ),
            rule(
                "common.automation.route_present",
                20,
                ProfilePriority::Critical,
                "Infrastructure trust depends heavily on automation signals.",
            ),
            rule(
                "common.docs.route_present",
                14,
                ProfilePriority::High,
                "Operational repositories need runbook or architecture documentation.",
            ),
            rule(
                "common.quickstart.route_present",
                10,
                ProfilePriority::High,
                "First-run guidance should show safe local or dry-run usage.",
            ),
            rule(
                "common.release.route_present",
                8,
                ProfilePriority::Normal,
                "Release routing helps operators evaluate rollout risk.",
            ),
            rule(
                "common.license.file_present",
                5,
                ProfilePriority::Normal,
                "License clarity still matters for reuse.",
            ),
            rule(
                "common.support.route_present",
                4,
                ProfilePriority::Normal,
                "Support routing helps operational incidents find the right channel.",
            ),
            rule(
                "common.contributing.route_present",
                2,
                ProfilePriority::Low,
                "Contribution guidance is secondary to safety and operations.",
            ),
        ],
        ProfileKind::Docs => vec![
            rule(
                "common.identity.readme_present",
                15,
                ProfilePriority::Critical,
                "A docs repository needs immediate subject identity.",
            ),
            rule(
                "common.docs.route_present",
                28,
                ProfilePriority::Critical,
                "Documentation routing is the primary product surface.",
            ),
            rule(
                "common.quickstart.route_present",
                18,
                ProfilePriority::High,
                "Readers need a first useful path into the material.",
            ),
            rule(
                "common.support.route_present",
                10,
                ProfilePriority::High,
                "Docs users need a route for questions and corrections.",
            ),
            rule(
                "common.contributing.route_present",
                10,
                ProfilePriority::Normal,
                "Docs quality often improves through contribution guidance.",
            ),
            rule(
                "common.license.file_present",
                8,
                ProfilePriority::Normal,
                "Documentation reuse needs license clarity.",
            ),
            rule(
                "common.release.route_present",
                5,
                ProfilePriority::Low,
                "Release routing matters when docs track product versions.",
            ),
            rule(
                "common.security.route_present",
                3,
                ProfilePriority::Low,
                "Security routing is less central unless docs cover sensitive systems.",
            ),
            rule(
                "common.automation.route_present",
                3,
                ProfilePriority::Low,
                "Automation is useful for docs builds but not the first route.",
            ),
        ],
        ProfileKind::Tutorial => vec![
            rule(
                "common.identity.readme_present",
                15,
                ProfilePriority::Critical,
                "A tutorial needs immediate topic identity.",
            ),
            rule(
                "common.quickstart.route_present",
                30,
                ProfilePriority::Critical,
                "Tutorial value depends on the first successful path.",
            ),
            rule(
                "common.docs.route_present",
                20,
                ProfilePriority::High,
                "Supporting documentation keeps the README from becoming a manual.",
            ),
            rule(
                "common.support.route_present",
                10,
                ProfilePriority::High,
                "Learners need a route for questions or corrections.",
            ),
            rule(
                "common.license.file_present",
                8,
                ProfilePriority::Normal,
                "Reuse of tutorial text or code needs license clarity.",
            ),
            rule(
                "common.contributing.route_present",
                7,
                ProfilePriority::Normal,
                "Tutorial corrections need contribution guidance when public.",
            ),
            rule(
                "common.release.route_present",
                5,
                ProfilePriority::Low,
                "Release routing helps when tutorial steps change over time.",
            ),
            rule(
                "common.automation.route_present",
                3,
                ProfilePriority::Low,
                "Automation is useful for checking examples but secondary.",
            ),
            rule(
                "common.security.route_present",
                2,
                ProfilePriority::Low,
                "Security routing is useful when tutorial content touches risky setup.",
            ),
        ],
        ProfileKind::Research => vec![
            rule(
                "common.identity.readme_present",
                18,
                ProfilePriority::Critical,
                "A research repository needs clear artifact identity.",
            ),
            rule(
                "common.docs.route_present",
                22,
                ProfilePriority::Critical,
                "Research users need method and reproduction documentation.",
            ),
            rule(
                "common.license.file_present",
                16,
                ProfilePriority::High,
                "Research reuse and citation need licensing clarity.",
            ),
            rule(
                "common.quickstart.route_present",
                14,
                ProfilePriority::High,
                "Reproduction starts with an executable first path.",
            ),
            rule(
                "common.release.route_present",
                10,
                ProfilePriority::Normal,
                "Release routing helps distinguish paper, artifact, and later changes.",
            ),
            rule(
                "common.contributing.route_present",
                8,
                ProfilePriority::Normal,
                "Contribution routing helps external replication fixes.",
            ),
            rule(
                "common.support.route_present",
                5,
                ProfilePriority::Normal,
                "Support routing gives researchers a contact path.",
            ),
            rule(
                "common.security.route_present",
                4,
                ProfilePriority::Low,
                "Security routing matters when data, models, or dependencies are involved.",
            ),
            rule(
                "common.automation.route_present",
                3,
                ProfilePriority::Low,
                "Automation is useful when reproducibility checks exist.",
            ),
        ],
        ProfileKind::Template => vec![
            rule(
                "common.identity.readme_present",
                18,
                ProfilePriority::Critical,
                "A template needs clear generated-project identity.",
            ),
            rule(
                "common.quickstart.route_present",
                22,
                ProfilePriority::Critical,
                "Template users need a first generation path.",
            ),
            rule(
                "common.docs.route_present",
                18,
                ProfilePriority::High,
                "Template options and customization need documentation routing.",
            ),
            rule(
                "common.license.file_present",
                14,
                ProfilePriority::High,
                "Template reuse needs clear license terms.",
            ),
            rule(
                "common.contributing.route_present",
                10,
                ProfilePriority::Normal,
                "Template maintenance often benefits from contribution guidance.",
            ),
            rule(
                "common.release.route_present",
                8,
                ProfilePriority::Normal,
                "Release routing helps users choose a template version.",
            ),
            rule(
                "common.support.route_present",
                5,
                ProfilePriority::Normal,
                "Support routing helps template users with generation issues.",
            ),
            rule(
                "common.automation.route_present",
                3,
                ProfilePriority::Low,
                "Automation is useful if generated output is validated.",
            ),
            rule(
                "common.security.route_present",
                2,
                ProfilePriority::Low,
                "Security routing is useful when templates create dependency surfaces.",
            ),
        ],
    }
}

fn rule(
    pattern_id: &'static str,
    weight: u32,
    priority: ProfilePriority,
    reason: &'static str,
) -> ProfileRuleDefinition {
    ProfileRuleDefinition {
        pattern_id,
        weight,
        priority,
        reason,
    }
}

fn to_profile_rule_result(
    index: usize,
    profile: ProfileKind,
    definition: &ProfileRuleDefinition,
    baseline_rule: Option<&BaselineRuleResult>,
) -> ProfileRuleResult {
    match baseline_rule {
        Some(rule) => ProfileRuleResult {
            rule_id: format!("profile-rule-{index:04}"),
            profile,
            pattern_id: definition.pattern_id.to_string(),
            title: rule.title.clone(),
            route: rule.route,
            status: rule.status,
            weight: definition.weight,
            priority: definition.priority,
            evidence_ids: rule.evidence_ids.clone(),
            finding_id: rule.finding_id.clone(),
            reason: definition.reason.to_string(),
        },
        None => ProfileRuleResult {
            rule_id: format!("profile-rule-{index:04}"),
            profile,
            pattern_id: definition.pattern_id.to_string(),
            title: "Unknown baseline pattern".to_string(),
            route: None,
            status: BaselineStatus::Missing,
            weight: definition.weight,
            priority: definition.priority,
            evidence_ids: Vec::new(),
            finding_id: None,
            reason: definition.reason.to_string(),
        },
    }
}

fn score_view(rules: &[ProfileRuleResult]) -> ProfileScoreView {
    let total_weight = rules.iter().map(|rule| rule.weight).sum::<u32>();
    let earned_weight = rules
        .iter()
        .filter(|rule| rule.status == BaselineStatus::Present)
        .map(|rule| rule.weight)
        .sum::<u32>();
    let score_x100 = earned_weight
        .saturating_mul(100)
        .checked_div(total_weight)
        .unwrap_or(0)
        .min(100) as u8;

    ProfileScoreView {
        earned_weight,
        total_weight,
        score_x100,
        present_rules: rules
            .iter()
            .filter(|rule| rule.status == BaselineStatus::Present)
            .count(),
        missing_rules: rules
            .iter()
            .filter(|rule| rule.status == BaselineStatus::Missing)
            .count(),
        note: "Score view is a deterministic priority view over observed baseline patterns, not a popularity, trust, security, or quality guarantee.".to_string(),
    }
}

fn ordered_recommendations(
    rules: &[ProfileRuleResult],
    findings_by_id: &BTreeMap<&str, &Finding>,
) -> Vec<ProfileRecommendation> {
    let mut recommendations = rules
        .iter()
        .filter(|rule| rule.status == BaselineStatus::Missing)
        .map(|rule| {
            let finding = rule
                .finding_id
                .as_deref()
                .and_then(|finding_id| findings_by_id.get(finding_id).copied());
            ProfileRecommendation {
                rank: 0,
                finding_id: rule.finding_id.clone(),
                pattern_id: rule.pattern_id.clone(),
                title: finding
                    .map(|finding| finding.title.clone())
                    .unwrap_or_else(|| rule.title.clone()),
                gate: finding
                    .and_then(|finding| finding.recommendation.as_ref())
                    .map(|recommendation| recommendation.gate)
                    .unwrap_or(GateKind::Guarded),
                severity: finding
                    .map(|finding| finding.severity)
                    .unwrap_or(Severity::Info),
                priority: rule.priority,
                weight: rule.weight,
                reason: rule.reason.clone(),
            }
        })
        .collect::<Vec<_>>();

    recommendations.sort_by(|left, right| {
        priority_rank(right.priority)
            .cmp(&priority_rank(left.priority))
            .then_with(|| right.weight.cmp(&left.weight))
            .then_with(|| severity_rank(right.severity).cmp(&severity_rank(left.severity)))
            .then_with(|| left.pattern_id.cmp(&right.pattern_id))
    });

    for (index, recommendation) in recommendations.iter_mut().enumerate() {
        recommendation.rank = index + 1;
    }

    recommendations
}

fn priority_rank(priority: ProfilePriority) -> u8 {
    match priority {
        ProfilePriority::Low => 1,
        ProfilePriority::Normal => 2,
        ProfilePriority::High => 3,
        ProfilePriority::Critical => 4,
    }
}

fn severity_rank(severity: Severity) -> u8 {
    match severity {
        Severity::Info => 1,
        Severity::Low => 2,
        Severity::Medium => 3,
        Severity::High => 4,
    }
}
