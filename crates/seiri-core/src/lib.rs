use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: &str = "seiri.block_p.v1";
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
    pub evidence_ledger: Vec<EvidenceRecord>,
    pub pattern_matches: Vec<PatternMatch>,
    pub route_states: Vec<RouteStateReport>,
    pub missing_route_priority: MissingRoutePriorityReport,
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
            evidence_ledger: Vec::new(),
            pattern_matches: Vec::new(),
            route_states: Vec::new(),
            missing_route_priority: MissingRoutePriorityReport::empty(),
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadmeRouteMapEntry {
    pub route: RouteKind,
    pub state: RouteState,
    pub observed_gap_count: Option<u32>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evidence {
    pub id: String,
    pub kind: EvidenceKind,
    pub path: Option<String>,
    pub route: Option<RouteKind>,
    pub value: String,
    pub source: EvidenceSource,
}

pub type EvidenceId = String;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceRecord {
    pub id: EvidenceId,
    pub legacy_evidence_id: Option<String>,
    pub kind: EvidenceKind,
    pub path: Option<String>,
    pub route: Option<RouteKind>,
    pub value: String,
    pub source: EvidenceSource,
    pub scope: EvidenceScope,
    pub confidence: EvidenceConfidence,
    pub span: Option<EvidenceSpan>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceScope {
    Root,
    Nested,
    Fixture,
    Generated,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceConfidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceSpan {
    pub start_line: usize,
    pub end_line: usize,
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
pub struct RouteStateReport {
    pub route: RouteKind,
    pub state: RouteState,
    pub evidence_ids: Vec<EvidenceId>,
    pub confidence: EvidenceConfidence,
    pub reason: String,
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
            boundary: "Missing route priority is a deterministic routing hint from observed evidence, fixed calibration priors, and route co-occurrence rules; it is not a popularity, trust, security, quality, or policy guarantee.".to_string(),
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
pub struct MissingRoutePriority {
    pub rank: usize,
    pub route: RouteKind,
    pub state: RouteState,
    pub gate: GateKind,
    pub severity: Severity,
    pub priority: ProfilePriority,
    pub priority_score_x100: u8,
    pub observed_missing_repositories: Option<u32>,
    pub observed_missing_x1000: Option<u16>,
    pub baseline_pattern_ids: Vec<String>,
    pub candidate_pattern_ids: Vec<String>,
    pub co_occurrence_gap_ids: Vec<String>,
    pub evidence_ids: Vec<EvidenceId>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteCoOccurrenceGap {
    pub id: String,
    pub title: String,
    pub observed_repositories: u32,
    pub support_x1000: u16,
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
    pub top_confidence_x100: Option<u8>,
    pub emitted_profiles: usize,
    pub ambiguous: bool,
    pub boundary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileBranch {
    pub rank: usize,
    pub profile: ProfileKind,
    pub prior_x1000: u16,
    pub confidence_x100: u8,
    pub evidence_score_x100: u8,
    pub score_x100: u8,
    pub matched_signals: Vec<String>,
    pub missing_signals: Vec<String>,
    pub rationale: String,
}

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
    pub sources: Vec<CalibrationSource>,
    pub summary: CalibrationSummary,
    pub stats: Vec<PatternStats>,
    pub route_requirements: Vec<RouteRequirement>,
    pub profile_branches: Vec<ProfileBranch>,
    pub pending_patterns: Vec<PendingPatternCandidate>,
    pub weight_suggestions: Vec<WeightSuggestion>,
    pub claim_boundary: ClaimBoundary,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalibrationSource {
    pub id: String,
    pub kind: CalibrationSourceKind,
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
pub struct CodexReviewContext {
    pub schema_version: String,
    pub tool: String,
    pub repo_root: String,
    pub profile: Option<ProfileKind>,
    pub audit: CodexAuditSummary,
    pub route_review: CodexRouteReviewSummary,
    pub routes: Vec<CodexRouteDigest>,
    pub co_occurrence_gaps: Vec<CodexCoOccurrenceDigest>,
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
    pub evidence_ledger_records: usize,
    pub route_states: usize,
    pub strong_routes: usize,
    pub weak_routes: usize,
    pub missing_routes: usize,
    pub findings: usize,
    pub pattern_matches: usize,
    pub profile_score_x100: Option<u8>,
    pub profile_branches: usize,
    pub top_profile: Option<ProfileKind>,
    pub top_profile_confidence_x100: Option<u8>,
    pub missing_route_priorities: usize,
    pub co_occurrence_gaps: usize,
    pub top_missing_route: Option<RouteKind>,
    pub top_missing_route_priority_x100: Option<u8>,
    pub required_present: Option<usize>,
    pub required_missing: Option<usize>,
    pub optional_present: Option<usize>,
    pub optional_missing: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexRouteReviewSummary {
    pub strong_routes: usize,
    pub weak_routes: usize,
    pub missing_routes: usize,
    pub co_occurrence_gaps: usize,
    pub safe_fixes: usize,
    pub guarded_drafts: usize,
    pub manual_decisions: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexRouteDigest {
    pub route: RouteKind,
    pub state: RouteState,
    pub confidence: EvidenceConfidence,
    pub evidence_ids: Vec<EvidenceId>,
    pub priority_score_x100: Option<u8>,
    pub gate: Option<GateKind>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexCoOccurrenceDigest {
    pub id: String,
    pub title: String,
    pub gate: GateKind,
    pub priority: ProfilePriority,
    pub present_routes: Vec<RouteKind>,
    pub missing_routes: Vec<RouteKind>,
    pub missing_signals: Vec<String>,
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
    pub safety: PatchSafetyLevel,
    pub preview_only: bool,
    pub requires_confirmation: bool,
    pub path: String,
    pub title: String,
    pub planned_change: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexBlockedDigest {
    pub id: String,
    pub gate: GateKind,
    pub source: PatchPlanSource,
    pub safety: PatchSafetyLevel,
    pub route: Option<RouteKind>,
    pub priority: ProfilePriority,
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
    pub planner_version: String,
    pub mode: PatchPlanMode,
    pub profile: Option<ProfileKind>,
    pub safety_policy: PatchPlanSafetyPolicy,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchPlanSafetyPolicy {
    pub version: String,
    pub writes_files: bool,
    pub applies_patches: bool,
    pub safe_gate_only: bool,
    pub requires_existing_targets: bool,
    pub blocks_unsafe_to_invent: bool,
    pub guarded_and_manual_are_blocked: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchPlanSummary {
    pub total_candidates: usize,
    pub safe_operations: usize,
    pub safe_blocked: usize,
    pub guarded_items: usize,
    pub manual_items: usize,
    pub preview_only_operations: usize,
    pub preflight_passed: usize,
    pub preflight_failed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchPlanOperation {
    pub id: String,
    pub gate: GateKind,
    pub kind: PatchOperationKind,
    pub source: PatchPlanSource,
    pub safety: PatchSafetyLevel,
    pub priority: ProfilePriority,
    pub title: String,
    pub path: String,
    pub route: Option<RouteKind>,
    pub finding_id: Option<String>,
    pub pattern_id: String,
    pub preview_only: bool,
    pub requires_confirmation: bool,
    pub rationale: String,
    pub planned_change: String,
    pub preflight: Vec<PatchPreflightCheck>,
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
    pub source: PatchPlanSource,
    pub safety: PatchSafetyLevel,
    pub severity: Severity,
    pub priority: ProfilePriority,
    pub title: String,
    pub route: Option<RouteKind>,
    pub finding_id: Option<String>,
    pub pattern_id: String,
    pub reason: String,
    pub preflight: Vec<PatchPreflightCheck>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatchPlanSource {
    ProfileRecommendation,
    FindingRecommendation,
    MissingRoutePriority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatchSafetyLevel {
    PreviewOnly,
    ReviewRequired,
    ManualOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchPreflightCheck {
    pub kind: PatchPreflightCheckKind,
    pub status: PatchPreflightStatus,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatchPreflightCheckKind {
    DryRunOnly,
    SafeGate,
    RouteSafeToInvent,
    SupportedOperation,
    ExistingReadme,
    ReadmeRouteAbsent,
    ExistingTarget,
    NoPolicyContent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatchPreflightStatus {
    Pass,
    Blocked,
    Fail,
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
