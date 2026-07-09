use seiri_core::{
    stable_id, BaselineProfile, BaselineReport, BaselineRequirement, BaselineRuleResult,
    BaselineStatus, BaselineSummary, EvidenceKind, EvidenceScope, Finding, GateKind,
    ImportantFileKind, PatternGroup, PatternMatch, PatternOutcome, Recommendation, RepoSnapshot,
    RouteKind, Severity, SCHEMA_VERSION,
};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternDefinition {
    pub id: &'static str,
    pub group: PatternGroup,
    pub title: &'static str,
    pub route: Option<RouteKind>,
    pub detector: PatternDetector,
    pub requirement: BaselineRequirement,
    pub adoption_stage: PatternAdoptionStage,
    pub missing_severity: Severity,
    pub missing_gate: GateKind,
    pub missing_title: &'static str,
    pub missing_message: &'static str,
    pub recommendation_title: &'static str,
    pub recommendation_message: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternAdoptionStage {
    CommonBaseline,
    Candidate,
}

impl PatternAdoptionStage {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CommonBaseline => "common_baseline",
            Self::Candidate => "candidate",
        }
    }

    #[must_use]
    pub fn active_in_common_baseline(self) -> bool {
        matches!(self, Self::CommonBaseline)
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PatternGroupDefinition {
    pub group: PatternGroup,
    pub title: &'static str,
    pub description: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PatternRegistryDocument {
    pub schema_version: String,
    pub registry_version: &'static str,
    pub groups: Vec<PatternGroupDocument>,
    pub patterns: Vec<PatternDocument>,
    pub claim_boundary: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PatternGroupDocument {
    pub code: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub pattern_count: usize,
    pub baseline_count: usize,
    pub candidate_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PatternDocument {
    pub id: &'static str,
    pub group: PatternGroup,
    pub title: &'static str,
    pub route: Option<RouteKind>,
    pub detector_kind: &'static str,
    pub detector: String,
    pub requirement: BaselineRequirement,
    pub adoption_stage: &'static str,
    pub active_in_common_baseline: bool,
    pub missing_gate: GateKind,
    pub missing_severity: Severity,
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
    pub fn evaluation_definitions(&self) -> Vec<&PatternDefinition> {
        self.definitions
            .iter()
            .filter(|definition| definition.adoption_stage.active_in_common_baseline())
            .collect()
    }

    #[must_use]
    pub fn evaluate_patterns(&self, snapshot: &RepoSnapshot) -> Vec<PatternMatch> {
        self.evaluation_definitions()
            .into_iter()
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

#[must_use]
pub fn evidence_ids_for_definition(
    snapshot: &RepoSnapshot,
    definition: &PatternDefinition,
) -> Vec<String> {
    evidence_ids_for_detector(snapshot, definition.detector)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaselineEvaluation {
    pub pattern_matches: Vec<PatternMatch>,
    pub report: BaselineReport,
    pub findings: Vec<Finding>,
}

const PATTERN_GROUPS: [PatternGroupDefinition; 13] = [
    PatternGroupDefinition {
        group: PatternGroup::Idn,
        title: "Identity",
        description: "Repository identity, purpose, and root README presence.",
    },
    PatternGroupDefinition {
        group: PatternGroup::Doc,
        title: "Documentation",
        description: "Documentation routes and docs surface discoverability.",
    },
    PatternGroupDefinition {
        group: PatternGroup::Qst,
        title: "Quickstart",
        description: "First-run, install, example, and usage entry routes.",
    },
    PatternGroupDefinition {
        group: PatternGroup::Sup,
        title: "Support",
        description: "Question, help, and support routing.",
    },
    PatternGroupDefinition {
        group: PatternGroup::Sec,
        title: "Security",
        description: "Security policy, disclosure, and safety routing.",
    },
    PatternGroupDefinition {
        group: PatternGroup::Ctr,
        title: "Contribution",
        description: "Contribution and change-submission routes.",
    },
    PatternGroupDefinition {
        group: PatternGroup::Int,
        title: "Intake",
        description: "Issue, bug, feature, and discussion intake surfaces.",
    },
    PatternGroupDefinition {
        group: PatternGroup::Aut,
        title: "Automation",
        description: "CI, workflow, badge, and repeatability signals.",
    },
    PatternGroupDefinition {
        group: PatternGroup::Rel,
        title: "Release",
        description: "Changelog, release, version, and compatibility routes.",
    },
    PatternGroupDefinition {
        group: PatternGroup::Own,
        title: "Ownership",
        description: "Ownership, maintainer, and code-owner surfaces.",
    },
    PatternGroupDefinition {
        group: PatternGroup::Gov,
        title: "Governance",
        description: "Governance, decision, and project stewardship routes.",
    },
    PatternGroupDefinition {
        group: PatternGroup::Hyg,
        title: "Hygiene",
        description: "Repository hygiene, cleanup, and maintenance posture routes.",
    },
    PatternGroupDefinition {
        group: PatternGroup::Lif,
        title: "Lifecycle",
        description:
            "License, maintenance, deprecation, and supported-version boundaries needed for reuse.",
    },
];

#[must_use]
pub fn pattern_groups() -> &'static [PatternGroupDefinition] {
    &PATTERN_GROUPS
}

#[must_use]
pub fn common_registry() -> PatternRegistry {
    PatternRegistry::new(vec![
        baseline_pattern(
            PatternGroup::Idn,
            "common.identity.readme_present",
            "Root README is present",
            Some(RouteKind::Identity),
            PatternDetector::EvidenceKind(EvidenceKind::ReadmePresent),
            BaselineRequirement::Required,
            Severity::Medium,
            GateKind::Manual,
            "README is missing",
            "RepoSeiri could not find a README at the repository root.",
            "Create README routing hub",
            "Create a README that identifies the repository and routes users to docs, support, contributing, security, license, and release information.",
        ),
        baseline_pattern(
            PatternGroup::Doc,
            "common.docs.route_present",
            "Documentation route is visible",
            Some(RouteKind::Docs),
            PatternDetector::ReadmeRoute(RouteKind::Docs),
            BaselineRequirement::Required,
            Severity::Low,
            GateKind::Safe,
            "Documentation route is missing",
            "The common baseline did not detect a documentation route.",
            "Expose docs route",
            "Add or expose a README link to documentation when documentation exists, or keep the finding as a planning item until documentation is created.",
        ),
        baseline_pattern(
            PatternGroup::Qst,
            "common.quickstart.route_present",
            "Quickstart route is visible",
            Some(RouteKind::Quickstart),
            PatternDetector::ReadmeRoute(RouteKind::Quickstart),
            BaselineRequirement::Required,
            Severity::Low,
            GateKind::Guarded,
            "Quickstart route is missing",
            "The common baseline did not detect install, usage, example, or quickstart language.",
            "Add first-run guidance",
            "Add quickstart guidance only after confirming the repository's intended user workflow.",
        ),
        baseline_pattern(
            PatternGroup::Lif,
            "common.license.file_present",
            "License file is present",
            Some(RouteKind::License),
            PatternDetector::ImportantFile(ImportantFileKind::License),
            BaselineRequirement::Required,
            Severity::Low,
            GateKind::Manual,
            "License file is missing",
            "The common baseline did not detect a root license file.",
            "Choose a license",
            "License selection is a human decision; RepoSeiri should only route or report it before a maintainer chooses the license.",
        ),
        baseline_pattern(
            PatternGroup::Sup,
            "common.support.route_present",
            "Support route is visible",
            Some(RouteKind::Support),
            PatternDetector::ReadmeRoute(RouteKind::Support),
            BaselineRequirement::Optional,
            Severity::Info,
            GateKind::Guarded,
            "Support route is not visible",
            "The common baseline did not detect a support or question route.",
            "Expose support route",
            "Add a support route only after confirming how this repository wants to handle questions and bug reports.",
        ),
        baseline_pattern(
            PatternGroup::Ctr,
            "common.contributing.route_present",
            "Contributing route is visible",
            Some(RouteKind::Contributing),
            PatternDetector::ReadmeRoute(RouteKind::Contributing),
            BaselineRequirement::Optional,
            Severity::Info,
            GateKind::Guarded,
            "Contributing route is not visible",
            "The common baseline did not detect a contribution route.",
            "Expose contribution route",
            "Add a contribution route when external contribution is expected; otherwise keep this as informational.",
        ),
        baseline_pattern(
            PatternGroup::Sec,
            "common.security.route_present",
            "Security route is visible",
            Some(RouteKind::Security),
            PatternDetector::ReadmeRoute(RouteKind::Security),
            BaselineRequirement::Optional,
            Severity::Info,
            GateKind::Manual,
            "Security route is not visible",
            "The common baseline did not detect a security disclosure route.",
            "Expose security policy route",
            "Security policy content is a maintainer decision; RepoSeiri should not invent disclosure policy without confirmation.",
        ),
        baseline_pattern(
            PatternGroup::Rel,
            "common.release.route_present",
            "Release route is visible",
            Some(RouteKind::Release),
            PatternDetector::ReadmeRoute(RouteKind::Release),
            BaselineRequirement::Optional,
            Severity::Info,
            GateKind::Guarded,
            "Release route is not visible",
            "The common baseline did not detect changelog, release, version, or compatibility routing.",
            "Expose release route",
            "Add release or changelog routing when users need update-risk context.",
        ),
        baseline_pattern(
            PatternGroup::Lif,
            "common.lifecycle.route_present",
            "Lifecycle route is visible",
            Some(RouteKind::Lifecycle),
            PatternDetector::ReadmeRoute(RouteKind::Lifecycle),
            BaselineRequirement::Optional,
            Severity::Info,
            GateKind::Manual,
            "Lifecycle route is not visible",
            "The common baseline did not detect maintenance, deprecation, supported-version, archival, or end-of-life routing.",
            "Expose lifecycle route",
            "Route existing lifecycle guidance only after confirming the project's real maintenance, deprecation, or version-support policy.",
        ),
        baseline_pattern(
            PatternGroup::Aut,
            "common.automation.route_present",
            "Automation signal is visible",
            Some(RouteKind::Automation),
            PatternDetector::Route(RouteKind::Automation),
            BaselineRequirement::Optional,
            Severity::Info,
            GateKind::Guarded,
            "Automation signal is not visible",
            "The common baseline did not detect CI, workflow, badge, or automation evidence.",
            "Expose automation signal",
            "Add automation evidence only when the repository has a real workflow or test signal to expose.",
        ),
        candidate_pattern(
            PatternGroup::Sup,
            "SUP-001",
            "Support policy file is present",
            Some(RouteKind::Support),
            PatternDetector::ImportantFile(ImportantFileKind::Support),
            GateKind::Guarded,
            "Expose support policy file",
            "Prefer routing an existing SUPPORT file before drafting a new support policy.",
        ),
        candidate_pattern(
            PatternGroup::Sec,
            "SEC-001",
            "Security policy file is present",
            Some(RouteKind::Security),
            PatternDetector::ImportantFile(ImportantFileKind::Security),
            GateKind::Manual,
            "Expose security policy file",
            "Prefer routing an existing SECURITY file before proposing new disclosure policy content.",
        ),
        candidate_pattern(
            PatternGroup::Sec,
            "SEC-004",
            "Security automation workflow is present",
            Some(RouteKind::Security),
            PatternDetector::ImportantFile(ImportantFileKind::SecurityAutomation),
            GateKind::Guarded,
            "Expose security automation signal",
            "Surface existing CodeQL, vulnerability scanner, fuzzing, Scorecard, or security workflow evidence without claiming security outcomes.",
        ),
        candidate_pattern(
            PatternGroup::Sec,
            "SEC-007",
            "Dependency bot configuration is present",
            Some(RouteKind::Automation),
            PatternDetector::ImportantFile(ImportantFileKind::DependencyBot),
            GateKind::Guarded,
            "Expose dependency bot signal",
            "Surface existing Dependabot or Renovate configuration as supply-chain routing evidence without inventing update policy.",
        ),
        candidate_pattern(
            PatternGroup::Int,
            "INT-001",
            "Issue intake route is visible",
            Some(RouteKind::Support),
            PatternDetector::ReadmeRoute(RouteKind::Support),
            GateKind::Guarded,
            "Expose issue intake route",
            "Route users to the repository's real bug, feature, question, or discussion intake surface.",
        ),
        candidate_pattern(
            PatternGroup::Int,
            "INT-002",
            "Issue template file is present",
            Some(RouteKind::Intake),
            PatternDetector::ImportantFile(ImportantFileKind::IssueTemplate),
            GateKind::Guarded,
            "Expose issue template signal",
            "Route users to existing issue templates before proposing new triage structure.",
        ),
        candidate_pattern(
            PatternGroup::Int,
            "INT-003",
            "Issue forms YAML are present",
            Some(RouteKind::Intake),
            PatternDetector::ImportantFile(ImportantFileKind::IssueForm),
            GateKind::Guarded,
            "Expose issue forms signal",
            "Surface structured issue forms as intake evidence; form drafts require maintainer review of labels, required fields, and taxonomy.",
        ),
        candidate_pattern(
            PatternGroup::Int,
            "INT-010",
            "Pull request template is present",
            Some(RouteKind::Intake),
            PatternDetector::ImportantFile(ImportantFileKind::PullRequestTemplate),
            GateKind::Guarded,
            "Expose PR template signal",
            "Surface existing PR template evidence; draft review policy or checklist text only after maintainer confirmation.",
        ),
        candidate_pattern(
            PatternGroup::Aut,
            "AUT-001",
            "CI workflow file is present",
            Some(RouteKind::Automation),
            PatternDetector::ImportantFile(ImportantFileKind::Workflow),
            GateKind::Guarded,
            "Expose workflow signal",
            "Surface real workflow evidence instead of implying automation that does not exist.",
        ),
        candidate_pattern(
            PatternGroup::Aut,
            "AUT-009",
            "Security scan workflow is present",
            Some(RouteKind::Automation),
            PatternDetector::ImportantFile(ImportantFileKind::SecurityAutomation),
            GateKind::Guarded,
            "Expose security scan workflow",
            "Route existing CodeQL, SAST, vulnerability scanner, Scorecard, or fuzzing workflow evidence as automation context only.",
        ),
        candidate_pattern(
            PatternGroup::Rel,
            "REL-002",
            "Changelog file is present",
            Some(RouteKind::Release),
            PatternDetector::ImportantFile(ImportantFileKind::Changelog),
            GateKind::Guarded,
            "Expose changelog file",
            "Route users to release notes when they exist, especially for update-risk review.",
        ),
        candidate_pattern(
            PatternGroup::Lif,
            "LIF-001",
            "Lifecycle route is visible",
            Some(RouteKind::Lifecycle),
            PatternDetector::ReadmeRoute(RouteKind::Lifecycle),
            GateKind::Manual,
            "Expose lifecycle route",
            "Route existing maintenance, deprecation, supported-version, archival, or end-of-life guidance without inventing lifecycle policy.",
        ),
        candidate_pattern(
            PatternGroup::Own,
            "OWN-001",
            "Ownership file is present",
            Some(RouteKind::Ownership),
            PatternDetector::ImportantFile(ImportantFileKind::Codeowners),
            GateKind::Manual,
            "Expose ownership file",
            "Report owner or CODEOWNERS evidence without inventing maintainership.",
        ),
        candidate_pattern(
            PatternGroup::Gov,
            "GOV-001",
            "Governance route is visible",
            Some(RouteKind::Governance),
            PatternDetector::ReadmeRoute(RouteKind::Governance),
            GateKind::Manual,
            "Expose governance route",
            "Route governance material when it already exists; avoid inventing project authority.",
        ),
        candidate_pattern(
            PatternGroup::Hyg,
            "HYG-001",
            "Repository hygiene route or root hygiene evidence is visible",
            Some(RouteKind::Hygiene),
            PatternDetector::Route(RouteKind::Hygiene),
            GateKind::Guarded,
            "Expose hygiene route",
            "Use hygiene signals as review context, not as an automatic quality guarantee.",
        ),
    ])
}

#[must_use]
pub fn registry_document(registry: &PatternRegistry) -> PatternRegistryDocument {
    let groups = pattern_groups()
        .iter()
        .map(|group_definition| {
            let definitions = registry
                .definitions()
                .iter()
                .filter(|definition| definition.group == group_definition.group)
                .collect::<Vec<_>>();
            PatternGroupDocument {
                code: group_definition.group.code(),
                title: group_definition.title,
                description: group_definition.description,
                pattern_count: definitions.len(),
                baseline_count: definitions
                    .iter()
                    .filter(|definition| {
                        definition.adoption_stage == PatternAdoptionStage::CommonBaseline
                    })
                    .count(),
                candidate_count: definitions
                    .iter()
                    .filter(|definition| {
                        definition.adoption_stage == PatternAdoptionStage::Candidate
                    })
                    .count(),
            }
        })
        .collect::<Vec<_>>();

    let patterns = registry
        .definitions()
        .iter()
        .map(|definition| PatternDocument {
            id: definition.id,
            group: definition.group,
            title: definition.title,
            route: definition.route,
            detector_kind: detector_basis(definition.detector),
            detector: detector_label(definition.detector),
            requirement: definition.requirement,
            adoption_stage: definition.adoption_stage.as_str(),
            active_in_common_baseline: definition.adoption_stage.active_in_common_baseline(),
            missing_gate: definition.missing_gate,
            missing_severity: definition.missing_severity,
        })
        .collect::<Vec<_>>();

    PatternRegistryDocument {
        schema_version: SCHEMA_VERSION.to_string(),
        registry_version: "pattern_registry.v2",
        groups,
        patterns,
        claim_boundary: "Registry patterns are deterministic review rules and candidates. They are not popularity, trust, security, or quality guarantees.",
    }
}

pub fn registry_to_json(registry: &PatternRegistry) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&registry_document(registry))
}

pub fn common_registry_to_json() -> Result<String, serde_json::Error> {
    registry_to_json(&common_registry())
}

#[must_use]
pub fn render_registry_markdown(registry: &PatternRegistry) -> String {
    let document = registry_document(registry);
    let mut out = String::new();
    out.push_str("# RepoSeiri Pattern Registry\n\n");
    out.push_str(&format!("- Schema: `{}`\n", document.schema_version));
    out.push_str(&format!("- Registry: `{}`\n", document.registry_version));
    out.push_str(&format!("- Boundary: {}\n\n", document.claim_boundary));

    out.push_str("## Groups\n\n");
    for group in &document.groups {
        out.push_str(&format!(
            "- `{}` {}: `{}` patterns (`{}` baseline / `{}` candidate)\n",
            group.code,
            group.title,
            group.pattern_count,
            group.baseline_count,
            group.candidate_count
        ));
    }
    out.push('\n');

    for group in pattern_groups() {
        out.push_str(&format!("## {} - {}\n\n", group.group.code(), group.title));
        out.push_str(&format!("{}\n\n", group.description));
        let patterns = registry
            .definitions()
            .iter()
            .filter(|definition| definition.group == group.group)
            .collect::<Vec<_>>();
        if patterns.is_empty() {
            out.push_str("- No registered patterns.\n\n");
            continue;
        }
        for definition in patterns {
            let route = definition
                .route
                .map_or_else(|| "none".to_string(), |route| format!("{route:?}"));
            out.push_str(&format!(
                "- `{}` `{}` route `{}` detector `{}`: {}\n",
                definition.id,
                definition.adoption_stage.as_str(),
                route,
                detector_label(definition.detector),
                definition.title
            ));
        }
        out.push('\n');
    }

    out
}

#[must_use]
pub fn render_common_registry_markdown() -> String {
    render_registry_markdown(&common_registry())
}

#[allow(clippy::too_many_arguments)]
fn baseline_pattern(
    group: PatternGroup,
    id: &'static str,
    title: &'static str,
    route: Option<RouteKind>,
    detector: PatternDetector,
    requirement: BaselineRequirement,
    missing_severity: Severity,
    missing_gate: GateKind,
    missing_title: &'static str,
    missing_message: &'static str,
    recommendation_title: &'static str,
    recommendation_message: &'static str,
) -> PatternDefinition {
    pattern(
        group,
        id,
        title,
        route,
        detector,
        requirement,
        missing_severity,
        missing_gate,
        missing_title,
        missing_message,
        recommendation_title,
        recommendation_message,
        PatternAdoptionStage::CommonBaseline,
    )
}

#[allow(clippy::too_many_arguments)]
fn candidate_pattern(
    group: PatternGroup,
    id: &'static str,
    title: &'static str,
    route: Option<RouteKind>,
    detector: PatternDetector,
    missing_gate: GateKind,
    recommendation_title: &'static str,
    recommendation_message: &'static str,
) -> PatternDefinition {
    pattern(
        group,
        id,
        title,
        route,
        detector,
        BaselineRequirement::Optional,
        Severity::Info,
        missing_gate,
        "Candidate pattern is not active",
        "This registry candidate is not evaluated by the common baseline until calibration review adopts it.",
        recommendation_title,
        recommendation_message,
        PatternAdoptionStage::Candidate,
    )
}

#[allow(clippy::too_many_arguments)]
fn pattern(
    group: PatternGroup,
    id: &'static str,
    title: &'static str,
    route: Option<RouteKind>,
    detector: PatternDetector,
    requirement: BaselineRequirement,
    missing_severity: Severity,
    missing_gate: GateKind,
    missing_title: &'static str,
    missing_message: &'static str,
    recommendation_title: &'static str,
    recommendation_message: &'static str,
    adoption_stage: PatternAdoptionStage,
) -> PatternDefinition {
    PatternDefinition {
        id,
        group,
        title,
        route,
        detector,
        requirement,
        adoption_stage,
        missing_severity,
        missing_gate,
        missing_title,
        missing_message,
        recommendation_title,
        recommendation_message,
    }
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
    let evaluation_definitions = registry.evaluation_definitions();
    let pattern_matches = registry.evaluate_patterns(snapshot);
    let mut findings = Vec::new();
    let mut rules = Vec::new();
    let mut summary = BaselineSummary {
        required_present: 0,
        required_missing: 0,
        optional_present: 0,
        optional_missing: 0,
    };

    for (index, definition) in evaluation_definitions.iter().enumerate() {
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
        PatternDetector::EvidenceKind(kind) => {
            if let Some(ledger_ids) = ledger_legacy_ids(snapshot, |record| {
                record.scope == EvidenceScope::Root && record.kind == kind
            }) {
                ledger_ids
            } else {
                snapshot
                    .evidence
                    .iter()
                    .filter(|evidence| evidence.kind == kind)
                    .map(|evidence| evidence.id.clone())
                    .collect()
            }
        }
        PatternDetector::Route(route) => {
            if let Some(ledger_ids) = ledger_legacy_ids(snapshot, |record| {
                record.scope == EvidenceScope::Root && record.route == Some(route)
            }) {
                ledger_ids
            } else {
                snapshot
                    .evidence
                    .iter()
                    .filter(|evidence| evidence.route == Some(route))
                    .map(|evidence| evidence.id.clone())
                    .collect()
            }
        }
        PatternDetector::ReadmeRoute(route) => {
            if let Some(ledger_ids) = ledger_legacy_ids(snapshot, |record| {
                record.scope == EvidenceScope::Root
                    && record.route == Some(route)
                    && matches!(
                        record.kind,
                        EvidenceKind::MarkdownHeading
                            | EvidenceKind::MarkdownLink
                            | EvidenceKind::RouteCandidate
                    )
            }) {
                ledger_ids
            } else {
                snapshot
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
                    .collect()
            }
        }
        PatternDetector::ImportantFile(kind) => {
            let expected = format!("{kind:?}");
            if let Some(ledger_ids) = ledger_legacy_ids(snapshot, |record| {
                record.scope == EvidenceScope::Root
                    && record.kind == EvidenceKind::ImportantFile
                    && record.value == expected
            }) {
                ledger_ids
            } else {
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
}

fn ledger_legacy_ids(
    snapshot: &RepoSnapshot,
    predicate: impl Fn(&seiri_core::EvidenceRecord) -> bool,
) -> Option<Vec<String>> {
    if snapshot.evidence_ledger.is_empty() {
        return None;
    }

    Some(
        snapshot
            .evidence_ledger
            .iter()
            .filter(|record| predicate(record))
            .map(|record| {
                record
                    .legacy_evidence_id
                    .clone()
                    .unwrap_or_else(|| record.id.clone())
            })
            .collect(),
    )
}

fn detector_basis(detector: PatternDetector) -> &'static str {
    match detector {
        PatternDetector::EvidenceKind(_) => "evidence kind",
        PatternDetector::Route(_) => "trust route",
        PatternDetector::ReadmeRoute(_) => "README trust route",
        PatternDetector::ImportantFile(_) => "important file",
    }
}

fn detector_label(detector: PatternDetector) -> String {
    match detector {
        PatternDetector::EvidenceKind(kind) => format!("evidence kind:{kind:?}"),
        PatternDetector::Route(route) => format!("trust route:{route:?}"),
        PatternDetector::ReadmeRoute(route) => format!("README route:{route:?}"),
        PatternDetector::ImportantFile(kind) => format!("important file:{kind:?}"),
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
