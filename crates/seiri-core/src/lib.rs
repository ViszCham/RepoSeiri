#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

mod audit_delta;
mod calibration_prior;
mod codex_view;
mod document_index;
mod document_scan;
mod evidence_kernel;
mod facets;
mod github_local;
mod obligation_graph;
mod observation;
mod patch_proposal;
mod profile_semantics;
mod remote_evidence;
mod repository_scope;
mod review_priority;
mod route_assessment;
mod route_content;
mod route_content_contract;
mod route_target;

pub use calibration_prior::{
    AggregatePrior, AggregatePriorError, CalibrationKey, CalibrationLookup, CalibrationProvider,
    CalibrationUnavailableReason, NoCalibrationProvider, PriorBasis, PriorVisibility,
};

pub use codex_view::{CodexAction, CodexCommand, CodexCommandError, CODEX_SCHEMA_VERSION};
pub use document_index::{
    DocumentIndex, DocumentIndexError, DocumentRole, DocumentRoleCoverage, DocumentScanStatus,
    IndexedDocument,
};
pub use document_scan::{
    DocumentDiagnostic, DocumentDiagnosticKind, DocumentEvent, DocumentScan,
    DocumentScanInvariantError,
};
pub use evidence_kernel::{
    stable_evidence_id, ByteOffset, DocumentId, DocumentRecord, EvidenceAtom, EvidenceDraft,
    EvidenceFact, EvidenceId, EvidenceKernel, EvidenceKernelError, EvidenceProducer,
    EvidenceProvenance, EvidenceSourceSpan, MarkdownEvidenceKind, ParseEvidenceIdError,
    ReadmePresence, SourceDomain,
};
pub use facets::{
    facet_evidence_ids, FacetAssessment, FacetReport, FacetReportError, RepositoryFacet,
};
pub use github_local::{
    ActionReference, ActionReferenceKind, CodeownerEntry, Codeowners, CodeownersOp,
    CodeownersPatternProgram, CodeownersSkippedLine, CriticalPathCoverage, CriticalPathKind,
    DependencyBot, DependencyBotProvider, DependencyUpdate, GithubDiagnostic, GithubDiagnosticKind,
    GithubDocumentIr, GithubDocumentKind, GithubLocalDocument, GithubLocalDocumentError,
    GithubLocalDocuments, GithubLocalDocumentsError, GithubParseStatus, GithubSemanticsReport,
    IssueForm, IssueFormField, IssueFormFieldKind, IssueFormRequiredFields, IssueRouteCandidate,
    IssueRouteCandidateKind, PermissionEntry, PermissionSet, StaticUnknownReason, StaticValue,
    StructuredBudgetKind, TokenPermission, Workflow, WorkflowJob, WorkflowJobCandidate,
    WorkflowJobCandidateKind, WorkflowStep, WorkflowTrigger,
};
pub use obligation_graph::{
    ConditionalObligation, DocumentConflict, DocumentConflictSide, DocumentConsistencyError,
    DocumentConsistencyReport, DocumentTargetRelation,
};
pub use observation::{
    CoverageId, CoverageIncompleteReason, CoverageIndex, CoverageIndexError, CoverageRecord,
    CoverageScope, CoverageStatus, EvidenceSet, Observation, ObservationError, UnknownReason,
};
pub use patch_proposal::{
    PatchAnalysisRun, PatchAnchorContext, PatchAnchorSlice, PatchBaseDigest, PatchEditContent,
    PatchProposal, PatchProposalApplyError, PatchProposalBinding, PatchProposalBindingError,
    PatchProposalDecision, PatchProposalIssue, PatchProposalIssueKind, PatchProposalPreflight,
    PatchTextEdit, PolicySlotKind, TextDocumentBase, TextEditSpan, TextEncoding, TextLineEnding,
    UnresolvedPolicySlot, PATCH_ANCHOR_CONTEXT_BYTES, PATCH_PROPOSAL_SCHEMA_VERSION,
};
pub use profile_semantics::{
    CalibrationPriorState, ProfileBranchSemantics, ProfileEvidenceMatch, ProfileFit,
    ProfileRankScore,
};
pub use remote_evidence::{
    RemoteEvidenceReport, RemoteEvidenceStatus, RemoteRepositoryMetadata,
    RemoteRepositoryReference, RemoteRepositoryReferenceError, RemoteUnavailableReason,
};
pub use repository_scope::{
    AnalysisScope, FreshnessReport, GitCommitHeader, GitDiagnostic, GitDiagnosticKind,
    GitObservationState, GitReadBudget, GitReferenceKind, GitReferenceObservation,
    GitTemporalObservation, GitTimestamp, IgnoredPathReason, IgnoredShallowRecord,
    LifecycleSignalFreshness, ManifestObservationStatus, RepositoryRootKind, RepositoryScopeGraph,
    RepositoryScopeReport, RepositoryScopeRoot, ScopeEdge, ScopeEdgeKind, ScopeNode, ScopeNodeId,
    ScopeNodeKind, ScopeReadBudget, TargetReachabilityFreshness, TemporalActivityFreshness,
    WorkspaceManifestKind, WorkspaceManifestObservation,
};
pub use review_priority::{
    ReviewGap, ReviewGapKind, ReviewPriority, ReviewPriorityReport, ReviewPrioritySummary,
};
pub use route_assessment::{
    ReadmeRouteAssessment, ReadmeRoutingAssessment, RouteAssessment, RouteAssessmentError,
    RouteConflictAssessment, RouteEvidenceGroups, RouteFreshness, RoutePolicyBoundary,
    RoutePresenceAssessment, RouteSummaryProjection, TargetReachabilityAssessment,
};
pub use route_content::RouteContentAtom;
pub use route_content_contract::{
    route_content_contract, BilingualStructuralPair, ContentSlotAssessment, ContentSlotId,
    ContentSlotKind, ContentSlotSpec, MeaningAtomSet, PolicySensitivity, PolicySensitivityWire,
    RouteContentReport, ROUTE_CONTENT_CONTRACT,
};
pub use route_target::{classify_target_relation, RouteTargetRef, RouteTargetRole, TargetRelation};

