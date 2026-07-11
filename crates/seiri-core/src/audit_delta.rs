use crate::{
    AnalysisScope, ContentSlotId, CoverageStatus, DocumentId, EvidenceId, ProfileKind,
    RepositoryFacet, RouteFreshness, RouteKind, RoutePolicyBoundary, RouteTargetRole,
    ScopeReadBudget,
};
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{Display, Formatter};

pub const PORTABLE_AUDIT_SCHEMA_VERSION: &str = "seiri.portable-audit.v1";
pub const AUDIT_DELTA_SCHEMA_VERSION: &str = "seiri.audit-delta.v1";
pub const PATCH_PLAN_SCHEMA_VERSION: &str = "seiri.patch-plan.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Digest32([u8; 32]);

impl Digest32 {
    #[must_use]
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }
}

impl Display for Digest32 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("sha256:")?;
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl Serialize for Digest32 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Digest32 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        let hex = value
            .strip_prefix("sha256:")
            .filter(|hex| hex.len() == 64)
            .ok_or_else(|| {
                D::Error::custom("digest must use sha256 plus 64 lowercase hex digits")
            })?;
        if !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(D::Error::custom("digest contains invalid hex digits"));
        }
        let mut bytes = [0u8; 32];
        for (index, pair) in hex.as_bytes().chunks_exact(2).enumerate() {
            let pair = std::str::from_utf8(pair).map_err(D::Error::custom)?;
            bytes[index] = u8::from_str_radix(pair, 16).map_err(D::Error::custom)?;
        }
        Ok(Self(bytes))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisVisibility {
    #[default]
    Standard,
    PublicSyntheticCalibration,
    LocalPrivateCalibration,
    RedactedCalibration,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysisBudgetConfiguration {
    pub filesystem_max_depth: usize,
    pub filesystem_max_entries: usize,
    pub filesystem_max_ignored_records: usize,
    pub filesystem_additional_ignored_names: Vec<String>,
    pub document_max_documents: usize,
    pub document_max_total_source_bytes: usize,
    pub document_max_source_bytes: usize,
    pub document_max_events: usize,
    pub document_max_diagnostics: usize,
    pub git_max_refs: u32,
    pub git_max_tags: u32,
    pub git_max_commit_headers: u32,
    pub scope: ScopeReadBudget,
}

impl Default for AnalysisBudgetConfiguration {
    fn default() -> Self {
        Self {
            filesystem_max_depth: 32,
            filesystem_max_entries: 100_000,
            filesystem_max_ignored_records: 4_096,
            filesystem_additional_ignored_names: Vec::new(),
            document_max_documents: 32,
            document_max_total_source_bytes: 4 * 1024 * 1024,
            document_max_source_bytes: 2 * 1024 * 1024,
            document_max_events: 65_536,
            document_max_diagnostics: 1_024,
            git_max_refs: 4_096,
            git_max_tags: 2_048,
            git_max_commit_headers: 10_000,
            scope: ScopeReadBudget::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysisConfiguration {
    pub schema_version: String,
    pub scope: AnalysisScope,
    pub profile: ProfileKind,
    pub budgets: AnalysisBudgetConfiguration,
    pub pattern_registry_fingerprint: String,
    pub visibility: AnalysisVisibility,
    pub redacted_calibration_fingerprint: Option<String>,
}

impl Default for AnalysisConfiguration {
    fn default() -> Self {
        Self {
            schema_version: crate::ANALYSIS_SCHEMA_VERSION.to_string(),
            scope: AnalysisScope::Repository,
            profile: ProfileKind::Common,
            budgets: AnalysisBudgetConfiguration::default(),
            pattern_registry_fingerprint: String::new(),
            visibility: AnalysisVisibility::Standard,
            redacted_calibration_fingerprint: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortableObservationState {
    Present,
    Absent,
    Conflict,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortableRouteRecord {
    pub route: RouteKind,
    pub root_structured: bool,
    pub inherited: bool,
    pub readme_routed: bool,
    pub repository_local_targets: usize,
    pub shared_target_conflicts: usize,
    pub freshness: RouteFreshness,
    pub policy: RoutePolicyBoundary,
    pub missing_pattern: bool,
    pub observation: PortableObservationState,
    pub coverage: CoverageStatus,
    pub evidence_ids: Vec<EvidenceId>,
    pub digest: Digest32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortableContentSlotRecord {
    pub slot: ContentSlotId,
    pub code: String,
    pub route: RouteKind,
    pub observation: PortableObservationState,
    pub coverage: CoverageStatus,
    pub evidence_ids: Vec<EvidenceId>,
    pub digest: Digest32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortableCoverageRecord {
    pub key: String,
    pub status: CoverageStatus,
    pub digest: Digest32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortableConflictRecord {
    pub id: String,
    pub route: RouteKind,
    pub evidence_ids: Vec<EvidenceId>,
    pub digest: Digest32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortableObligationRecord {
    pub id: String,
    pub route: RouteKind,
    pub observation: PortableObservationState,
    pub evidence_ids: Vec<EvidenceId>,
    pub coverage: CoverageStatus,
    pub digest: Digest32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortableFacetRecord {
    pub facet: RepositoryFacet,
    pub observation: PortableObservationState,
    pub evidence_ids: Vec<EvidenceId>,
    pub coverage: CoverageStatus,
    pub digest: Digest32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortableDocumentRecord {
    pub document: Option<DocumentId>,
    pub path: String,
    pub coverage: CoverageStatus,
    pub digest: Digest32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditSnapshotDigest {
    pub schema: String,
    pub configuration: Digest32,
    pub evidence: Digest32,
    pub routes: Digest32,
    pub documents: Digest32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortableAuditSnapshot {
    pub schema_version: String,
    pub configuration: AnalysisConfiguration,
    pub digest: AuditSnapshotDigest,
    pub routes: Vec<PortableRouteRecord>,
    pub content_slots: Vec<PortableContentSlotRecord>,
    pub coverage: Vec<PortableCoverageRecord>,
    pub conflicts: Vec<PortableConflictRecord>,
    pub obligations: Vec<PortableObligationRecord>,
    pub facets: Vec<PortableFacetRecord>,
    pub documents: Vec<PortableDocumentRecord>,
    pub boundary: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeltaState {
    Added,
    Removed,
    Changed,
    Unchanged,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeltaUnknownReason {
    SchemaMismatch,
    ScopeMismatch,
    ConfigurationMismatch,
    PartialCoverage,
    MissingComparableRecord,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", content = "reason", rename_all = "snake_case")]
pub enum DeltaCompatibility {
    Comparable,
    Unknown(DeltaUnknownReason),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteDelta {
    pub route: RouteKind,
    pub state: DeltaState,
    pub before: Option<PortableRouteRecord>,
    pub after: Option<PortableRouteRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactDelta {
    pub key: String,
    pub state: DeltaState,
    pub before: Option<Digest32>,
    pub after: Option<Digest32>,
    pub before_coverage: CoverageStatus,
    pub after_coverage: CoverageStatus,
    pub evidence_ids: Vec<EvidenceId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegressionCandidate {
    pub domain: String,
    pub key: String,
    pub state: DeltaState,
    pub evidence_ids: Vec<EvidenceId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImprovementCandidate {
    pub domain: String,
    pub key: String,
    pub state: DeltaState,
    pub evidence_ids: Vec<EvidenceId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditDeltaReport {
    pub schema_version: String,
    pub compatibility: DeltaCompatibility,
    pub before: AuditSnapshotDigest,
    pub after: AuditSnapshotDigest,
    pub routes: Vec<RouteDelta>,
    pub content_slots: Vec<ArtifactDelta>,
    pub coverage: Vec<ArtifactDelta>,
    pub conflicts: Vec<ArtifactDelta>,
    pub obligations: Vec<ArtifactDelta>,
    pub facets: Vec<ArtifactDelta>,
    pub regressions: Vec<RegressionCandidate>,
    pub improvements: Vec<ImprovementCandidate>,
    pub boundary: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatchHoldReason {
    NoExistingTarget,
    TargetNotRepositoryLocal,
    CanonicalConflict,
    UnknownTargetRelation,
    MissingReadme,
    StaleBase,
    StaleAnchor,
    PairedLanguageIncomplete,
    UnsupportedEncoding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ExistingTargetId(pub u32);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddExistingRouteLink {
    pub route: RouteKind,
    pub target: ExistingTargetId,
    pub target_path: String,
    pub target_role: RouteTargetRole,
    pub document: DocumentId,
    pub insertion_anchor: crate::PatchAnchorContext,
    pub analysis_run: crate::PatchAnalysisRun,
    pub proposal: crate::PatchProposal,
    pub binding: crate::PatchProposalBinding,
    pub paired_language: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchHold {
    pub route: RouteKind,
    pub target_path: Option<String>,
    pub reason: PatchHoldReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchPlan {
    pub schema_version: String,
    pub operations: Vec<AddExistingRouteLink>,
    pub held: Vec<PatchHold>,
    pub writes_files: bool,
    pub boundary: String,
}

impl Default for PatchPlan {
    fn default() -> Self {
        Self {
            schema_version: PATCH_PLAN_SCHEMA_VERSION.to_string(),
            operations: Vec::new(),
            held: Vec::new(),
            writes_files: false,
            boundary: "Patch planning emits dry-run links to existing repository-local targets only. It does not write files, generate policy bodies, execute Git or GitHub operations, or establish authenticity, safety, or correctness.".to_string(),
        }
    }
}
