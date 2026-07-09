use seiri_core::{
    BaselineReport, BaselineRuleResult, BaselineStatus, FileKind, Finding, GateKind,
    ImportantFileKind, ProfileBranch, ProfileBranchSummary, ProfileKind, ProfilePriority,
    ProfileRecommendation, ProfileReport, ProfileRuleResult, ProfileScoreView, RepoSnapshot,
    RouteKind, Severity,
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
    let mut report = evaluate_profile_from_parts(baseline, &snapshot.findings, profile);
    let branches = profile_branches(Some(snapshot), baseline, profile);
    report.branch_summary = profile_branch_summary(profile, &branches);
    report.branches = branches;
    Some(report)
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

    let branches = profile_branches(None, baseline, profile);

    ProfileReport {
        profile,
        score,
        branch_summary: profile_branch_summary(profile, &branches),
        branches,
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
        ProfileKind::Product => vec![
            rule(
                "common.identity.readme_present",
                15,
                ProfilePriority::Critical,
                "A product repository needs immediate product identity and audience.",
            ),
            rule(
                "common.support.route_present",
                22,
                ProfilePriority::Critical,
                "Product users need a clear support and triage route.",
            ),
            rule(
                "common.docs.route_present",
                20,
                ProfilePriority::High,
                "Product repositories need user-facing documentation.",
            ),
            rule(
                "common.quickstart.route_present",
                14,
                ProfilePriority::High,
                "A first-run route helps users evaluate the product quickly.",
            ),
            rule(
                "common.release.route_present",
                12,
                ProfilePriority::High,
                "Release notes help product users understand change risk.",
            ),
            rule(
                "common.security.route_present",
                7,
                ProfilePriority::Normal,
                "Security disclosure matters when users run or depend on the product.",
            ),
            rule(
                "common.license.file_present",
                5,
                ProfilePriority::Normal,
                "License clarity still matters for distribution and reuse.",
            ),
            rule(
                "common.automation.route_present",
                3,
                ProfilePriority::Low,
                "Automation is useful but secondary to user-facing routes.",
            ),
            rule(
                "common.contributing.route_present",
                2,
                ProfilePriority::Low,
                "Contribution guidance is useful after support and docs routes are clear.",
            ),
        ],
        ProfileKind::Runtime => vec![
            rule(
                "common.identity.readme_present",
                15,
                ProfilePriority::Critical,
                "A runtime or compiler repository needs clear toolchain identity.",
            ),
            rule(
                "common.security.route_present",
                20,
                ProfilePriority::Critical,
                "Runtime and compiler users need a visible security route.",
            ),
            rule(
                "common.release.route_present",
                18,
                ProfilePriority::Critical,
                "Release trains and compatibility notes are central for runtimes.",
            ),
            rule(
                "common.docs.route_present",
                15,
                ProfilePriority::High,
                "Runtime users need build, language, and operational documentation.",
            ),
            rule(
                "common.automation.route_present",
                12,
                ProfilePriority::High,
                "A visible build or test signal matters for runtime reliability review.",
            ),
            rule(
                "common.contributing.route_present",
                8,
                ProfilePriority::Normal,
                "Runtime projects often need contribution and build guidance.",
            ),
            rule(
                "common.quickstart.route_present",
                6,
                ProfilePriority::Normal,
                "A quickstart helps users verify the toolchain locally.",
            ),
            rule(
                "common.license.file_present",
                4,
                ProfilePriority::Normal,
                "License clarity matters for redistribution.",
            ),
            rule(
                "common.support.route_present",
                2,
                ProfilePriority::Low,
                "Support routing is useful but secondary to release and security routes.",
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
        ProfileKind::Ml | ProfileKind::Research => vec![
            rule(
                "common.identity.readme_present",
                18,
                ProfilePriority::Critical,
                "An ML, data, or research repository needs clear artifact identity.",
            ),
            rule(
                "common.docs.route_present",
                22,
                ProfilePriority::Critical,
                "Users need method, data, and reproduction documentation.",
            ),
            rule(
                "common.license.file_present",
                16,
                ProfilePriority::High,
                "Research, model, and data reuse need licensing clarity.",
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

#[must_use]
pub fn branch_profiles() -> &'static [(ProfileKind, u16)] {
    &[
        (ProfileKind::Library, 280),
        (ProfileKind::Infra, 170),
        (ProfileKind::Cli, 110),
        (ProfileKind::Product, 90),
        (ProfileKind::Runtime, 45),
        (ProfileKind::Docs, 95),
        (ProfileKind::Tutorial, 100),
        (ProfileKind::Ml, 75),
        (ProfileKind::Template, 35),
    ]
}

fn profile_branches(
    snapshot: Option<&RepoSnapshot>,
    baseline: &BaselineReport,
    selected_profile: ProfileKind,
) -> Vec<ProfileBranch> {
    let baseline_by_pattern = baseline
        .rules
        .iter()
        .map(|rule| (rule.pattern_id.as_str(), rule))
        .collect::<BTreeMap<_, _>>();

    let mut branches = branch_profiles()
        .iter()
        .map(|(profile, prior_x1000)| {
            let rules = profile_rules(*profile)
                .iter()
                .enumerate()
                .map(|(index, definition)| {
                    to_profile_rule_result(
                        index + 1,
                        *profile,
                        definition,
                        baseline_by_pattern.get(definition.pattern_id).copied(),
                    )
                })
                .collect::<Vec<_>>();
            let score = score_view(&rules);
            let (evidence_score_x100, matched_signals, missing_signals) =
                profile_signal_score(snapshot, baseline, *profile);
            let confidence_x100 =
                confidence_score(*prior_x1000, evidence_score_x100, score.score_x100);
            let selected_note = if *profile == selected_profile {
                " Selected CLI/API profile; confidence remains evidence-weighted."
            } else {
                ""
            };

            ProfileBranch {
                rank: 0,
                profile: *profile,
                prior_x1000: *prior_x1000,
                confidence_x100,
                evidence_score_x100,
                score_x100: score.score_x100,
                matched_signals,
                missing_signals,
                rationale: format!(
                    "Combines Block J prior, observed route/file/path signals, and current profile score. This is a branch hint, not a repository type assertion.{selected_note}"
                ),
            }
        })
        .collect::<Vec<_>>();

    branches.sort_by(|left, right| {
        right
            .confidence_x100
            .cmp(&left.confidence_x100)
            .then_with(|| right.evidence_score_x100.cmp(&left.evidence_score_x100))
            .then_with(|| right.prior_x1000.cmp(&left.prior_x1000))
            .then_with(|| left.profile.cmp(&right.profile))
    });

    for (index, branch) in branches.iter_mut().enumerate() {
        branch.rank = index + 1;
    }

    branches
}

fn profile_branch_summary(
    selected_profile: ProfileKind,
    branches: &[ProfileBranch],
) -> ProfileBranchSummary {
    let top = branches.first();
    let second = branches.get(1);
    let ambiguous = match (top, second) {
        (Some(top), Some(second)) => {
            top.confidence_x100 < 60
                || top.confidence_x100.saturating_sub(second.confidence_x100) < 15
        }
        (Some(top), None) => top.confidence_x100 < 60,
        _ => true,
    };

    ProfileBranchSummary {
        selected_profile,
        top_profile: top.map(|branch| branch.profile),
        top_confidence_x100: top.map(|branch| branch.confidence_x100),
        emitted_profiles: branches.len(),
        ambiguous,
        boundary: "Profile branch confidence is a deterministic routing hint from observed evidence and fixed priors; it is not a repository type assertion, popularity claim, trust claim, security claim, or quality guarantee.".to_string(),
    }
}

fn profile_signal_score(
    snapshot: Option<&RepoSnapshot>,
    baseline: &BaselineReport,
    profile: ProfileKind,
) -> (u8, Vec<String>, Vec<String>) {
    let mut score = 0u32;
    let mut total = 0u32;
    let mut matched = Vec::new();
    let mut missing = Vec::new();

    macro_rules! signal {
        ($label:expr, $weight:expr, $condition:expr) => {{
            total += $weight;
            if $condition {
                score += $weight;
                matched.push($label.to_string());
            } else {
                missing.push($label.to_string());
            }
        }};
    }

    match profile {
        ProfileKind::Library => {
            signal!(
                "package manifest",
                12,
                has_important_file(snapshot, ImportantFileKind::CargoToml)
            );
            signal!(
                "docs route",
                16,
                has_route(snapshot, baseline, RouteKind::Docs)
            );
            signal!(
                "quickstart route",
                16,
                has_route(snapshot, baseline, RouteKind::Quickstart)
            );
            signal!(
                "release route",
                10,
                has_route(snapshot, baseline, RouteKind::Release)
            );
            signal!(
                "license boundary",
                12,
                has_route(snapshot, baseline, RouteKind::License)
            );
            signal!(
                "examples or API wording",
                8,
                path_or_readme_contains(snapshot, &["examples", "api", "sdk", "client"])
            );
        }
        ProfileKind::Cli => {
            signal!(
                "first command or quickstart",
                18,
                has_route(snapshot, baseline, RouteKind::Quickstart)
            );
            signal!(
                "release route",
                12,
                has_route(snapshot, baseline, RouteKind::Release)
            );
            signal!(
                "support route",
                10,
                has_route(snapshot, baseline, RouteKind::Support)
            );
            signal!(
                "automation signal",
                8,
                has_route(snapshot, baseline, RouteKind::Automation)
            );
            signal!(
                "binary or command path",
                12,
                path_or_readme_contains(
                    snapshot,
                    &["src/main.rs", "bin/", "cmd/", "cli", "command"]
                )
            );
        }
        ProfileKind::Infra => {
            signal!(
                "workflow automation",
                16,
                has_route(snapshot, baseline, RouteKind::Automation)
            );
            signal!(
                "security route",
                16,
                has_route(snapshot, baseline, RouteKind::Security)
            );
            signal!(
                "ops docs route",
                12,
                has_route(snapshot, baseline, RouteKind::Docs)
            );
            signal!(
                "ownership route",
                10,
                has_route(snapshot, baseline, RouteKind::Ownership)
            );
            signal!(
                "deployment or infra path",
                14,
                path_or_readme_contains(
                    snapshot,
                    &[
                        "helm",
                        "k8s",
                        "kubernetes",
                        "terraform",
                        "deploy",
                        "operator"
                    ]
                )
            );
        }
        ProfileKind::Product => {
            signal!(
                "support route",
                18,
                has_route(snapshot, baseline, RouteKind::Support)
            );
            signal!(
                "docs route",
                14,
                has_route(snapshot, baseline, RouteKind::Docs)
            );
            signal!(
                "release route",
                12,
                has_route(snapshot, baseline, RouteKind::Release)
            );
            signal!(
                "quickstart route",
                8,
                has_route(snapshot, baseline, RouteKind::Quickstart)
            );
            signal!(
                "app or product wording",
                12,
                path_or_readme_contains(snapshot, &["app", "web", "ui", "product", "screenshot"])
            );
        }
        ProfileKind::Runtime => {
            signal!(
                "security route",
                16,
                has_route(snapshot, baseline, RouteKind::Security)
            );
            signal!(
                "release route",
                16,
                has_route(snapshot, baseline, RouteKind::Release)
            );
            signal!(
                "governance route",
                12,
                has_route(snapshot, baseline, RouteKind::Governance)
            );
            signal!(
                "build automation",
                10,
                has_route(snapshot, baseline, RouteKind::Automation)
            );
            signal!(
                "runtime or compiler path",
                12,
                path_or_readme_contains(
                    snapshot,
                    &["runtime", "compiler", "toolchain", "fuzz", "tests"]
                )
            );
        }
        ProfileKind::Docs => {
            signal!(
                "docs route",
                20,
                has_route(snapshot, baseline, RouteKind::Docs)
            );
            signal!(
                "docs directory",
                14,
                has_important_file(snapshot, ImportantFileKind::DocsDirectory)
            );
            signal!(
                "contribution route",
                10,
                has_route(snapshot, baseline, RouteKind::Contributing)
            );
            signal!(
                "governance route",
                8,
                has_route(snapshot, baseline, RouteKind::Governance)
            );
            signal!(
                "spec or guide wording",
                10,
                path_or_readme_contains(snapshot, &["spec", "guide", "manual", "proposal"])
            );
        }
        ProfileKind::Tutorial => {
            signal!(
                "quickstart route",
                20,
                has_route(snapshot, baseline, RouteKind::Quickstart)
            );
            signal!(
                "docs route",
                12,
                has_route(snapshot, baseline, RouteKind::Docs)
            );
            signal!(
                "support route",
                10,
                has_route(snapshot, baseline, RouteKind::Support)
            );
            signal!(
                "examples or tutorial path",
                14,
                path_or_readme_contains(
                    snapshot,
                    &["examples", "tutorial", "lesson", "notebook", "sample"]
                )
            );
        }
        ProfileKind::Ml | ProfileKind::Research => {
            signal!(
                "docs route",
                14,
                has_route(snapshot, baseline, RouteKind::Docs)
            );
            signal!(
                "license boundary",
                12,
                has_route(snapshot, baseline, RouteKind::License)
            );
            signal!(
                "quickstart route",
                10,
                has_route(snapshot, baseline, RouteKind::Quickstart)
            );
            signal!(
                "model data or paper path",
                18,
                path_or_readme_contains(
                    snapshot,
                    &[
                        "model",
                        "dataset",
                        "data",
                        "notebook",
                        "paper",
                        "experiment"
                    ]
                )
            );
            signal!(
                "release or artifact route",
                8,
                has_route(snapshot, baseline, RouteKind::Release)
            );
        }
        ProfileKind::Template => {
            signal!(
                "quickstart route",
                16,
                has_route(snapshot, baseline, RouteKind::Quickstart)
            );
            signal!(
                "automation signal",
                12,
                has_route(snapshot, baseline, RouteKind::Automation)
            );
            signal!(
                "release route",
                10,
                has_route(snapshot, baseline, RouteKind::Release)
            );
            signal!(
                "template or action path",
                18,
                path_or_readme_contains(
                    snapshot,
                    &[
                        "template",
                        "action.yml",
                        "cookiecutter",
                        "scaffold",
                        "generator"
                    ]
                )
            );
        }
        ProfileKind::Common => {
            signal!(
                "identity route",
                16,
                has_route(snapshot, baseline, RouteKind::Identity)
            );
            signal!(
                "docs route",
                16,
                has_route(snapshot, baseline, RouteKind::Docs)
            );
            signal!(
                "license boundary",
                16,
                has_route(snapshot, baseline, RouteKind::License)
            );
        }
    }

    let evidence_score = if total == 0 {
        0
    } else {
        score.saturating_mul(100).checked_div(total).unwrap_or(0) as u8
    };
    (evidence_score.min(100), matched, missing)
}

fn confidence_score(prior_x1000: u16, evidence_score_x100: u8, score_x100: u8) -> u8 {
    let prior_component = u32::from(prior_x1000) / 20;
    let evidence_component = u32::from(evidence_score_x100) * 55 / 100;
    let score_component = u32::from(score_x100) * 30 / 100;
    prior_component
        .saturating_add(evidence_component)
        .saturating_add(score_component)
        .min(100) as u8
}

fn has_route(snapshot: Option<&RepoSnapshot>, baseline: &BaselineReport, route: RouteKind) -> bool {
    if baseline
        .rules
        .iter()
        .any(|rule| rule.route == Some(route) && rule.status == BaselineStatus::Present)
    {
        return true;
    }

    snapshot.is_some_and(|snapshot| {
        snapshot.route_states.iter().any(|state| {
            state.route == route
                && !matches!(
                    state.state,
                    seiri_core::RouteState::Absent | seiri_core::RouteState::UnsafeToInvent
                )
        }) || snapshot
            .evidence_ledger
            .iter()
            .any(|record| record.route == Some(route))
    })
}

fn has_important_file(snapshot: Option<&RepoSnapshot>, kind: ImportantFileKind) -> bool {
    snapshot.is_some_and(|snapshot| {
        snapshot
            .important_files
            .iter()
            .any(|important| important.kind == kind)
    })
}

fn path_or_readme_contains(snapshot: Option<&RepoSnapshot>, needles: &[&str]) -> bool {
    let Some(snapshot) = snapshot else {
        return false;
    };
    snapshot.files.iter().any(|record| {
        let path = record.path.to_ascii_lowercase();
        let kind_match = matches!(record.kind, FileKind::Directory | FileKind::File);
        kind_match && needles.iter().any(|needle| path.contains(needle))
    }) || snapshot.readme.as_ref().is_some_and(|readme| {
        readme
            .headings
            .iter()
            .any(|heading| contains_signal(&heading.text, needles))
            || readme.links.iter().any(|link| {
                contains_signal(&link.text, needles) || contains_signal(&link.target, needles)
            })
            || readme.route_candidates.iter().any(|candidate| {
                contains_signal(&candidate.text, needles)
                    || candidate
                        .target
                        .as_ref()
                        .is_some_and(|target| contains_signal(target, needles))
            })
    })
}

fn contains_signal(value: &str, needles: &[&str]) -> bool {
    let value = value.to_ascii_lowercase();
    needles.iter().any(|needle| value.contains(needle))
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
