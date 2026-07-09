use seiri_core::{
    stable_id, BaselineProfile, BaselineReport, BaselineRequirement, BaselineRuleResult,
    BaselineStatus, BaselineSummary, EvidenceKind, Finding, GateKind, ImportantFileKind,
    PatternMatch, PatternOutcome, Recommendation, RepoSnapshot, RouteKind, Severity,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternDefinition {
    pub id: &'static str,
    pub title: &'static str,
    pub route: Option<RouteKind>,
    pub detector: PatternDetector,
    pub requirement: BaselineRequirement,
    pub missing_severity: Severity,
    pub missing_gate: GateKind,
    pub missing_title: &'static str,
    pub missing_message: &'static str,
    pub recommendation_title: &'static str,
    pub recommendation_message: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternDetector {
    EvidenceKind(EvidenceKind),
    Route(RouteKind),
    ReadmeRoute(RouteKind),
    ImportantFile(ImportantFileKind),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternRegistry {
    definitions: Vec<PatternDefinition>,
}

impl PatternRegistry {
    #[must_use]
    pub fn new(definitions: Vec<PatternDefinition>) -> Self {
        Self { definitions }
    }

    #[must_use]
    pub fn definitions(&self) -> &[PatternDefinition] {
        &self.definitions
    }

    #[must_use]
    pub fn evaluate_patterns(&self, snapshot: &RepoSnapshot) -> Vec<PatternMatch> {
        self.definitions
            .iter()
            .enumerate()
            .map(|(index, definition)| {
                let evidence_ids = evidence_ids_for_detector(snapshot, definition.detector);
                PatternMatch {
                    id: stable_id("pattern-match", index + 1),
                    pattern_id: definition.id.to_string(),
                    title: definition.title.to_string(),
                    route: definition.route,
                    outcome: if evidence_ids.is_empty() {
                        PatternOutcome::Missing
                    } else {
                        PatternOutcome::Present
                    },
                    evidence_ids,
                    basis: detector_basis(definition.detector).to_string(),
                }
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaselineEvaluation {
    pub pattern_matches: Vec<PatternMatch>,
    pub report: BaselineReport,
    pub findings: Vec<Finding>,
}

#[must_use]
pub fn common_registry() -> PatternRegistry {
    PatternRegistry::new(vec![
        PatternDefinition {
            id: "common.identity.readme_present",
            title: "Root README is present",
            route: Some(RouteKind::Identity),
            detector: PatternDetector::EvidenceKind(EvidenceKind::ReadmePresent),
            requirement: BaselineRequirement::Required,
            missing_severity: Severity::Medium,
            missing_gate: GateKind::Manual,
            missing_title: "README is missing",
            missing_message: "RepoSeiri could not find a README at the repository root.",
            recommendation_title: "Create README routing hub",
            recommendation_message: "Create a README that identifies the repository and routes users to docs, support, contributing, security, license, and release information.",
        },
        PatternDefinition {
            id: "common.docs.route_present",
            title: "Documentation route is visible",
            route: Some(RouteKind::Docs),
            detector: PatternDetector::ReadmeRoute(RouteKind::Docs),
            requirement: BaselineRequirement::Required,
            missing_severity: Severity::Low,
            missing_gate: GateKind::Safe,
            missing_title: "Documentation route is missing",
            missing_message: "The common baseline did not detect a documentation route.",
            recommendation_title: "Expose docs route",
            recommendation_message: "Add or expose a README link to documentation when documentation exists, or keep the finding as a planning item until documentation is created.",
        },
        PatternDefinition {
            id: "common.quickstart.route_present",
            title: "Quickstart route is visible",
            route: Some(RouteKind::Quickstart),
            detector: PatternDetector::ReadmeRoute(RouteKind::Quickstart),
            requirement: BaselineRequirement::Required,
            missing_severity: Severity::Low,
            missing_gate: GateKind::Guarded,
            missing_title: "Quickstart route is missing",
            missing_message: "The common baseline did not detect install, usage, example, or quickstart language.",
            recommendation_title: "Add first-run guidance",
            recommendation_message: "Add quickstart guidance only after confirming the repository's intended user workflow.",
        },
        PatternDefinition {
            id: "common.license.file_present",
            title: "License file is present",
            route: Some(RouteKind::License),
            detector: PatternDetector::ImportantFile(ImportantFileKind::License),
            requirement: BaselineRequirement::Required,
            missing_severity: Severity::Low,
            missing_gate: GateKind::Manual,
            missing_title: "License file is missing",
            missing_message: "The common baseline did not detect a root license file.",
            recommendation_title: "Choose a license",
            recommendation_message: "License selection is a human decision; RepoSeiri should only route or report it before a maintainer chooses the license.",
        },
        PatternDefinition {
            id: "common.support.route_present",
            title: "Support route is visible",
            route: Some(RouteKind::Support),
            detector: PatternDetector::ReadmeRoute(RouteKind::Support),
            requirement: BaselineRequirement::Optional,
            missing_severity: Severity::Info,
            missing_gate: GateKind::Guarded,
            missing_title: "Support route is not visible",
            missing_message: "The common baseline did not detect a support or question route.",
            recommendation_title: "Expose support route",
            recommendation_message: "Add a support route only after confirming how this repository wants to handle questions and bug reports.",
        },
        PatternDefinition {
            id: "common.contributing.route_present",
            title: "Contributing route is visible",
            route: Some(RouteKind::Contributing),
            detector: PatternDetector::ReadmeRoute(RouteKind::Contributing),
            requirement: BaselineRequirement::Optional,
            missing_severity: Severity::Info,
            missing_gate: GateKind::Guarded,
            missing_title: "Contributing route is not visible",
            missing_message: "The common baseline did not detect a contribution route.",
            recommendation_title: "Expose contribution route",
            recommendation_message: "Add a contribution route when external contribution is expected; otherwise keep this as informational.",
        },
        PatternDefinition {
            id: "common.security.route_present",
            title: "Security route is visible",
            route: Some(RouteKind::Security),
            detector: PatternDetector::ReadmeRoute(RouteKind::Security),
            requirement: BaselineRequirement::Optional,
            missing_severity: Severity::Info,
            missing_gate: GateKind::Manual,
            missing_title: "Security route is not visible",
            missing_message: "The common baseline did not detect a security disclosure route.",
            recommendation_title: "Expose security policy route",
            recommendation_message: "Security policy content is a maintainer decision; RepoSeiri should not invent disclosure policy without confirmation.",
        },
        PatternDefinition {
            id: "common.release.route_present",
            title: "Release route is visible",
            route: Some(RouteKind::Release),
            detector: PatternDetector::ReadmeRoute(RouteKind::Release),
            requirement: BaselineRequirement::Optional,
            missing_severity: Severity::Info,
            missing_gate: GateKind::Guarded,
            missing_title: "Release route is not visible",
            missing_message: "The common baseline did not detect changelog, release, version, or compatibility routing.",
            recommendation_title: "Expose release route",
            recommendation_message: "Add release or changelog routing when users need update-risk context.",
        },
        PatternDefinition {
            id: "common.automation.route_present",
            title: "Automation signal is visible",
            route: Some(RouteKind::Automation),
            detector: PatternDetector::Route(RouteKind::Automation),
            requirement: BaselineRequirement::Optional,
            missing_severity: Severity::Info,
            missing_gate: GateKind::Guarded,
            missing_title: "Automation signal is not visible",
            missing_message: "The common baseline did not detect CI, workflow, badge, or automation evidence.",
            recommendation_title: "Expose automation signal",
            recommendation_message: "Add automation evidence only when the repository has a real workflow or test signal to expose.",
        },
    ])
}

#[must_use]
pub fn evaluate_common_baseline(snapshot: &RepoSnapshot) -> BaselineEvaluation {
    let registry = common_registry();
    evaluate_with_registry(snapshot, &registry)
}

#[must_use]
pub fn evaluate_with_registry(
    snapshot: &RepoSnapshot,
    registry: &PatternRegistry,
) -> BaselineEvaluation {
    let pattern_matches = registry.evaluate_patterns(snapshot);
    let mut findings = Vec::new();
    let mut rules = Vec::new();
    let mut summary = BaselineSummary {
        required_present: 0,
        required_missing: 0,
        optional_present: 0,
        optional_missing: 0,
    };

    for (index, definition) in registry.definitions().iter().enumerate() {
        let pattern_match = &pattern_matches[index];
        let status = match pattern_match.outcome {
            PatternOutcome::Present => BaselineStatus::Present,
            PatternOutcome::Missing => BaselineStatus::Missing,
        };
        increment_summary(&mut summary, definition.requirement, status);

        let finding_id = if status == BaselineStatus::Missing {
            let id = stable_id("finding", findings.len() + 1);
            findings.push(Finding {
                id: id.clone(),
                severity: definition.missing_severity,
                title: definition.missing_title.to_string(),
                message: definition.missing_message.to_string(),
                evidence_ids: pattern_match.evidence_ids.clone(),
                recommendation: Some(Recommendation {
                    id: stable_id("rec", findings.len() + 1),
                    gate: definition.missing_gate,
                    title: definition.recommendation_title.to_string(),
                    message: definition.recommendation_message.to_string(),
                }),
            });
            Some(id)
        } else {
            None
        };

        rules.push(BaselineRuleResult {
            rule_id: stable_id("baseline-rule", index + 1),
            pattern_id: definition.id.to_string(),
            title: definition.title.to_string(),
            route: definition.route,
            requirement: definition.requirement,
            status,
            severity: definition.missing_severity,
            evidence_ids: pattern_match.evidence_ids.clone(),
            finding_id,
            message: baseline_message(definition, status).to_string(),
        });
    }

    BaselineEvaluation {
        pattern_matches,
        report: BaselineReport {
            profile: BaselineProfile::Common,
            summary,
            rules,
        },
        findings,
    }
}

fn evidence_ids_for_detector(snapshot: &RepoSnapshot, detector: PatternDetector) -> Vec<String> {
    match detector {
        PatternDetector::EvidenceKind(kind) => snapshot
            .evidence
            .iter()
            .filter(|evidence| evidence.kind == kind)
            .map(|evidence| evidence.id.clone())
            .collect(),
        PatternDetector::Route(route) => snapshot
            .evidence
            .iter()
            .filter(|evidence| evidence.route == Some(route))
            .map(|evidence| evidence.id.clone())
            .collect(),
        PatternDetector::ReadmeRoute(route) => snapshot
            .evidence
            .iter()
            .filter(|evidence| {
                evidence.route == Some(route)
                    && matches!(
                        evidence.kind,
                        EvidenceKind::MarkdownHeading
                            | EvidenceKind::MarkdownLink
                            | EvidenceKind::RouteCandidate
                    )
            })
            .map(|evidence| evidence.id.clone())
            .collect(),
        PatternDetector::ImportantFile(kind) => {
            let expected = format!("{kind:?}");
            snapshot
                .evidence
                .iter()
                .filter(|evidence| {
                    evidence.kind == EvidenceKind::ImportantFile && evidence.value == expected
                })
                .map(|evidence| evidence.id.clone())
                .collect()
        }
    }
}

fn detector_basis(detector: PatternDetector) -> &'static str {
    match detector {
        PatternDetector::EvidenceKind(_) => "evidence kind",
        PatternDetector::Route(_) => "trust route",
        PatternDetector::ReadmeRoute(_) => "README trust route",
        PatternDetector::ImportantFile(_) => "important file",
    }
}

fn increment_summary(
    summary: &mut BaselineSummary,
    requirement: BaselineRequirement,
    status: BaselineStatus,
) {
    match (requirement, status) {
        (BaselineRequirement::Required, BaselineStatus::Present) => summary.required_present += 1,
        (BaselineRequirement::Required, BaselineStatus::Missing) => summary.required_missing += 1,
        (BaselineRequirement::Optional, BaselineStatus::Present) => summary.optional_present += 1,
        (BaselineRequirement::Optional, BaselineStatus::Missing) => summary.optional_missing += 1,
    }
}

fn baseline_message(definition: &PatternDefinition, status: BaselineStatus) -> &'static str {
    match status {
        BaselineStatus::Present => "Pattern observed in common baseline evidence.",
        BaselineStatus::Missing => definition.missing_message,
    }
}
