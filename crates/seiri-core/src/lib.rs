use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: &str = "seiri.block_f.v1";
pub const TOOL_NAME: &str = "RepoSeiri";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoSnapshot {
    pub schema_version: String,
    pub tool: String,
    pub repo_root: String,
    pub entry_count: usize,
    pub files: Vec<FileRecord>,
    pub important_files: Vec<ImportantFile>,
    pub readme: Option<ReadmeSummary>,
    pub evidence: Vec<Evidence>,
    pub pattern_matches: Vec<PatternMatch>,
    pub baseline: Option<BaselineReport>,
    pub profile: Option<ProfileReport>,
    pub findings: Vec<Finding>,
}

impl RepoSnapshot {
    #[must_use]
    pub fn new(repo_root: impl Into<String>) -> Self {
        Self {
            schema_version: SCHEMA_VERSION.to_string(),
            tool: TOOL_NAME.to_string(),
            repo_root: repo_root.into(),
            entry_count: 0,
            files: Vec::new(),
            important_files: Vec::new(),
            readme: None,
            evidence: Vec::new(),
            pattern_matches: Vec::new(),
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
    Changelog,
    Codeowners,
    CargoToml,
    DocsDirectory,
    Workflow,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadmeSummary {
    pub path: String,
    pub headings: Vec<MarkdownHeading>,
    pub links: Vec<MarkdownLink>,
    pub badges: Vec<MarkdownBadge>,
    pub route_candidates: Vec<RouteCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkdownHeading {
    pub level: u8,
    pub text: String,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkdownLink {
    pub text: String,
    pub target: String,
    pub line: usize,
    pub route: Option<RouteKind>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkdownBadge {
    pub alt: String,
    pub target: String,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteCandidate {
    pub route: RouteKind,
    pub source: RouteSource,
    pub text: String,
    pub target: Option<String>,
    pub line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
    Contributing,
    Security,
    Release,
    Governance,
    License,
    Automation,
    Ownership,
    Hygiene,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evidence {
    pub id: String,
    pub kind: EvidenceKind,
    pub path: Option<String>,
    pub route: Option<RouteKind>,
    pub value: String,
    pub source: EvidenceSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
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
    pub evidence_ids: Vec<String>,
    pub basis: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternOutcome {
    Present,
    Missing,
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
    pub evidence_ids: Vec<String>,
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
    Docs,
    Tutorial,
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
            Self::Docs => "docs",
            Self::Tutorial => "tutorial",
            Self::Research => "research",
            Self::Template => "template",
        };
        f.write_str(value)
    }
}

impl std::str::FromStr for ProfileKind {
    type Err = ProfileParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "common" => Ok(Self::Common),
            "library" | "lib" => Ok(Self::Library),
            "cli" | "command-line" | "command_line" => Ok(Self::Cli),
            "infra" | "infrastructure" => Ok(Self::Infra),
            "docs" | "documentation" => Ok(Self::Docs),
            "tutorial" => Ok(Self::Tutorial),
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
pub struct ProfileScoreView {
    pub earned_weight: u32,
    pub total_weight: u32,
    pub score_x100: u8,
    pub present_rules: usize,
    pub missing_rules: usize,
    pub note: String,
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
    pub evidence_ids: Vec<String>,
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
pub struct EvidenceSchemaVersion {
    pub schema_version: String,
    pub compatible_from: String,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BenchmarkDataset {
    pub schema_version: String,
    pub dataset_id: String,
    pub name: String,
    pub collected_at: String,
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
    pub evidence_kind: Option<EvidenceKind>,
    #[serde(default)]
    pub route: Option<RouteKind>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default = "default_observation_count")]
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalibrationRun {
    pub schema_version: String,
    pub run_id: String,
    pub dataset_id: String,
    pub summary: CalibrationSummary,
    pub stats: Vec<PatternStats>,
    pub pending_patterns: Vec<PendingPatternCandidate>,
    pub weight_suggestions: Vec<WeightSuggestion>,
    pub claim_boundary: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalibrationSummary {
    pub records: usize,
    pub known_pattern_stats: usize,
    pub pending_patterns: usize,
    pub weight_suggestions: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatternStats {
    pub pattern_id: String,
    pub repositories: usize,
    pub observations: u64,
    pub frequency_x1000: u16,
    pub profile_correlations: Vec<ProfilePatternCorrelation>,
    pub co_occurrences: Vec<PatternCoOccurrence>,
    pub confidence: CalibrationConfidence,
    pub confidence_note: String,
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
    pub example_locations: Vec<String>,
    pub review_status: CalibrationReviewStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeightSuggestion {
    pub id: String,
    pub profile: ProfileKind,
    pub pattern_id: String,
    pub current_weight: Option<u32>,
    pub suggested_weight: u32,
    pub suggested_delta: i32,
    pub priority: ProfilePriority,
    pub support_repositories: usize,
    pub frequency_x1000: u16,
    pub confidence: CalibrationConfidence,
    pub review_status: CalibrationReviewStatus,
    pub rationale: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexReviewContext {
    pub schema_version: String,
    pub tool: String,
    pub repo_root: String,
    pub profile: Option<ProfileKind>,
    pub audit: CodexAuditSummary,
    pub plan: PatchPlanSummary,
    pub findings: Vec<CodexFindingDigest>,
    pub safe_operations: Vec<CodexPatchDigest>,
    pub blocked_items: Vec<CodexBlockedDigest>,
    pub user_actions: Vec<CodexUserAction>,
    pub pr_draft: CodexPrDraft,
    pub claim_boundary: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexAuditSummary {
    pub entries_scanned: usize,
    pub evidence_items: usize,
    pub findings: usize,
    pub pattern_matches: usize,
    pub profile_score_x100: Option<u8>,
    pub required_present: Option<usize>,
    pub required_missing: Option<usize>,
    pub optional_present: Option<usize>,
    pub optional_missing: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexFindingDigest {
    pub id: String,
    pub severity: Severity,
    pub title: String,
    pub gate: Option<GateKind>,
    pub recommendation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexPatchDigest {
    pub id: String,
    pub gate: GateKind,
    pub kind: PatchOperationKind,
    pub path: String,
    pub title: String,
    pub planned_change: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexBlockedDigest {
    pub id: String,
    pub gate: GateKind,
    pub pattern_id: String,
    pub title: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexUserAction {
    pub id: String,
    pub label: String,
    pub command: String,
    pub mutates_files: bool,
    pub requires_confirmation: bool,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexPrDraft {
    pub title: String,
    pub body: String,
    pub labels: Vec<String>,
    pub draft: bool,
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
pub struct PatchPlan {
    pub schema_version: String,
    pub mode: PatchPlanMode,
    pub profile: Option<ProfileKind>,
    pub summary: PatchPlanSummary,
    pub operations: Vec<PatchPlanOperation>,
    pub blocked: Vec<PatchPlanBlockedItem>,
    pub claim_boundary: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatchPlanMode {
    DryRun,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchPlanSummary {
    pub safe_operations: usize,
    pub safe_blocked: usize,
    pub guarded_items: usize,
    pub manual_items: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchPlanOperation {
    pub id: String,
    pub gate: GateKind,
    pub kind: PatchOperationKind,
    pub title: String,
    pub path: String,
    pub route: Option<RouteKind>,
    pub finding_id: Option<String>,
    pub pattern_id: String,
    pub rationale: String,
    pub planned_change: String,
    pub diff_preview: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatchOperationKind {
    AddReadmeRoute,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchPlanBlockedItem {
    pub id: String,
    pub gate: GateKind,
    pub title: String,
    pub finding_id: Option<String>,
    pub pattern_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceSource {
    pub scanner: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub severity: Severity,
    pub title: String,
    pub message: String,
    pub evidence_ids: Vec<String>,
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

fn default_observation_count() -> u32 {
    1
}
