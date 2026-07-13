#![forbid(unsafe_code)]

mod registry;
mod weight;

pub use registry::{ProfileDefinition, ProfileRegistry, ProfileRegistryError};
pub use weight::StaticProfileWeight;

use seiri_core::{
    facet_evidence_ids, BaselineReport, BaselineRuleResult, BaselineStatus, CalibrationKey,
    CalibrationLookup, CalibrationPriorState, CalibrationProvider, FacetAssessment, FacetReport,
    FileKind, Finding, GateKind, ImportantFileKind, NoCalibrationProvider, Observation,
    ProfileBranch, ProfileBranchSemantics, ProfileBranchSummary, ProfileEvidenceBasis, ProfileFit,
    ProfileKind, ProfilePriority, ProfilePurposeAffinity, ProfileRankScore, ProfileRecommendation,
    ProfileReport, ProfileRuleResult, ProfileScoreView, ProfileWeightBasis, RepositoryAnalysis,
    RepositoryFacet, RouteKind, Severity,
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileRuleDefinition {
    pub pattern_id: &'static str,
    pub weight: StaticProfileWeight,
    pub priority: ProfilePriority,
    pub reason: &'static str,
}

#[must_use]
pub fn evaluate_profile(
    snapshot: &RepositoryAnalysis,
    profile: ProfileKind,
) -> Option<ProfileReport> {
    evaluate_profile_with_calibration(snapshot, profile, &NoCalibrationProvider)
}

#[must_use]
pub fn evaluate_profile_with_calibration(
    snapshot: &RepositoryAnalysis,
    profile: ProfileKind,
    calibration: &dyn CalibrationProvider,
) -> Option<ProfileReport> {
    let baseline = snapshot.baseline.as_ref()?;
    let registry = common_profile_registry();
    let mut report =
        evaluate_profile_from_registry(baseline, &snapshot.findings, profile, &registry);
    let branches = profile_branches(Some(snapshot), baseline, profile, &registry, calibration);
    report.branch_summary = profile_branch_summary(profile, &branches);
    report.branches = branches;
    Some(report)
}

/// Evaluates coexisting repository facets without selecting a repository type.
#[must_use]
pub fn evaluate_facets(snapshot: &RepositoryAnalysis) -> FacetReport {
    let facets = RepositoryFacet::ALL
        .into_iter()
        .map(|facet| {
            let evidence = facet_evidence_ids(facet, &snapshot.evidence_kernel);
            let observation = if evidence.is_empty() {
                snapshot
                    .coverage
                    .observe_absence(seiri_core::CoverageScope::RepositoryFiles)
            } else {
                Observation::present((), evidence)
                    .expect("facet evidence ids are non-empty after collection")
            };
            FacetAssessment { facet, observation }
        })
        .collect();
    FacetReport::try_new(facets).expect("complete facet evaluation is canonical")
}

#[must_use]
pub fn evaluate_profile_from_parts(
    baseline: &BaselineReport,
    findings: &[Finding],
    profile: ProfileKind,
) -> ProfileReport {
    let registry = common_profile_registry();
    evaluate_profile_from_registry(baseline, findings, profile, &registry)
}

fn evaluate_profile_from_registry(
    baseline: &BaselineReport,
    findings: &[Finding],
    profile: ProfileKind,
    registry: &ProfileRegistry,
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

    let profile_definition = registry
        .definition(profile)
        .expect("complete profile registry must contain every profile");
    let mut rules = Vec::new();
    let mut scoring_inputs = Vec::new();
    for (index, definition) in profile_definition.rules.iter().enumerate() {
        let baseline_rule = baseline_by_pattern.get(definition.pattern_id).copied();
        let result = to_profile_rule_result(index + 1, profile, definition, baseline_rule);
        scoring_inputs.push(ProfileScoringInput {
            status: result.status,
            weight: definition.weight,
        });
        rules.push(result);
    }

    let score = score_view(&scoring_inputs);
    let recommendations = ordered_recommendations(&rules, &findings_by_id);

    let branches = profile_branches(None, baseline, profile, registry, &NoCalibrationProvider);

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
    common_profile_registry()
        .definition(profile)
        .expect("complete profile registry must contain every profile")
        .rules
        .clone()
}

#[must_use]
pub fn common_profile_registry() -> ProfileRegistry {
    let definitions = ProfileKind::ALL
        .into_iter()
        .map(|profile| ProfileDefinition {
            profile,
            rules: catalog_rules(profile),
        })
        .collect();
    ProfileRegistry::try_complete(definitions)
        .expect("built-in profile registry must satisfy completeness invariants")
}

fn catalog_rules(profile: ProfileKind) -> Vec<ProfileRuleDefinition> {
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
        (ProfileKind::Library, 0),
        (ProfileKind::Infra, 0),
        (ProfileKind::Cli, 0),
        (ProfileKind::Product, 0),
        (ProfileKind::Runtime, 0),
        (ProfileKind::Docs, 0),
        (ProfileKind::Tutorial, 0),
        (ProfileKind::Ml, 0),
        (ProfileKind::Research, 0),
        (ProfileKind::Template, 0),
    ]
}

