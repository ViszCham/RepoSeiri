use crate::{
    ClaimBoundaryKind, CoverageScope, EvidenceId, ImportantFileKind, MeaningAtom, Observation,
    RepositoryFacet, RouteContentAtom, RouteKind, SourceSpan,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ContentSlotId(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentSlotKind {
    Statement,
    Route,
    Command,
    ExpectedOutput,
    Policy,
    Version,
    Ownership,
    Artifact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicySensitivity {
    EvidenceOnly,
    MaintainerDecision,
    SecuritySensitive,
    LegalSensitive,
    ExecutionSensitive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ContentSlotSpec {
    pub id: ContentSlotId,
    pub code: &'static str,
    pub route: RouteKind,
    pub kind: ContentSlotKind,
    pub sensitivity: PolicySensitivity,
    pub scope: CoverageScope,
    pub enabled_by_any_facet: &'static [RepositoryFacet],
    pub markers: &'static [&'static str],
    pub important_files: &'static [ImportantFileKind],
    pub indicates: &'static [MeaningAtom],
    pub does_not_indicate: &'static [ClaimBoundaryKind],
    #[serde(skip)]
    pub pattern_atom: Option<RouteContentAtom>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MeaningAtomSet(pub Vec<MeaningAtom>);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentSlotAssessment {
    pub slot: ContentSlotId,
    pub code: String,
    pub route: RouteKind,
    pub enabled: bool,
    pub condition_evidence_ids: Vec<EvidenceId>,
    pub sensitivity: PolicySensitivityWire,
    pub observation: Observation<MeaningAtomSet>,
    pub indicates: Vec<MeaningAtom>,
    pub does_not_indicate: Vec<ClaimBoundaryKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicySensitivityWire {
    EvidenceOnly,
    MaintainerDecision,
    SecuritySensitive,
    LegalSensitive,
    ExecutionSensitive,
}

impl From<PolicySensitivity> for PolicySensitivityWire {
    fn from(value: PolicySensitivity) -> Self {
        match value {
            PolicySensitivity::EvidenceOnly => Self::EvidenceOnly,
            PolicySensitivity::MaintainerDecision => Self::MaintainerDecision,
            PolicySensitivity::SecuritySensitive => Self::SecuritySensitive,
            PolicySensitivity::LegalSensitive => Self::LegalSensitive,
            PolicySensitivity::ExecutionSensitive => Self::ExecutionSensitive,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BilingualStructuralPair {
    pub document_path: String,
    pub left_heading: SourceSpan,
    pub right_heading: SourceSpan,
    pub normalized_targets: Vec<String>,
    pub evidence_ids: Vec<EvidenceId>,
    pub candidate_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteContentReport {
    pub assessments: Vec<ContentSlotAssessment>,
    pub structural_pairs: Vec<BilingualStructuralPair>,
    pub boundary: String,
}

impl Default for RouteContentReport {
    fn default() -> Self {
        Self {
            assessments: Vec::new(),
            structural_pairs: Vec::new(),
            boundary: "Content slots are bounded repository observations. Structural JA/EN pairs share normalized targets only; they do not establish translation or semantic equivalence. Expected output is not command success. Security-policy observations retain the NotSecurityGuarantee boundary.".to_string(),
        }
    }
}

impl RouteContentReport {
    #[must_use]
    pub fn assessment(&self, id: ContentSlotId) -> Option<&ContentSlotAssessment> {
        self.assessments.iter().find(|item| item.slot == id)
    }
}

const OBSERVED: &[MeaningAtom] = &[MeaningAtom::ContentSlotObserved];
const EXPECTED: &[MeaningAtom] = &[
    MeaningAtom::ContentSlotObserved,
    MeaningAtom::ExpectedOutputDocumented,
];
const NONE: &[ClaimBoundaryKind] = &[];
const SECURITY_BOUNDARY: &[ClaimBoundaryKind] = &[ClaimBoundaryKind::NotSecurityGuarantee];
const LEGAL_BOUNDARY: &[ClaimBoundaryKind] = &[
    ClaimBoundaryKind::NotLegalFitnessGuarantee,
    ClaimBoundaryKind::NotLegalAdvice,
];
const RUNTIME_BOUNDARY: &[ClaimBoundaryKind] = &[ClaimBoundaryKind::NotRuntimeVerification];

macro_rules! slot {
    ($id:literal, $code:literal, $route:ident, $kind:ident, $sensitivity:ident,
     $scope:expr, $facets:expr, $markers:expr, $files:expr, $meaning:expr, $boundaries:expr,
     $pattern_atom:expr) => {
        ContentSlotSpec {
            id: ContentSlotId($id),
            code: $code,
            route: RouteKind::$route,
            kind: ContentSlotKind::$kind,
            sensitivity: PolicySensitivity::$sensitivity,
            scope: $scope,
            enabled_by_any_facet: $facets,
            markers: $markers,
            important_files: $files,
            indicates: $meaning,
            does_not_indicate: $boundaries,
            pattern_atom: $pattern_atom,
        }
    };
}

const PACKAGE_OR_BINARY: &[RepositoryFacet] = &[RepositoryFacet::Package, RepositoryFacet::Binary];
const PACKAGE: &[RepositoryFacet] = &[RepositoryFacet::Package];
const PRODUCT_OR_BINARY: &[RepositoryFacet] = &[RepositoryFacet::Product, RepositoryFacet::Binary];
const EMPTY_FACETS: &[RepositoryFacet] = &[];

pub static ROUTE_CONTENT_CONTRACT: &[ContentSlotSpec] = &[
    slot!(
        1,
        "identity.purpose",
        Identity,
        Statement,
        EvidenceOnly,
        CoverageScope::MarkdownDocuments,
        EMPTY_FACETS,
        &["purpose", "overview", "about", "what is"],
        &[],
        OBSERVED,
        NONE,
        Some(RouteContentAtom::IdentityPurpose)
    ),
    slot!(
        2,
        "identity.audience",
        Identity,
        Statement,
        EvidenceOnly,
        CoverageScope::MarkdownDocuments,
        EMPTY_FACETS,
        &["audience", "for users", "who should"],
        &[],
        OBSERVED,
        NONE,
        Some(RouteContentAtom::IdentityAudienceOrScope)
    ),
    slot!(
        3,
        "identity.status",
        Identity,
        Statement,
        MaintainerDecision,
        CoverageScope::MarkdownDocuments,
        EMPTY_FACETS,
        &["status", "experimental", "stable"],
        &[],
        OBSERVED,
        &[ClaimBoundaryKind::NotMaintenanceGuarantee],
        None
    ),
    slot!(
        4,
        "identity.scope",
        Identity,
        Statement,
        EvidenceOnly,
        CoverageScope::MarkdownDocuments,
        EMPTY_FACETS,
        &["scope", "out of scope", "non-goal"],
        &[],
        OBSERVED,
        NONE,
        Some(RouteContentAtom::IdentityAudienceOrScope)
    ),
    slot!(
        5,
        "docs.index",
        Docs,
        Route,
        EvidenceOnly,
        CoverageScope::MarkdownDocuments,
        EMPTY_FACETS,
        &["documentation", "docs", "manual"],
        &[ImportantFileKind::DocsDirectory],
        OBSERVED,
        NONE,
        Some(RouteContentAtom::DocsNavigation)
    ),
    slot!(
        6,
        "docs.user_guide",
        Docs,
        Route,
        EvidenceOnly,
        CoverageScope::MarkdownDocuments,
        EMPTY_FACETS,
        &["user guide", "guide", "tutorial", "concept"],
        &[],
        OBSERVED,
        NONE,
        Some(RouteContentAtom::DocsConceptGuide)
    ),
    slot!(
        7,
        "docs.api",
        Docs,
        Route,
        EvidenceOnly,
        CoverageScope::MarkdownDocuments,
        PACKAGE,
        &["api", "reference", "rustdoc"],
        &[],
        OBSERVED,
        NONE,
        None
    ),
    slot!(
        8,
        "docs.developer",
        Docs,
        Route,
        EvidenceOnly,
        CoverageScope::MarkdownDocuments,
        EMPTY_FACETS,
        &["architecture", "developer", "design"],
        &[],
        OBSERVED,
        NONE,
        Some(RouteContentAtom::DocsConceptGuide)
    ),
    slot!(
        9,
        "docs.version",
        Docs,
        Version,
        MaintainerDecision,
        CoverageScope::MarkdownDocuments,
        EMPTY_FACETS,
        &["versioned docs", "documentation version", "latest version"],
        &[],
        OBSERVED,
        NONE,
        None
    ),
    slot!(
        10,
        "quickstart.prerequisites",
        Quickstart,
        Statement,
        ExecutionSensitive,
        CoverageScope::MarkdownDocuments,
        EMPTY_FACETS,
        &["prerequisite", "requirements", "requires rust"],
        &[],
        OBSERVED,
        RUNTIME_BOUNDARY,
        None
    ),
    slot!(
        11,
        "quickstart.install",
        Quickstart,
        Command,
        ExecutionSensitive,
        CoverageScope::MarkdownDocuments,
        EMPTY_FACETS,
        &["install", "cargo install", "build"],
        &[],
        OBSERVED,
        RUNTIME_BOUNDARY,
        Some(RouteContentAtom::QuickstartInstallation)
    ),
    slot!(
        12,
        "quickstart.first_action",
        Quickstart,
        Command,
        ExecutionSensitive,
        CoverageScope::MarkdownDocuments,
        EMPTY_FACETS,
        &["getting started", "first run", "usage", "cargo run"],
        &[],
        OBSERVED,
        RUNTIME_BOUNDARY,
        Some(RouteContentAtom::QuickstartFirstRun)
    ),
    slot!(
        13,
        "quickstart.minimal_example",
        Quickstart,
        Command,
        ExecutionSensitive,
        CoverageScope::MarkdownDocuments,
        EMPTY_FACETS,
        &["example", "minimal example", "hello"],
        &[],
        OBSERVED,
        RUNTIME_BOUNDARY,
        Some(RouteContentAtom::QuickstartFirstRun)
    ),
    slot!(
        14,
        "quickstart.expected_output",
        Quickstart,
        ExpectedOutput,
        ExecutionSensitive,
        CoverageScope::MarkdownDocuments,
        EMPTY_FACETS,
        &["expected output", "output:"],
        &[],
        EXPECTED,
        RUNTIME_BOUNDARY,
        None
    ),
    slot!(
        15,
        "support.channel",
        Support,
        Route,
        MaintainerDecision,
        CoverageScope::MarkdownDocuments,
        EMPTY_FACETS,
        &["support", "discussion", "contact", "help"],
        &[ImportantFileKind::Support],
        OBSERVED,
        NONE,
        Some(RouteContentAtom::SupportQuestionChannel)
    ),
    slot!(
        16,
        "support.question_type",
        Support,
        Policy,
        MaintainerDecision,
        CoverageScope::MarkdownDocuments,
        EMPTY_FACETS,
        &["question", "how-to", "usage question"],
        &[],
        OBSERVED,
        NONE,
        Some(RouteContentAtom::SupportQuestionChannel)
    ),
    slot!(
        17,
        "support.scope",
        Support,
        Policy,
        MaintainerDecision,
        CoverageScope::MarkdownDocuments,
        EMPTY_FACETS,
        &["support scope", "supported", "unsupported"],
        &[],
        OBSERVED,
        NONE,
        None
    ),
    slot!(
        18,
        "support.response_expectation",
        Support,
        Policy,
        MaintainerDecision,
        CoverageScope::MarkdownDocuments,
        EMPTY_FACETS,
        &["response", "reply", "best effort"],
        &[],
        OBSERVED,
        &[ClaimBoundaryKind::NotMaintenanceGuarantee],
        Some(RouteContentAtom::SupportResponseExpectation)
    ),
    slot!(
        19,
        "intake.bug",
        Intake,
        Route,
        MaintainerDecision,
        CoverageScope::RepositoryFiles,
        EMPTY_FACETS,
        &["bug report", "bug"],
        &[
            ImportantFileKind::IssueTemplate,
            ImportantFileKind::IssueForm
        ],
        OBSERVED,
        NONE,
        None
    ),
    slot!(
        20,
        "intake.feature",
        Intake,
        Route,
        MaintainerDecision,
        CoverageScope::RepositoryFiles,
        EMPTY_FACETS,
        &["feature request", "enhancement"],
        &[
            ImportantFileKind::IssueTemplate,
            ImportantFileKind::IssueForm
        ],
        OBSERVED,
        NONE,
        None
    ),
    slot!(
        21,
        "intake.docs",
        Intake,
        Route,
        MaintainerDecision,
        CoverageScope::RepositoryFiles,
        EMPTY_FACETS,
        &["documentation issue", "docs issue"],
        &[
            ImportantFileKind::IssueTemplate,
            ImportantFileKind::IssueForm
        ],
        OBSERVED,
        NONE,
        None
    ),
    slot!(
        22,
        "intake.question",
        Intake,
        Route,
        MaintainerDecision,
        CoverageScope::MarkdownDocuments,
        EMPTY_FACETS,
        &["question", "discussion"],
        &[],
        OBSERVED,
        NONE,
        None
    ),
    slot!(
        23,
        "intake.security",
        Intake,
        Route,
        SecuritySensitive,
        CoverageScope::MarkdownDocuments,
        EMPTY_FACETS,
        &["security", "vulnerability", "private report"],
        &[ImportantFileKind::Security],
        OBSERVED,
        SECURITY_BOUNDARY,
        Some(RouteContentAtom::IntakeSecurityRedirect)
    ),
    slot!(
        24,
        "intake.reproduction",
        Intake,
        Policy,
        EvidenceOnly,
        CoverageScope::RepositoryFiles,
        EMPTY_FACETS,
        &["reproduce", "reproduction", "steps to reproduce"],
        &[ImportantFileKind::IssueForm],
        OBSERVED,
        NONE,
        Some(RouteContentAtom::IntakeReproductionContext)
    ),
    slot!(
        25,
        "intake.version_environment",
        Intake,
        Policy,
        EvidenceOnly,
        CoverageScope::RepositoryFiles,
        EMPTY_FACETS,
        &["environment", "version", "operating system"],
        &[ImportantFileKind::IssueForm],
        OBSERVED,
        NONE,
        Some(RouteContentAtom::IntakeReproductionContext)
    ),
    slot!(
        26,
        "contributing.setup",
        Contributing,
        Command,
        ExecutionSensitive,
        CoverageScope::MarkdownDocuments,
        EMPTY_FACETS,
        &["development setup", "contributing", "checkout"],
        &[ImportantFileKind::Contributing],
        OBSERVED,
        RUNTIME_BOUNDARY,
        Some(RouteContentAtom::ContributingDevelopmentSetup)
    ),
    slot!(
        27,
        "contributing.test",
        Contributing,
        Command,
        ExecutionSensitive,
        CoverageScope::MarkdownDocuments,
        EMPTY_FACETS,
        &["cargo test", "test", "clippy", "lint"],
        &[],
        OBSERVED,
        RUNTIME_BOUNDARY,
        Some(RouteContentAtom::ContributingValidationCommand)
    ),
    slot!(
        28,
        "contributing.review_prerequisite",
        Contributing,
        Policy,
        MaintainerDecision,
        CoverageScope::RepositoryFiles,
        EMPTY_FACETS,
        &["pull request", "review", "before submitting"],
        &[ImportantFileKind::PullRequestTemplate],
        OBSERVED,
        NONE,
        None
    ),
    slot!(
        29,
        "contributing.acceptance_boundary",
        Contributing,
        Policy,
        MaintainerDecision,
        CoverageScope::MarkdownDocuments,
        EMPTY_FACETS,
        &["acceptance", "may be rejected", "maintainer decision"],
        &[],
        OBSERVED,
        &[ClaimBoundaryKind::NotOwnerApproval],
        None
    ),
    slot!(
        30,
        "security.private_disclosure",
        Security,
        Route,
        SecuritySensitive,
        CoverageScope::MarkdownDocuments,
        EMPTY_FACETS,
        &[
            "private",
            "privately",
            "security advisory",
            "disclosure",
            "report",
            "vulnerability",
        ],
        &[ImportantFileKind::Security],
        OBSERVED,
        SECURITY_BOUNDARY,
        Some(RouteContentAtom::SecurityDisclosureChannel)
    ),
    slot!(
        31,
        "security.supported_versions",
        Security,
        Version,
        SecuritySensitive,
        CoverageScope::MarkdownDocuments,
        EMPTY_FACETS,
        &[
            "supported versions",
            "version support",
            "security policy",
            "security scope",
        ],
        &[],
        OBSERVED,
        SECURITY_BOUNDARY,
        Some(RouteContentAtom::SecurityPolicyScope)
    ),
    slot!(
        32,
        "security.response_expectation",
        Security,
        Policy,
        SecuritySensitive,
        CoverageScope::MarkdownDocuments,
        EMPTY_FACETS,
        &["response", "acknowledge", "disclosure timeline"],
        &[],
        OBSERVED,
        SECURITY_BOUNDARY,
        None
    ),
    slot!(
        33,
        "security.automation",
        Security,
        Artifact,
        SecuritySensitive,
        CoverageScope::RepositoryFiles,
        EMPTY_FACETS,
        &["security scan", "codeql", "audit"],
        &[ImportantFileKind::SecurityAutomation],
        OBSERVED,
        SECURITY_BOUNDARY,
        None
    ),
    slot!(
        34,
        "release.changelog",
        Release,
        Route,
        EvidenceOnly,
        CoverageScope::RepositoryFiles,
        EMPTY_FACETS,
        &["changelog", "changes"],
        &[ImportantFileKind::Changelog],
        OBSERVED,
        NONE,
        Some(RouteContentAtom::ReleaseChangeHistory)
    ),
    slot!(
        35,
        "release.channel",
        Release,
        Route,
        MaintainerDecision,
        CoverageScope::MarkdownDocuments,
        EMPTY_FACETS,
        &["release", "releases", "distribution"],
        &[],
        OBSERVED,
        NONE,
        None
    ),
    slot!(
        36,
        "release.compatibility",
        Release,
        Statement,
        MaintainerDecision,
        CoverageScope::MarkdownDocuments,
        PACKAGE_OR_BINARY,
        &["compatibility", "breaking change", "semver"],
        &[],
        OBSERVED,
        NONE,
        Some(RouteContentAtom::ReleaseCompatibilityNotes)
    ),
    slot!(
        37,
        "release.migration",
        Release,
        Route,
        MaintainerDecision,
        CoverageScope::MarkdownDocuments,
        EMPTY_FACETS,
        &["migration", "upgrade guide"],
        &[],
        OBSERVED,
        NONE,
        Some(RouteContentAtom::ReleaseCompatibilityNotes)
    ),
    slot!(
        38,
        "release.deprecation",
        Release,
        Statement,
        MaintainerDecision,
        CoverageScope::MarkdownDocuments,
        EMPTY_FACETS,
        &["deprecation", "deprecated"],
        &[],
        OBSERVED,
        &[ClaimBoundaryKind::NotMaintenanceGuarantee],
        None
    ),
    slot!(
        39,
        "lifecycle.status",
        Lifecycle,
        Statement,
        MaintainerDecision,
        CoverageScope::MarkdownDocuments,
        EMPTY_FACETS,
        &["maintenance", "maintained", "archived", "experimental"],
        &[],
        OBSERVED,
        &[ClaimBoundaryKind::NotMaintenanceGuarantee],
        Some(RouteContentAtom::LifecycleMaintenanceStatus)
    ),
    slot!(
        40,
        "lifecycle.successor",
        Lifecycle,
        Route,
        MaintainerDecision,
        CoverageScope::MarkdownDocuments,
        EMPTY_FACETS,
        &["successor", "replacement", "superseded"],
        &[],
        OBSERVED,
        NONE,
        None
    ),
    slot!(
        41,
        "lifecycle.migration",
        Lifecycle,
        Route,
        MaintainerDecision,
        CoverageScope::MarkdownDocuments,
        EMPTY_FACETS,
        &["migration", "move to", "upgrade", "end of life"],
        &[],
        OBSERVED,
        NONE,
        Some(RouteContentAtom::LifecycleDeprecationPlan)
    ),
    slot!(
        42,
        "governance.decision",
        Governance,
        Policy,
        MaintainerDecision,
        CoverageScope::MarkdownDocuments,
        EMPTY_FACETS,
        &["governance", "decision", "rfc"],
        &[],
        OBSERVED,
        NONE,
        Some(RouteContentAtom::GovernanceDecisionProcess)
    ),
    slot!(
        43,
        "governance.proposal",
        Governance,
        Route,
        MaintainerDecision,
        CoverageScope::MarkdownDocuments,
        EMPTY_FACETS,
        &["proposal", "rfc", "request for comments"],
        &[],
        OBSERVED,
        NONE,
        Some(RouteContentAtom::GovernanceDecisionProcess)
    ),
    slot!(
        44,
        "governance.roles",
        Governance,
        Ownership,
        MaintainerDecision,
        CoverageScope::MarkdownDocuments,
        EMPTY_FACETS,
        &["maintainer", "steward", "owner"],
        &[],
        OBSERVED,
        &[ClaimBoundaryKind::NotOwnerApproval],
        Some(RouteContentAtom::GovernanceMaintainerRole)
    ),
    slot!(
        45,
        "governance.record",
        Governance,
        Artifact,
        EvidenceOnly,
        CoverageScope::MarkdownDocuments,
        EMPTY_FACETS,
        &["decision record", "adr", "minutes"],
        &[],
        OBSERVED,
        NONE,
        None
    ),
    slot!(
        46,
        "license.local_file",
        License,
        Artifact,
        LegalSensitive,
        CoverageScope::RepositoryFiles,
        EMPTY_FACETS,
        &[],
        &[ImportantFileKind::License],
        OBSERVED,
        LEGAL_BOUNDARY,
        Some(RouteContentAtom::LicenseReference)
    ),
    slot!(
        47,
        "license.readme_route",
        License,
        Route,
        LegalSensitive,
        CoverageScope::MarkdownDocuments,
        EMPTY_FACETS,
        &["license", "licensing"],
        &[],
        OBSERVED,
        LEGAL_BOUNDARY,
        Some(RouteContentAtom::LicenseUsageTerms)
    ),
    slot!(
        48,
        "license.scope",
        License,
        Policy,
        LegalSensitive,
        CoverageScope::MarkdownDocuments,
        EMPTY_FACETS,
        &["licensed under", "usage terms", "copyright"],
        &[],
        OBSERVED,
        LEGAL_BOUNDARY,
        Some(RouteContentAtom::LicenseUsageTerms)
    ),
    slot!(
        49,
        "license.additional_artifacts",
        License,
        Policy,
        LegalSensitive,
        CoverageScope::MarkdownDocuments,
        EMPTY_FACETS,
        &["third-party license", "assets license", "notices"],
        &[],
        OBSERVED,
        LEGAL_BOUNDARY,
        None
    ),
    slot!(
        50,
        "automation.triggers",
        Automation,
        Policy,
        ExecutionSensitive,
        CoverageScope::RepositoryFiles,
        EMPTY_FACETS,
        &["workflow_dispatch", "pull_request", "push"],
        &[ImportantFileKind::Workflow],
        OBSERVED,
        RUNTIME_BOUNDARY,
        Some(RouteContentAtom::AutomationWorkflowReference)
    ),
    slot!(
        51,
        "automation.job_classes",
        Automation,
        Policy,
        ExecutionSensitive,
        CoverageScope::RepositoryFiles,
        EMPTY_FACETS,
        &["test", "lint", "build", "release"],
        &[ImportantFileKind::Workflow],
        OBSERVED,
        RUNTIME_BOUNDARY,
        None
    ),
    slot!(
        52,
        "automation.permissions",
        Automation,
        Policy,
        SecuritySensitive,
        CoverageScope::RepositoryFiles,
        EMPTY_FACETS,
        &["permissions", "contents: read"],
        &[ImportantFileKind::Workflow],
        OBSERVED,
        SECURITY_BOUNDARY,
        None
    ),
    slot!(
        53,
        "automation.action_dependencies",
        Automation,
        Artifact,
        ExecutionSensitive,
        CoverageScope::RepositoryFiles,
        EMPTY_FACETS,
        &["uses:"],
        &[
            ImportantFileKind::Workflow,
            ImportantFileKind::DependencyBot
        ],
        OBSERVED,
        RUNTIME_BOUNDARY,
        Some(RouteContentAtom::AutomationWorkflowReference)
    ),
    ContentSlotSpec {
        id: ContentSlotId(54),
        code: "automation.release_docs_security",
        route: RouteKind::Automation,
        kind: ContentSlotKind::Artifact,
        sensitivity: PolicySensitivity::ExecutionSensitive,
        scope: CoverageScope::RepositoryFiles,
        enabled_by_any_facet: EMPTY_FACETS,
        markers: &["release", "docs", "codeql", "audit"],
        important_files: &[
            ImportantFileKind::Workflow,
            ImportantFileKind::SecurityAutomation,
        ],
        indicates: OBSERVED,
        does_not_indicate: RUNTIME_BOUNDARY,
        pattern_atom: Some(RouteContentAtom::AutomationStatusSignal),
    },
    slot!(
        55,
        "ownership.codeowners",
        Ownership,
        Artifact,
        MaintainerDecision,
        CoverageScope::RepositoryFiles,
        EMPTY_FACETS,
        &[],
        &[ImportantFileKind::Codeowners],
        OBSERVED,
        &[ClaimBoundaryKind::NotOwnerApproval],
        Some(RouteContentAtom::OwnershipReference)
    ),
    slot!(
        56,
        "ownership.critical_coverage",
        Ownership,
        Ownership,
        MaintainerDecision,
        CoverageScope::RepositoryFiles,
        EMPTY_FACETS,
        &["critical", "/src/", "/.github/", "codeowners", "ownership"],
        &[ImportantFileKind::Codeowners],
        OBSERVED,
        &[ClaimBoundaryKind::NotOwnerApproval],
        Some(RouteContentAtom::OwnershipCriticalPath)
    ),
    slot!(
        57,
        "ownership.token_syntax",
        Ownership,
        Ownership,
        EvidenceOnly,
        CoverageScope::RepositoryFiles,
        EMPTY_FACETS,
        &["@", "*"],
        &[ImportantFileKind::Codeowners],
        OBSERVED,
        &[ClaimBoundaryKind::NotOwnerApproval],
        None
    ),
    slot!(
        58,
        "ownership.uncovered_scope",
        Ownership,
        Policy,
        MaintainerDecision,
        CoverageScope::RepositoryFiles,
        EMPTY_FACETS,
        &["unowned", "uncovered", "fallback owner"],
        &[],
        OBSERVED,
        &[ClaimBoundaryKind::NotOwnerApproval],
        None
    ),
    slot!(
        59,
        "hygiene.ignored_artifacts",
        Hygiene,
        Policy,
        EvidenceOnly,
        CoverageScope::RepositoryFiles,
        EMPTY_FACETS,
        &["target/", ".env", "ignore"],
        &[ImportantFileKind::Gitignore],
        OBSERVED,
        NONE,
        Some(RouteContentAtom::HygieneGeneratedArtifactPolicy)
    ),
    slot!(
        60,
        "hygiene.large_files",
        Hygiene,
        Policy,
        EvidenceOnly,
        CoverageScope::RepositoryFiles,
        EMPTY_FACETS,
        &["large file", "lfs"],
        &[ImportantFileKind::Gitattributes],
        OBSERVED,
        NONE,
        None
    ),
    slot!(
        61,
        "hygiene.generated",
        Hygiene,
        Policy,
        EvidenceOnly,
        CoverageScope::RepositoryFiles,
        EMPTY_FACETS,
        &["generated", "linguist-generated"],
        &[ImportantFileKind::Gitattributes],
        OBSERVED,
        NONE,
        Some(RouteContentAtom::HygieneGeneratedArtifactPolicy)
    ),
    slot!(
        62,
        "hygiene.vendored",
        Hygiene,
        Policy,
        EvidenceOnly,
        CoverageScope::RepositoryFiles,
        EMPTY_FACETS,
        &["vendored", "vendor/"],
        &[ImportantFileKind::Gitattributes],
        OBSERVED,
        NONE,
        None
    ),
    ContentSlotSpec {
        id: ContentSlotId(63),
        code: "hygiene.storage",
        route: RouteKind::Hygiene,
        kind: ContentSlotKind::Policy,
        sensitivity: PolicySensitivity::MaintainerDecision,
        scope: CoverageScope::RepositoryFiles,
        enabled_by_any_facet: PRODUCT_OR_BINARY,
        markers: &[
            "artifact storage",
            "cache",
            "retention",
            "format",
            "formatting",
            "style",
            "editorconfig",
        ],
        important_files: &[ImportantFileKind::EditorConfig],
        indicates: OBSERVED,
        does_not_indicate: NONE,
        pattern_atom: Some(RouteContentAtom::HygieneFormattingPolicy),
    },
];

#[must_use]
pub fn route_content_contract() -> &'static [ContentSlotSpec] {
    ROUTE_CONTENT_CONTRACT
}