pub const ANALYSIS_SCHEMA_VERSION: &str = "seiri.analysis.v1";
pub const CALIBRATION_SCHEMA_VERSION: &str = "seiri.calibration.v1";
pub const EVIDENCE_SCHEMA_VERSION: &str = "seiri.evidence.v1";
pub const TOOL_NAME: &str = "RepoSeiri";
pub const WORDING_LINT_SCHEMA_VERSION: &str = "seiri.wording-lint.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryAnalysis {
    pub schema_version: String,
    pub tool: String,
    pub repo_root: String,
    pub entry_count: usize,
    pub files: Vec<FileRecord>,
    pub important_files: Vec<ImportantFile>,
    pub analysis_configuration: AnalysisConfiguration,
    pub document_index: DocumentIndex,
    pub github_local_documents: GithubLocalDocuments,
    pub github_semantics: GithubSemanticsReport,
    pub readme_document: Option<DocumentScan>,
    pub readme_summary: Option<ReadmeSummary>,
    pub evidence_kernel: EvidenceKernel,
    pub coverage: CoverageIndex,
    pub route_content: RouteContentReport,
    pub facets: FacetReport,
    pub document_consistency: DocumentConsistencyReport,
    pub route_targets: Vec<RouteTargetRef>,
    pub remote_evidence: RemoteEvidenceReport,
    pub repository_scope: RepositoryScopeReport,
    pub freshness: FreshnessReport,
    pub pattern_matches: Vec<PatternMatch>,
    pub route_assessments: Vec<RouteAssessment>,
    pub missing_route_priority: MissingRoutePriorityReport,
    pub review_priority: ReviewPriorityReport,
    pub claims: Vec<ContentClaim>,
    pub baseline: Option<BaselineReport>,
    pub profile: Option<ProfileReport>,
    pub findings: Vec<Finding>,
}