fn profile_branches(
    snapshot: Option<&RepositoryAnalysis>,
    baseline: &BaselineReport,
    selected_profile: ProfileKind,
    registry: &ProfileRegistry,
    calibration: &dyn CalibrationProvider,
) -> Vec<ProfileBranch> {
    let baseline_by_pattern = baseline
        .rules
        .iter()
        .map(|rule| (rule.pattern_id.as_str(), rule))
        .collect::<BTreeMap<_, _>>();

    let mut branches = branch_profiles()
        .iter()
        .map(|(profile, _static_order)| {
            let definition = registry
                .definition(*profile)
                .expect("complete profile registry must contain branch profiles");
            let scoring_inputs = definition
                .rules
                .iter()
                .map(|definition| ProfileScoringInput {
                    status: baseline_by_pattern
                        .get(definition.pattern_id)
                        .map_or(BaselineStatus::Missing, |rule| {
                            evidence_backed_status(rule)
                        }),
                    weight: definition.weight,
                })
                .collect::<Vec<_>>();
            let score = score_view(&scoring_inputs);
            let (evidence_score_x100, matched_signals, missing_signals) =
                profile_signal_score(snapshot, baseline, *profile);
            let (prior_weight_x100, calibration_prior) = match calibration
                .prior(&CalibrationKey::ProfileBranch(*profile))
            {
                CalibrationLookup::NotRequested => (0, CalibrationPriorState::NotRequested),
                CalibrationLookup::Available(prior) => (
                    prior.rank_weight_x100(),
                    CalibrationPriorState::AppliedRedacted,
                ),
                CalibrationLookup::Unavailable(_) => (0, CalibrationPriorState::Unavailable),
            };
            let rank_score_x100 =
                profile_rank_score(prior_weight_x100, evidence_score_x100, score.score_x100);
            let selected_note = if *profile == selected_profile {
                " Selected CLI/API profile; confidence remains evidence-weighted."
            } else {
                ""
            };

            ProfileBranch {
                rank: 0,
                profile: *profile,
                semantics: ProfileBranchSemantics {
                    fit: ProfileFit::from_bounded(score.score_x100),
                    purpose_affinity: ProfilePurposeAffinity::from_bounded(evidence_score_x100),
                    rank_score: ProfileRankScore::from_bounded(rank_score_x100),
                    calibration_prior,
                },
                matched_signals,
                missing_signals,
                rationale: format!(
                    "Combines observed route/file/path evidence and profile fit. An explicit local calibration prior may affect rank but its value remains redacted from public fields. This is a branch hint, not a repository type assertion.{selected_note}"
                ),
            }
        })
        .collect::<Vec<_>>();

    branches.sort_by(|left, right| {
        right
            .semantics
            .rank_score
            .get()
            .cmp(&left.semantics.rank_score.get())
            .then_with(|| {
                right
                    .semantics
                    .purpose_affinity
                    .get()
                    .cmp(&left.semantics.purpose_affinity.get())
            })
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
            top.semantics.rank_score.get() < 60
                || top
                    .semantics
                    .rank_score
                    .get()
                    .saturating_sub(second.semantics.rank_score.get())
                    < 15
        }
        (Some(top), None) => top.semantics.rank_score.get() < 60,
        _ => true,
    };

    ProfileBranchSummary {
        selected_profile,
        top_profile: top.map(|branch| branch.profile),
        top_rank_score_x100: top.map(|branch| branch.semantics.rank_score.get()),
        emitted_profiles: branches.len(),
        ambiguous,
        boundary: "Profile fit, typed purpose affinity, rank score, and calibration-prior state are separate values. Purpose affinity excludes fixture, test, generated, and supporting-example paths. Rank score is not a probability or a repository type, popularity, trust, security, or quality assertion.".to_string(),
    }
}