impl RepositoryAnalysis {
    #[must_use]
    pub fn new(repo_root: impl Into<String>) -> Self {
        Self {
            schema_version: ANALYSIS_SCHEMA_VERSION.to_string(),
            tool: TOOL_NAME.to_string(),
            repo_root: repo_root.into(),
            entry_count: 0,
            files: Vec::new(),
            important_files: Vec::new(),
            analysis_configuration: AnalysisConfiguration::default(),
            document_index: DocumentIndex::default(),
            github_local_documents: GithubLocalDocuments::default(),
            github_semantics: GithubSemanticsReport::default(),
            readme_document: None,
            readme_summary: None,
            evidence_kernel: EvidenceKernel::default(),
            coverage: CoverageIndex::default(),
            route_content: RouteContentReport::default(),
            facets: FacetReport::default(),
            document_consistency: DocumentConsistencyReport::default(),
            route_targets: Vec::new(),
            remote_evidence: RemoteEvidenceReport::default(),
            repository_scope: RepositoryScopeReport::default(),
            freshness: FreshnessReport::default(),
            pattern_matches: Vec::new(),
            route_assessments: Vec::new(),
            missing_route_priority: MissingRoutePriorityReport::empty(),
            review_priority: ReviewPriorityReport::default(),
            claims: Vec::new(),
            baseline: None,
            profile: None,
            findings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileRecord {
    pub path: String,
    pub kind: FileKind,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileKind {
    File,
    Directory,
    Symlink,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportantFile {
    pub path: String,
    pub kind: ImportantFileKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportantFileKind {
    Readme,
    License,
    Contributing,
    Security,
    Support,
    IssueTemplate,
    IssueForm,
    PullRequestTemplate,
    Changelog,
    Codeowners,
    CargoToml,
    DocsDirectory,
    Workflow,
    DependencyBot,
    SecurityAutomation,
    Gitignore,
    Gitattributes,
    EditorConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadmeSummary {
    pub path: String,
    pub headings: Vec<MarkdownHeading>,
    pub links: Vec<MarkdownLink>,
    pub badges: Vec<MarkdownBadge>,
    pub route_candidates: Vec<RouteCandidate>,
    pub route_map: ReadmeRouteMap,
}

/// 1-based line/column plus byte offsets into the source document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SourceSpan {
    pub line: usize,
    pub column: usize,
    pub byte_start: usize,
    pub byte_end: usize,
}

impl SourceSpan {
    #[must_use]
    pub const fn new(line: usize, column: usize, byte_start: usize, byte_end: usize) -> Self {
        assert!(line > 0, "source span line must be 1-based");
        assert!(column > 0, "source span column must be 1-based");
        assert!(byte_start <= byte_end, "source span byte range is reversed");
        Self {
            line,
            column,
            byte_start,
            byte_end,
        }
    }

    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.line > 0 && self.column > 0 && self.byte_start <= self.byte_end
    }
}

impl<'de> Deserialize<'de> for SourceSpan {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct WireSpan {
            line: usize,
            column: usize,
            byte_start: usize,
            byte_end: usize,
        }

        let wire = WireSpan::deserialize(deserializer)?;
        if wire.line == 0 || wire.column == 0 || wire.byte_start > wire.byte_end {
            return Err(serde::de::Error::custom(
                "source span requires 1-based line/column and an ordered byte range",
            ));
        }
        Ok(Self::new(
            wire.line,
            wire.column,
            wire.byte_start,
            wire.byte_end,
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkdownHeading {
    pub level: u8,
    pub text: String,
    pub line: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkdownLink {
    pub text: String,
    pub target: String,
    pub line: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<SourceSpan>,
    pub route: Option<RouteKind>,
    #[serde(default)]
    pub kind: MarkdownLinkKind,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarkdownLinkKind {
    #[default]
    Inline,
    Reference,
    Autolink,
    Image,
    HtmlAnchor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkdownBadge {
    pub alt: String,
    pub target: String,
    pub line: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteCandidate {
    pub route: RouteKind,
    pub source: RouteSource,
    pub text: String,
    pub target: Option<String>,
    pub line: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadmeRouteMap {
    pub summary: ReadmeRouteMapSummary,
    pub entries: Vec<ReadmeRouteMapEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadmeRouteMapSummary {
    pub routes: usize,
    pub routed: usize,
    pub weak: usize,
    pub conflicting: usize,
    pub overloaded: usize,
    pub stale: usize,
    pub absent: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AggregateEstimateBasis {
    FixedAggregateCalibration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct AggregateRepositoryEstimate {
    pub estimated_repositories: u32,
    pub denominator: u32,
    pub rate_x1000: u16,
    pub basis: AggregateEstimateBasis,
}

impl AggregateRepositoryEstimate {
    #[must_use]
    pub fn fixed(estimated_repositories: u32, denominator: u32) -> Self {
        assert!(
            denominator > 0,
            "aggregate estimate denominator must be non-zero"
        );
        assert!(
            estimated_repositories <= denominator,
            "aggregate estimate cannot exceed its denominator"
        );
        let rate_x1000 =
            ((u64::from(estimated_repositories) * 1000) / u64::from(denominator)) as u16;
        Self {
            estimated_repositories,
            denominator,
            rate_x1000,
            basis: AggregateEstimateBasis::FixedAggregateCalibration,
        }
    }
}

impl<'de> Deserialize<'de> for AggregateRepositoryEstimate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct WireEstimate {
            estimated_repositories: u32,
            denominator: u32,
            rate_x1000: u16,
            basis: AggregateEstimateBasis,
        }

        let wire = WireEstimate::deserialize(deserializer)?;
        if wire.denominator == 0 {
            return Err(serde::de::Error::custom(
                "aggregate estimate denominator must be non-zero",
            ));
        }
        if wire.estimated_repositories > wire.denominator {
            return Err(serde::de::Error::custom(
                "aggregate estimate cannot exceed its denominator",
            ));
        }
        let expected_rate =
            ((u64::from(wire.estimated_repositories) * 1000) / u64::from(wire.denominator)) as u16;
        if wire.rate_x1000 != expected_rate {
            return Err(serde::de::Error::custom(
                "aggregate estimate rate does not match its count and denominator",
            ));
        }
        Ok(Self {
            estimated_repositories: wire.estimated_repositories,
            denominator: wire.denominator,
            rate_x1000: wire.rate_x1000,
            basis: wire.basis,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReadmeRouteMapEntry {
    pub route: RouteKind,
    pub assessment: ReadmeRouteAssessment,
    pub state: RouteState,
    #[serde(default)]
    pub gap_estimate: Option<AggregateRepositoryEstimate>,
    pub candidate_count: usize,
    pub heading_count: usize,
    pub link_count: usize,
    pub badge_count: usize,
    pub target_count: usize,
    pub stale_target_count: usize,
    pub conflicting_target_count: usize,
    pub evidence_lines: Vec<usize>,
    pub targets: Vec<ReadmeRouteTarget>,
    pub reason: String,
}

impl<'de> Deserialize<'de> for ReadmeRouteMapEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct WireEntry {
            route: RouteKind,
            assessment: ReadmeRouteAssessment,
            state: RouteState,
            #[serde(default)]
            gap_estimate: Option<AggregateRepositoryEstimate>,
            candidate_count: usize,
            heading_count: usize,
            link_count: usize,
            badge_count: usize,
            target_count: usize,
            stale_target_count: usize,
            conflicting_target_count: usize,
            evidence_lines: Vec<usize>,
            targets: Vec<ReadmeRouteTarget>,
            reason: String,
        }

        let wire = WireEntry::deserialize(deserializer)?;
        let derived = ReadmeRouteAssessment::from_observations(
            wire.candidate_count,
            wire.heading_count,
            wire.link_count,
            wire.badge_count,
            wire.target_count,
            &wire.targets,
        )
        .map_err(serde::de::Error::custom)?;
        let projected_state = derived.summary_state(wire.route);
        if wire.assessment != derived {
            return Err(serde::de::Error::custom(
                "README route assessment does not match its observations",
            ));
        }
        if wire.state != projected_state {
            return Err(serde::de::Error::custom(
                "README route state does not match its deterministic assessment projection",
            ));
        }
        if wire.stale_target_count != derived.target_reachability.repository_local_missing
            || wire.conflicting_target_count != derived.conflict.shared_target_count
        {
            return Err(serde::de::Error::custom(
                "README route counts do not match its assessment",
            ));
        }

        let _summary_reason = wire.reason;
        Ok(Self {
            route: wire.route,
            assessment: derived,
            state: projected_state,
            gap_estimate: wire.gap_estimate,
            candidate_count: wire.candidate_count,
            heading_count: wire.heading_count,
            link_count: wire.link_count,
            badge_count: wire.badge_count,
            target_count: wire.target_count,
            stale_target_count: derived.target_reachability.repository_local_missing,
            conflicting_target_count: derived.conflict.shared_target_count,
            evidence_lines: wire.evidence_lines,
            targets: wire.targets,
            reason: derived.summary_reason(wire.route).to_string(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadmeRouteTarget {
    pub target: String,
    pub line: usize,
    pub source: RouteSource,
    pub status: ReadmeRouteTargetStatus,
    pub routes: Vec<RouteKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadmeRouteTargetStatus {
    LocalPresent,
    LocalMissing,
    External,
    Anchor,
    Mail,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteSource {
    Heading,
    Link,
    Badge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteKind {
    Identity,
    Docs,
    Quickstart,
    Support,
    Intake,
    Contributing,
    Security,
    Release,
    Lifecycle,
    Governance,
    License,
    Automation,
    Ownership,
    Hygiene,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PatternGroup {
    #[serde(rename = "IDN")]
    Idn,
    #[serde(rename = "DOC")]
    Doc,
    #[serde(rename = "QST")]
    Qst,
    #[serde(rename = "SUP")]
    Sup,
    #[serde(rename = "SEC")]
    Sec,
    #[serde(rename = "CTR")]
    Ctr,
    #[serde(rename = "INT")]
    Int,
    #[serde(rename = "AUT")]
    Aut,
    #[serde(rename = "REL")]
    Rel,
    #[serde(rename = "OWN")]
    Own,
    #[serde(rename = "GOV")]
    Gov,
    #[serde(rename = "HYG")]
    Hyg,
    #[serde(rename = "LIF")]
    Lif,
}

impl PatternGroup {
    pub const ALL: [Self; 13] = [
        Self::Idn,
        Self::Doc,
        Self::Qst,
        Self::Sup,
        Self::Sec,
        Self::Ctr,
        Self::Int,
        Self::Aut,
        Self::Rel,
        Self::Own,
        Self::Gov,
        Self::Hyg,
        Self::Lif,
    ];

    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::Idn => "IDN",
            Self::Doc => "DOC",
            Self::Qst => "QST",
            Self::Sup => "SUP",
            Self::Sec => "SEC",
            Self::Ctr => "CTR",
            Self::Int => "INT",
            Self::Aut => "AUT",
            Self::Rel => "REL",
            Self::Own => "OWN",
            Self::Gov => "GOV",
            Self::Hyg => "HYG",
            Self::Lif => "LIF",
        }
    }
}

impl std::fmt::Display for PatternGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}

pub type ClaimId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceConfidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationEvidenceKind {
    FilePresent,
    ImportantFile,
    ReadmePresent,
    ReadmeMissing,
    MarkdownHeading,
    MarkdownLink,
    MarkdownBadge,
    RouteCandidate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatternMatch {
    pub id: String,
    pub pattern_id: String,
    pub title: String,
    pub route: Option<RouteKind>,
    pub outcome: PatternOutcome,
    pub evidence_ids: Vec<EvidenceId>,
    pub basis: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternOutcome {
    Present,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissingRoutePriorityReport {
    pub summary: MissingRoutePrioritySummary,
    pub priorities: Vec<MissingRoutePriority>,
    pub co_occurrence_gaps: Vec<RouteCoOccurrenceGap>,
    pub boundary: String,
}

impl MissingRoutePriorityReport {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            summary: MissingRoutePrioritySummary {
                candidates: 0,
                co_occurrence_gaps: 0,
                top_route: None,
                top_priority_x100: None,
                safe_gated: 0,
                guarded_gated: 0,
                manual_gated: 0,
            },
            priorities: Vec::new(),
            co_occurrence_gaps: Vec::new(),
            boundary: "Missing route priority is a deterministic routing hint from repository observations and route co-occurrence rules. Standard audit has no aggregate prior; explicit local calibration remains redacted. This is not a popularity, trust, security, quality, or policy guarantee.".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissingRoutePrioritySummary {
    pub candidates: usize,
    pub co_occurrence_gaps: usize,
    pub top_route: Option<RouteKind>,
    pub top_priority_x100: Option<u8>,
    pub safe_gated: usize,
    pub guarded_gated: usize,
    pub manual_gated: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MissingRoutePriority {
    pub rank: usize,
    pub route: RouteKind,
    pub state: RouteState,
    pub gate: GateKind,
    pub severity: Severity,
    pub priority: ProfilePriority,
    pub priority_score_x100: u8,
    #[serde(default)]
    pub calibration_estimate: Option<AggregateRepositoryEstimate>,
    pub baseline_pattern_ids: Vec<String>,
    pub candidate_pattern_ids: Vec<String>,
    pub co_occurrence_gap_ids: Vec<String>,
    pub evidence_ids: Vec<EvidenceId>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteCoOccurrenceGap {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub calibration_estimate: Option<AggregateRepositoryEstimate>,
    pub support_x1000: u16,
    #[serde(skip, default)]
    pub rank_weight_x100: u8,
    #[serde(skip, default)]
    pub calibration_prior: CalibrationPriorState,
    pub gate: GateKind,
    pub priority: ProfilePriority,
    pub present_routes: Vec<RouteKind>,
    pub missing_routes: Vec<RouteKind>,
    pub present_signals: Vec<String>,
    pub missing_signals: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteState {
    Absent,
    Implicit,
    Weak,
    Routed,
    Structured,
    Verified,
    Inherited,
    Overridden,
    Conflicting,
    Overloaded,
    Stale,
    UnsafeToInvent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaselineReport {
    pub profile: BaselineProfile,
    pub summary: BaselineSummary,
    pub rules: Vec<BaselineRuleResult>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BaselineProfile {
    Common,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaselineSummary {
    pub required_present: usize,
    pub required_missing: usize,
    pub optional_present: usize,
    pub optional_missing: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaselineRuleResult {
    pub rule_id: String,
    pub pattern_id: String,
    pub title: String,
    pub route: Option<RouteKind>,
    pub requirement: BaselineRequirement,
    pub status: BaselineStatus,
    pub severity: Severity,
    pub evidence_ids: Vec<EvidenceId>,
    pub finding_id: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BaselineRequirement {
    Required,
    Optional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BaselineStatus {
    Present,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileReport {
    pub profile: ProfileKind,
    pub score: ProfileScoreView,
    pub branch_summary: ProfileBranchSummary,
    pub branches: Vec<ProfileBranch>,
    pub rules: Vec<ProfileRuleResult>,
    pub recommendations: Vec<ProfileRecommendation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileKind {
    Common,
    Library,
    Cli,
    Infra,
    Product,
    Runtime,
    Docs,
    Tutorial,
    Ml,
    Research,
    Template,
}

impl std::fmt::Display for ProfileKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Common => "common",
            Self::Library => "library",
            Self::Cli => "cli",
            Self::Infra => "infra",
            Self::Product => "product",
            Self::Runtime => "runtime",
            Self::Docs => "docs",
            Self::Tutorial => "tutorial",
            Self::Ml => "ml",
            Self::Research => "research",
            Self::Template => "template",
        };
        f.write_str(value)
    }
}

impl ProfileKind {
    pub const ALL: [Self; 11] = [
        Self::Common,
        Self::Library,
        Self::Cli,
        Self::Infra,
        Self::Product,
        Self::Runtime,
        Self::Docs,
        Self::Tutorial,
        Self::Ml,
        Self::Research,
        Self::Template,
    ];
}

impl std::str::FromStr for ProfileKind {
    type Err = ProfileParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "common" => Ok(Self::Common),
            "library" | "lib" => Ok(Self::Library),
            "cli" | "command-line" | "command_line" => Ok(Self::Cli),
            "infra" | "infrastructure" => Ok(Self::Infra),
            "product" | "app" | "application" => Ok(Self::Product),
            "runtime" | "compiler" | "toolchain" => Ok(Self::Runtime),
            "docs" | "documentation" => Ok(Self::Docs),
            "tutorial" => Ok(Self::Tutorial),
            "ml" | "machine-learning" | "machine_learning" | "data" => Ok(Self::Ml),
            "research" => Ok(Self::Research),
            "template" => Ok(Self::Template),
            _ => Err(ProfileParseError {
                value: value.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileParseError {
    pub value: String,
}

impl std::fmt::Display for ProfileParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown profile '{}'", self.value)
    }
}

impl std::error::Error for ProfileParseError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileBranchSummary {
    pub selected_profile: ProfileKind,
    pub top_profile: Option<ProfileKind>,
    pub top_rank_score_x100: Option<u8>,
    pub emitted_profiles: usize,
    pub ambiguous: bool,
    pub boundary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileBranch {
    pub rank: usize,
    pub profile: ProfileKind,
    pub semantics: ProfileBranchSemantics,
    pub matched_signals: Vec<String>,
    pub missing_signals: Vec<String>,
    pub rationale: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileScoreView {
    #[serde(default)]
    pub evidence_basis: ProfileEvidenceBasis,
    #[serde(default)]
    pub weight_basis: ProfileWeightBasis,
    pub earned_weight: u32,
    pub total_weight: u32,
    pub score_x100: u8,
    pub present_rules: usize,
    pub missing_rules: usize,
    pub note: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProfileEvidenceBasis {
    #[default]
    RepositoryEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProfileWeightBasis {
    #[default]
    StaticProfileRegistry,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileRuleResult {
    pub rule_id: String,
    pub profile: ProfileKind,
    pub pattern_id: String,
    pub title: String,
    pub route: Option<RouteKind>,
    pub status: BaselineStatus,
    pub weight: u32,
    pub priority: ProfilePriority,
    pub evidence_ids: Vec<EvidenceId>,
    pub finding_id: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfilePriority {
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileRecommendation {
    pub rank: usize,
    pub finding_id: Option<String>,
    pub pattern_id: String,
    pub title: String,
    pub gate: GateKind,
    pub severity: Severity,
    pub priority: ProfilePriority,
    pub weight: u32,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceSchemaVersion {
    pub schema_version: String,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BenchmarkDataset {
    pub schema_version: String,
    pub dataset_id: String,
    pub name: String,
    pub collected_at: String,
    #[serde(default)]
    pub calibration_sources: Vec<CalibrationSource>,
    #[serde(default)]
    pub extraction_conditions: Vec<String>,
    #[serde(default)]
    pub limitations: Vec<String>,
    pub evidence_schema: EvidenceSchemaVersion,
    #[serde(default)]
    pub records: Vec<BenchmarkRepoRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BenchmarkRepoRecord {
    pub repo_id: String,
    #[serde(default)]
    pub owner: Option<String>,
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub stars: u64,
    #[serde(default)]
    pub age_days: u32,
    #[serde(default)]
    pub primary_language: Option<String>,
    #[serde(default)]
    pub topics: Vec<String>,
    #[serde(default)]
    pub activity: BenchmarkActivity,
    #[serde(default)]
    pub metadata_source: String,
    #[serde(default)]
    pub profile_hint: Option<ProfileKind>,
    #[serde(default)]
    pub observed_patterns: Vec<ObservedPattern>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BenchmarkActivity {
    #[serde(default)]
    pub pushed_within_days: Option<u32>,
    #[serde(default)]
    pub default_branch_commits: Option<u64>,
    #[serde(default)]
    pub open_issues: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservedPattern {
    #[serde(default)]
    pub pattern_id: Option<String>,
    pub raw_label: String,
    #[serde(default)]
    pub evidence_kind: Option<CalibrationEvidenceKind>,
    #[serde(default)]
    pub route: Option<RouteKind>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default = "default_observation_count")]
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalibrationRun {
    pub schema_version: String,
    pub run_id: String,
    pub dataset_id: String,
    #[serde(default)]
    pub pattern_pack: Option<CalibrationPatternPack>,
    pub sources: Vec<CalibrationSource>,
    pub summary: CalibrationSummary,
    pub stats: Vec<PatternStats>,
    pub route_requirements: Vec<RouteRequirement>,
    pub profile_branches: Vec<ProfileBranch>,
    pub pending_patterns: Vec<PendingPatternCandidate>,
    pub weight_suggestions: Vec<WeightSuggestion>,
    pub resource_trace: CalibrationResourceTrace,
    pub claim_boundary: ClaimBoundary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalibrationPatternPack {
    pub id: String,
    pub version: String,
    pub condition: String,
    pub registry_fingerprint: String,
    pub eligible_records: usize,
    pub excluded_records: usize,
}

impl CalibrationRun {
    #[must_use]
    pub fn source_visibility_summary(&self) -> CalibrationSourceVisibilitySummary {
        CalibrationSourceVisibilitySummary::from_sources(&self.sources)
    }

    #[must_use]
    pub fn redacted_for_public_output(&self) -> Self {
        let mut public_run = self.clone();
        let mut source_id_map = BTreeMap::<String, String>::new();

        public_run.sources = self
            .sources
            .iter()
            .enumerate()
            .map(|(index, source)| match source.visibility {
                CalibrationSourceVisibility::Public => source.clone(),
                CalibrationSourceVisibility::LocalOnly | CalibrationSourceVisibility::Redacted => {
                    let public_id = redacted_calibration_source_id(index + 1);
                    source_id_map.insert(source.id.clone(), public_id.clone());
                    redacted_calibration_source(source, public_id)
                }
            })
            .collect();

        if !source_id_map.is_empty() {
            public_run.dataset_id = "redacted-calibration-dataset".to_string();
            public_run.summary = CalibrationSummary {
                records: 0,
                sources: public_run.sources.len(),
                known_pattern_stats: 0,
                route_requirements: 0,
                profile_branches: 0,
                pending_patterns: 0,
                weight_suggestions: 0,
            };
            public_run.stats.clear();
            public_run.route_requirements.clear();
            public_run.profile_branches.clear();
            public_run.pending_patterns.clear();
            public_run.weight_suggestions.clear();
            let aggregation_mode = public_run.resource_trace.aggregation_mode;
            let record_identity = public_run.resource_trace.record_identity;
            public_run.resource_trace = CalibrationResourceTrace {
                aggregation_mode,
                record_identity,
                ..CalibrationResourceTrace::default()
            };
            if let Some(pack) = &mut public_run.pattern_pack {
                pack.eligible_records = 0;
                pack.excluded_records = 0;
            }
        }
        public_run.redact_source_references(&source_id_map);
        public_run
    }

    fn redact_source_references(&mut self, source_id_map: &BTreeMap<String, String>) {
        if source_id_map.is_empty() {
            return;
        }

        for stat in &mut self.stats {
            redact_source_ids(&mut stat.source_ids, source_id_map);
        }
        for requirement in &mut self.route_requirements {
            redact_source_ids(&mut requirement.source_ids, source_id_map);
        }
        for candidate in &mut self.pending_patterns {
            if redact_source_ids(&mut candidate.source_ids, source_id_map) {
                candidate.raw_label = "redacted local-only pattern candidate".to_string();
                candidate.example_locations.clear();
            }
        }
        for suggestion in &mut self.weight_suggestions {
            redact_source_ids(&mut suggestion.source_ids, source_id_map);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationAggregationMode {
    #[default]
    MaterializedDataset,
    StreamingJsonl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationRecordIdentity {
    #[default]
    RepositoryIdDeduplicated,
    OneNonemptyJsonlLinePerRepository,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalibrationReplayDigest(u64);

impl CalibrationReplayDigest {
    #[must_use]
    pub const fn from_u64(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for CalibrationReplayDigest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "fnv1a64:{:016x}", self.0)
    }
}

impl Serialize for CalibrationReplayDigest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for CalibrationReplayDigest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = String::deserialize(deserializer)?;
        let hex = wire
            .strip_prefix("fnv1a64:")
            .filter(|hex| hex.len() == 16)
            .ok_or_else(|| {
                serde::de::Error::custom(
                    "calibration replay digest must use fnv1a64 plus 16 lowercase hex digits",
                )
            })?;
        if !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(serde::de::Error::custom(
                "calibration replay digest contains invalid hex digits",
            ));
        }
        let value = u64::from_str_radix(hex, 16).map_err(serde::de::Error::custom)?;
        Ok(Self(value))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CalibrationResourceTrace {
    pub aggregation_mode: CalibrationAggregationMode,
    pub record_identity: CalibrationRecordIdentity,
    pub records_seen: usize,
    pub max_buffered_line_bytes: usize,
    pub max_patterns_per_record: usize,
    pub known_pattern_slots: usize,
    pub route_slots: usize,
    pub profile_slots: usize,
    pub co_occurrence_slots: usize,
    pub pending_pattern_slots: usize,
    pub metadata_source_slots: usize,
    pub retained_records: usize,
    pub retained_repository_id_entries: usize,
    pub per_pattern_repository_sets: usize,
    #[serde(default)]
    pub replay_digest: Option<CalibrationReplayDigest>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalibrationSummary {
    pub records: usize,
    pub sources: usize,
    pub known_pattern_stats: usize,
    pub route_requirements: usize,
    pub profile_branches: usize,
    pub pending_patterns: usize,
    pub weight_suggestions: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalibrationSourceVisibilitySummary {
    pub total: usize,
    pub public_sources: usize,
    pub local_only_sources: usize,
    pub redacted_sources: usize,
    pub pending_review: usize,
    pub adopted: usize,
    pub deferred: usize,
    pub rejected: usize,
}

impl CalibrationSourceVisibilitySummary {
    #[must_use]
    pub fn from_sources(sources: &[CalibrationSource]) -> Self {
        let mut summary = Self {
            total: sources.len(),
            ..Self::default()
        };
        for source in sources {
            match source.visibility {
                CalibrationSourceVisibility::Public => summary.public_sources += 1,
                CalibrationSourceVisibility::LocalOnly => summary.local_only_sources += 1,
                CalibrationSourceVisibility::Redacted => summary.redacted_sources += 1,
            }
            match source.review_status {
                CalibrationReviewStatus::PendingReview => summary.pending_review += 1,
                CalibrationReviewStatus::Adopted => summary.adopted += 1,
                CalibrationReviewStatus::Deferred => summary.deferred += 1,
                CalibrationReviewStatus::Rejected => summary.rejected += 1,
            }
        }
        summary
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalibrationSource {
    pub id: String,
    pub kind: CalibrationSourceKind,
    #[serde(default)]
    pub visibility: CalibrationSourceVisibility,
    pub label: String,
    pub collected_at: String,
    pub records: usize,
    pub scale: CalibrationScale,
    #[serde(default)]
    pub metadata_sources: Vec<String>,
    #[serde(default)]
    pub extraction_conditions: Vec<String>,
    #[serde(default)]
    pub limitations: Vec<String>,
    #[serde(default)]
    pub evidence_schema: Option<EvidenceSchemaVersion>,
    pub review_status: CalibrationReviewStatus,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationSourceVisibility {
    Public,
    #[default]
    LocalOnly,
    Redacted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationSourceKind {
    BenchmarkDataset,
    JsonlRecords,
    AggregateAnalysis,
    Fixture,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationScale {
    Tiny,
    Small,
    HundredK,
    Million,
    Custom,
}

fn redacted_calibration_source_id(index: usize) -> String {
    format!("redacted-calibration-source-{index:04}")
}

fn redacted_calibration_source(source: &CalibrationSource, public_id: String) -> CalibrationSource {
    CalibrationSource {
        id: public_id,
        kind: source.kind,
        visibility: CalibrationSourceVisibility::Redacted,
        label: "redacted local-only calibration source".to_string(),
        collected_at: "redacted".to_string(),
        records: 0,
        scale: CalibrationScale::Custom,
        metadata_sources: vec!["redacted".to_string()],
        extraction_conditions: vec![
            "Local-only source details are withheld from public output.".to_string(),
        ],
        limitations: vec![
            "Source path, body text, source-specific notes, and aggregate values are redacted; only review status remains.".to_string(),
        ],
        evidence_schema: None,
        review_status: source.review_status,
    }
}

fn redact_source_ids(source_ids: &mut [String], source_id_map: &BTreeMap<String, String>) -> bool {
    let mut redacted = false;
    for source_id in source_ids {
        if let Some(public_id) = source_id_map.get(source_id) {
            *source_id = public_id.clone();
            redacted = true;
        }
    }
    redacted
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatternStats {
    pub pattern_id: String,
    pub route: Option<RouteKind>,
    pub repositories: usize,
    pub observations: u64,
    pub frequency_x1000: u16,
    pub source_ids: Vec<String>,
    pub profile_correlations: Vec<ProfilePatternCorrelation>,
    pub co_occurrences: Vec<PatternCoOccurrence>,
    pub confidence: CalibrationConfidence,
    pub confidence_note: String,
    pub review_status: CalibrationReviewStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfilePatternCorrelation {
    pub profile: ProfileKind,
    pub repositories: usize,
    pub frequency_x1000: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatternCoOccurrence {
    pub pattern_id: String,
    pub repositories: usize,
    pub co_frequency_x1000: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingPatternCandidate {
    pub id: String,
    pub raw_label: String,
    pub observed_repositories: usize,
    pub observations: u64,
    pub source_ids: Vec<String>,
    pub example_locations: Vec<String>,
    pub review_status: CalibrationReviewStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteRequirement {
    pub id: String,
    pub route: RouteKind,
    pub supporting_repositories: usize,
    pub observations: u64,
    pub frequency_x1000: u16,
    pub suggested_requirement: BaselineRequirement,
    pub priority: ProfilePriority,
    pub source_ids: Vec<String>,
    pub confidence: CalibrationConfidence,
    pub review_status: CalibrationReviewStatus,
    pub rationale: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeightSuggestion {
    pub id: String,
    pub profile: ProfileKind,
    pub pattern_id: String,
    pub route: Option<RouteKind>,
    pub current_weight: Option<u32>,
    pub suggested_weight: u32,
    pub suggested_delta: i32,
    pub priority: ProfilePriority,
    pub support_repositories: usize,
    pub frequency_x1000: u16,
    pub source_ids: Vec<String>,
    pub confidence: CalibrationConfidence,
    pub review_status: CalibrationReviewStatus,
    pub rationale: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimBoundary {
    pub summary: String,
    pub review_required: bool,
    pub runtime_rule_adoption_allowed: bool,
    pub automatic_weight_adoption_allowed: bool,
    pub guarantee_allowed: bool,
    pub blocked_claims: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentClaim {
    pub id: ClaimId,
    pub route: RouteKind,
    pub state: RouteState,
    pub strength: ClaimStrength,
    pub evidence_ids: Vec<EvidenceId>,
    pub allowed_meanings: Vec<MeaningAtom>,
    pub boundaries: Vec<ClaimBoundaryKind>,
}

impl ContentClaim {
    #[must_use]
    pub fn new(
        index: usize,
        route: RouteKind,
        state: RouteState,
        strength: ClaimStrength,
        evidence_ids: Vec<EvidenceId>,
        allowed_meanings: Vec<MeaningAtom>,
        boundaries: Vec<ClaimBoundaryKind>,
    ) -> Self {
        Self {
            id: stable_claim_id(index),
            route,
            state,
            strength,
            evidence_ids,
            allowed_meanings,
            boundaries,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimStrength {
    Observed,
    Inferred,
    Suggested,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeaningAtom {
    RouteObserved,
    RouteMissing,
    RepositoryLocalTargetPresent,
    RepositoryLocalTargetMissing,
    ReadmeMentionsRoute,
    StructuredFilePresent,
    AutomationConfigured,
    HumanReviewRequired,
    PatchPreviewOnly,
    CalibrationCandidate,
    ContentSlotObserved,
    ExpectedOutputDocumented,
    StructuralParallelCandidate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimBoundaryKind {
    NotPopularityGuarantee,
    NotTrustGuarantee,
    NotSecurityGuarantee,
    NotQualityGuarantee,
    NotLegalFitnessGuarantee,
    NotLegalAdvice,
    NotMaintenanceGuarantee,
    NotRuntimeVerification,
    NotPublicationReadiness,
    NotOwnerApproval,
    NotProductionReadiness,
    NotAutomaticPolicyAdoption,
    NotAutomaticWeightAdoption,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WordingLintReport {
    pub schema_version: String,
    pub tool: String,
    pub repo_root: String,
    pub summary: WordingLintSummary,
    pub findings: Vec<WordingLintFinding>,
    pub boundary: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WordingLintSummary {
    pub files_scanned: usize,
    pub generated_surfaces: usize,
    pub findings: usize,
    pub suppressed_boundary_exceptions: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WordingLintFinding {
    pub id: String,
    pub source: WordingLintSourceKind,
    pub path: String,
    pub line: usize,
    pub column: usize,
    pub byte_start: usize,
    pub byte_end: usize,
    pub matched: String,
    pub rule: WordingRuleKind,
    pub boundary: ClaimBoundaryKind,
    pub replacement_hint: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WordingLintSourceKind {
    RepositoryFile,
    GeneratedReport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WordingRuleKind {
    GenericGuarantee,
    PopularityGuarantee,
    TrustGuarantee,
    SecurityGuarantee,
    QualityGuarantee,
    LegalFitnessGuarantee,
    LegalAdvice,
    MaintenanceGuarantee,
    RuntimeVerification,
    PublicationReadiness,
    ProductionReadiness,
    AutomaticPolicyAdoption,
    AutomaticWeightAdoption,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WordingBoundaryException {
    NegatedBoundaryStatement,
    TypedClaimBoundary,
}

#[derive(Debug, Clone, Copy)]
pub struct ClaimRefIndex<'a> {
    claims: &'a [ContentClaim],
}

impl<'a> ClaimRefIndex<'a> {
    #[must_use]
    pub fn new(claims: &'a [ContentClaim]) -> Self {
        Self { claims }
    }

    #[must_use]
    pub fn strength_count(self, strength: ClaimStrength) -> usize {
        self.claims
            .iter()
            .filter(|claim| claim.strength == strength)
            .count()
    }

    #[must_use]
    pub fn claim_ids_for_route_state(self, route: RouteKind, state: RouteState) -> Vec<ClaimId> {
        self.claims
            .iter()
            .filter(|claim| claim.route == route && claim.state == state)
            .map(|claim| claim.id.clone())
            .collect()
    }

    #[must_use]
    pub fn claim_ids_for_route(self, route: RouteKind) -> Vec<ClaimId> {
        self.claims
            .iter()
            .filter(|claim| claim.route == route)
            .map(|claim| claim.id.clone())
            .collect()
    }

    #[must_use]
    pub fn boundary_kinds_for_route_state(
        self,
        route: RouteKind,
        state: RouteState,
    ) -> Vec<ClaimBoundaryKind> {
        self.claims
            .iter()
            .filter(|claim| claim.route == route && claim.state == state)
            .flat_map(|claim| claim.boundaries.iter().copied())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    #[must_use]
    pub fn boundary_kinds_for_route(self, route: RouteKind) -> Vec<ClaimBoundaryKind> {
        self.claims
            .iter()
            .filter(|claim| claim.route == route)
            .flat_map(|claim| claim.boundaries.iter().copied())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    #[must_use]
    pub fn boundary_kinds_for_claim_ids(self, claim_ids: &[ClaimId]) -> Vec<ClaimBoundaryKind> {
        self.claims
            .iter()
            .filter(|claim| claim_ids.contains(&claim.id))
            .flat_map(|claim| claim.boundaries.iter().copied())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    #[must_use]
    pub fn boundary_kinds(self) -> Vec<ClaimBoundaryKind> {
        self.claims
            .iter()
            .flat_map(|claim| claim.boundaries.iter().copied())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RouteMeaningRule {
    pub route: RouteKind,
    pub state: RouteState,
    pub indicates: &'static [MeaningAtom],
    pub does_not_indicate: &'static [ClaimBoundaryKind],
}

pub const ROUTE_MEANING_ROUTES: &[RouteKind] = &[
    RouteKind::Identity,
    RouteKind::Docs,
    RouteKind::Quickstart,
    RouteKind::Support,
    RouteKind::Intake,
    RouteKind::Contributing,
    RouteKind::Security,
    RouteKind::Release,
    RouteKind::Lifecycle,
    RouteKind::Governance,
    RouteKind::License,
    RouteKind::Automation,
    RouteKind::Ownership,
    RouteKind::Hygiene,
    RouteKind::Unknown,
];

pub const ROUTE_MEANING_STATES: &[RouteState] = &[
    RouteState::Absent,
    RouteState::Implicit,
    RouteState::Weak,
    RouteState::Routed,
    RouteState::Structured,
    RouteState::Verified,
    RouteState::Inherited,
    RouteState::Overridden,
    RouteState::Conflicting,
    RouteState::Overloaded,
    RouteState::Stale,
    RouteState::UnsafeToInvent,
];

const MEANING_ABSENT: &[MeaningAtom] = &[MeaningAtom::RouteMissing];
const MEANING_IMPLICIT: &[MeaningAtom] = &[MeaningAtom::ReadmeMentionsRoute];
const MEANING_WEAK: &[MeaningAtom] = &[
    MeaningAtom::ReadmeMentionsRoute,
    MeaningAtom::HumanReviewRequired,
];
const MEANING_ROUTED: &[MeaningAtom] =
    &[MeaningAtom::ReadmeMentionsRoute, MeaningAtom::RouteObserved];
const MEANING_STRUCTURED: &[MeaningAtom] = &[
    MeaningAtom::StructuredFilePresent,
    MeaningAtom::RouteObserved,
];
const MEANING_VERIFIED: &[MeaningAtom] = &[
    MeaningAtom::RouteObserved,
    MeaningAtom::RepositoryLocalTargetPresent,
];
const MEANING_INHERITED: &[MeaningAtom] =
    &[MeaningAtom::RouteObserved, MeaningAtom::HumanReviewRequired];
const MEANING_CONFLICTING: &[MeaningAtom] =
    &[MeaningAtom::RouteObserved, MeaningAtom::HumanReviewRequired];
const MEANING_OVERLOADED: &[MeaningAtom] = &[
    MeaningAtom::ReadmeMentionsRoute,
    MeaningAtom::HumanReviewRequired,
];
const MEANING_STALE: &[MeaningAtom] = &[
    MeaningAtom::ReadmeMentionsRoute,
    MeaningAtom::RepositoryLocalTargetMissing,
    MeaningAtom::HumanReviewRequired,
];
const MEANING_UNSAFE_TO_INVENT: &[MeaningAtom] = &[
    MeaningAtom::RouteMissing,
    MeaningAtom::HumanReviewRequired,
    MeaningAtom::PatchPreviewOnly,
];

const ROUTE_NON_CLAIM_BOUNDARIES: &[ClaimBoundaryKind] = &[
    ClaimBoundaryKind::NotPopularityGuarantee,
    ClaimBoundaryKind::NotTrustGuarantee,
    ClaimBoundaryKind::NotSecurityGuarantee,
    ClaimBoundaryKind::NotQualityGuarantee,
    ClaimBoundaryKind::NotLegalFitnessGuarantee,
    ClaimBoundaryKind::NotLegalAdvice,
    ClaimBoundaryKind::NotMaintenanceGuarantee,
    ClaimBoundaryKind::NotRuntimeVerification,
    ClaimBoundaryKind::NotPublicationReadiness,
    ClaimBoundaryKind::NotOwnerApproval,
    ClaimBoundaryKind::NotProductionReadiness,
    ClaimBoundaryKind::NotAutomaticPolicyAdoption,
    ClaimBoundaryKind::NotAutomaticWeightAdoption,
];

#[must_use]
pub fn route_meaning_rule(route: RouteKind, state: RouteState) -> RouteMeaningRule {
    RouteMeaningRule {
        route,
        state,
        indicates: route_state_indicates(state),
        does_not_indicate: route_state_does_not_indicate(route, state),
    }
}

pub fn route_meaning_rules() -> impl Iterator<Item = RouteMeaningRule> {
    ROUTE_MEANING_ROUTES.iter().copied().flat_map(|route| {
        ROUTE_MEANING_STATES
            .iter()
            .copied()
            .map(move |state| route_meaning_rule(route, state))
    })
}

#[must_use]
pub fn route_state_indicates(state: RouteState) -> &'static [MeaningAtom] {
    match state {
        RouteState::Absent => MEANING_ABSENT,
        RouteState::Implicit => MEANING_IMPLICIT,
        RouteState::Weak => MEANING_WEAK,
        RouteState::Routed => MEANING_ROUTED,
        RouteState::Structured => MEANING_STRUCTURED,
        RouteState::Verified => MEANING_VERIFIED,
        RouteState::Inherited | RouteState::Overridden => MEANING_INHERITED,
        RouteState::Conflicting => MEANING_CONFLICTING,
        RouteState::Overloaded => MEANING_OVERLOADED,
        RouteState::Stale => MEANING_STALE,
        RouteState::UnsafeToInvent => MEANING_UNSAFE_TO_INVENT,
    }
}

#[must_use]
pub fn route_state_does_not_indicate(
    route: RouteKind,
    state: RouteState,
) -> &'static [ClaimBoundaryKind] {
    let _indicates = route_state_indicates(state);
    match route {
        RouteKind::Identity
        | RouteKind::Docs
        | RouteKind::Quickstart
        | RouteKind::Support
        | RouteKind::Intake
        | RouteKind::Contributing
        | RouteKind::Security
        | RouteKind::Release
        | RouteKind::Lifecycle
        | RouteKind::Governance
        | RouteKind::License
        | RouteKind::Automation
        | RouteKind::Ownership
        | RouteKind::Hygiene
        | RouteKind::Unknown => ROUTE_NON_CLAIM_BOUNDARIES,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationConfidence {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationReviewStatus {
    PendingReview,
    Adopted,
    Deferred,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub severity: Severity,
    pub title: String,
    pub message: String,
    pub evidence_ids: Vec<EvidenceId>,
    pub recommendation: Option<Recommendation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recommendation {
    pub id: String,
    pub gate: GateKind,
    pub title: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateKind {
    Safe,
    Guarded,
    Manual,
}

#[must_use]
pub fn stable_id(prefix: &str, index: usize) -> String {
    format!("{prefix}-{index:04}")
}

#[must_use]
pub fn stable_claim_id(index: usize) -> ClaimId {
    stable_id("claim", index)
}

fn default_observation_count() -> u32 {
    1
}
pub use audit_delta::{
    AddExistingRouteLink, AnalysisBudgetConfiguration, AnalysisConfiguration, AnalysisVisibility,
    ArtifactDelta, AuditDeltaReport, AuditSnapshotDigest, DeltaCompatibility, DeltaState,
    DeltaUnknownReason, Digest32, ExistingTargetId, ImprovementCandidate, PatchHold,
    PatchHoldReason, PatchPlan, PortableAuditSnapshot, PortableConflictRecord,
    PortableContentSlotRecord, PortableCoverageRecord, PortableDocumentRecord, PortableFacetRecord,
    PortableObligationRecord, PortableObservationState, PortableRouteRecord, RegressionCandidate,
    RouteDelta, AUDIT_DELTA_SCHEMA_VERSION, PATCH_PLAN_SCHEMA_VERSION,
    PORTABLE_AUDIT_SCHEMA_VERSION,
};