fn profile_signal_score(
    snapshot: Option<&RepositoryAnalysis>,
    baseline: &BaselineReport,
    profile: ProfileKind,
) -> (u8, Vec<String>, Vec<String>) {
    let mut score = 0u32;
    let mut total = 0u32;
    let mut matched = Vec::new();
    let mut missing = Vec::new();

    macro_rules! signal {
        ($label:expr, $weight:expr, $condition:expr) => {{
            if is_purpose_signal($label) {
                total += $weight;
                if $condition {
                    score += $weight;
                    matched.push($label.to_string());
                } else {
                    missing.push($label.to_string());
                }
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
        ProfileKind::Ml => {
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
                path_or_readme_contains(snapshot, &["model", "dataset", "data", "notebook"])
            );
            signal!(
                "release or artifact route",
                8,
                has_route(snapshot, baseline, RouteKind::Release)
            );
        }
        ProfileKind::Research => {
            signal!(
                "research artifact path",
                18,
                path_or_readme_contains(
                    snapshot,
                    &["paper", "papers", "experiment", "experiments", "research"]
                )
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

fn profile_rank_score(prior_weight_x100: u8, evidence_score_x100: u8, score_x100: u8) -> u8 {
    let prior_component = u32::from(prior_weight_x100) * 15 / 100;
    let evidence_component = u32::from(evidence_score_x100) * 55 / 100;
    let score_component = u32::from(score_x100) * 30 / 100;
    prior_component
        .saturating_add(evidence_component)
        .saturating_add(score_component)
        .min(100) as u8
}

fn is_purpose_signal(label: &str) -> bool {
    matches!(
        label,
        "package manifest"
            | "examples or API wording"
            | "binary or command path"
            | "deployment or infra path"
            | "app or product wording"
            | "runtime or compiler path"
            | "docs directory"
            | "spec or guide wording"
            | "examples or tutorial path"
            | "model data or paper path"
            | "research artifact path"
            | "template or action path"
    )
}

fn has_route(
    snapshot: Option<&RepositoryAnalysis>,
    baseline: &BaselineReport,
    route: RouteKind,
) -> bool {
    if baseline
        .rules
        .iter()
        .any(|rule| rule.route == Some(route) && rule.status == BaselineStatus::Present)
    {
        return true;
    }

    snapshot.is_some_and(|snapshot| {
        snapshot.route_assessments.iter().any(|assessment| {
            assessment.route() == route
                && (assessment.presence().root_structured()
                    || assessment.presence().inherited()
                    || assessment.readme().routing().is_present())
        }) || snapshot
            .evidence_kernel
            .facts()
            .iter()
            .any(|fact| fact.atom.route() == Some(route))
    })
}

fn has_important_file(snapshot: Option<&RepositoryAnalysis>, kind: ImportantFileKind) -> bool {
    snapshot.is_some_and(|snapshot| {
        snapshot
            .important_files
            .iter()
            .any(|important| important.kind == kind)
    })
}

fn path_or_readme_contains(snapshot: Option<&RepositoryAnalysis>, needles: &[&str]) -> bool {
    let Some(snapshot) = snapshot else {
        return false;
    };
    snapshot.files.iter().any(|record| {
        let path = record.path.replace('\\', "/").to_ascii_lowercase();
        let kind_match = matches!(record.kind, FileKind::Directory | FileKind::File);
        kind_match
            && is_primary_artifact_path(&path)
            && needles
                .iter()
                .any(|needle| typed_path_signal(&path, needle))
    }) || snapshot.readme_summary.as_ref().is_some_and(|readme| {
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
    let words = value
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>();
    needles.iter().any(|needle| {
        let needle = needle.trim_matches('/').to_ascii_lowercase();
        words.iter().any(|word| word == &needle)
    })
}

fn is_primary_artifact_path(path: &str) -> bool {
    !matches!(
        path.split('/').next().unwrap_or_default(),
        "fixtures"
            | "fixture"
            | "tests"
            | "test"
            | "examples"
            | "example"
            | "samples"
            | "sample"
            | "target"
            | "generated"
            | "dist"
            | "build"
            | "vendor"
            | "node_modules"
    )
}

fn typed_path_signal(path: &str, needle: &str) -> bool {
    let needle = needle.trim_matches('/').to_ascii_lowercase();
    if needle.contains('/') {
        return path == needle || path.ends_with(&format!("/{needle}"));
    }
    path.split('/').any(|segment| {
        segment == needle || segment.split('.').next().is_some_and(|stem| stem == needle)
    })
}

fn rule(
    pattern_id: &'static str,
    weight: u16,
    priority: ProfilePriority,
    reason: &'static str,
) -> ProfileRuleDefinition {
    ProfileRuleDefinition {
        pattern_id,
        weight: StaticProfileWeight::from_registry_value(weight)
            .expect("built-in profile weights must be non-zero"),
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
            status: evidence_backed_status(rule),
            weight: definition.weight.get(),
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
            weight: definition.weight.get(),
            priority: definition.priority,
            evidence_ids: Vec::new(),
            finding_id: None,
            reason: definition.reason.to_string(),
        },
    }
}

fn evidence_backed_status(rule: &BaselineRuleResult) -> BaselineStatus {
    if rule.status == BaselineStatus::Present && !rule.evidence_ids.is_empty() {
        BaselineStatus::Present
    } else {
        BaselineStatus::Missing
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ProfileScoringInput {
    status: BaselineStatus,
    weight: StaticProfileWeight,
}

fn score_view(inputs: &[ProfileScoringInput]) -> ProfileScoreView {
    let total_weight = inputs.iter().map(|input| input.weight.get()).sum::<u32>();
    let earned_weight = inputs
        .iter()
        .filter(|input| input.status == BaselineStatus::Present)
        .map(|input| input.weight.get())
        .sum::<u32>();
    let score_x100 = earned_weight
        .saturating_mul(100)
        .checked_div(total_weight)
        .unwrap_or(0)
        .min(100) as u8;

    ProfileScoreView {
        evidence_basis: ProfileEvidenceBasis::RepositoryEvidence,
        weight_basis: ProfileWeightBasis::StaticProfileRegistry,
        earned_weight,
        total_weight,
        score_x100,
        present_rules: inputs
            .iter()
            .filter(|input| input.status == BaselineStatus::Present)
            .count(),
        missing_rules: inputs
            .iter()
            .filter(|input| input.status == BaselineStatus::Missing)
            .count(),
        note: "Score view uses repository evidence and static profile-registry weights only. Calibration estimates remain review-only suggestions until separately adopted; this is not a popularity, trust, security, or quality guarantee.".to_string(),
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
