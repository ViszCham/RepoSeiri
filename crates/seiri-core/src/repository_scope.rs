use crate::{CoverageStatus, EvidenceId, FileKind, RouteState};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisScope {
    #[default]
    Repository,
    Workspace,
    Subtree,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepositoryRootKind {
    Worktree,
    LinkedWorktree,
    Bare,
    Subtree,
    MalformedGit,
    NoGit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryScopeRoot {
    pub analysis_root: String,
    pub worktree_root: Option<String>,
    pub git_dir: Option<String>,
    pub common_dir: Option<String>,
    pub kind: RepositoryRootKind,
    pub scope: AnalysisScope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitReadBudget {
    pub max_refs: u32,
    pub max_tags: u32,
    pub max_commit_headers: u32,
}

impl Default for GitReadBudget {
    fn default() -> Self {
        Self {
            max_refs: 4_096,
            max_tags: 2_048,
            max_commit_headers: 10_000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeReadBudget {
    pub max_nodes: u32,
    pub max_manifest_bytes: u64,
    pub max_ignored_records: u32,
}

impl Default for ScopeReadBudget {
    fn default() -> Self {
        Self {
            max_nodes: 4_096,
            max_manifest_bytes: 2 * 1024 * 1024,
            max_ignored_records: 4_096,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitTimestamp {
    pub seconds_since_epoch: i64,
    pub offset_minutes: i16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitReferenceKind {
    LocalBranch,
    RemoteBranch,
    Tag,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitReferenceObservation {
    pub name: String,
    pub target: String,
    pub kind: GitReferenceKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitCommitHeader {
    pub object_id: String,
    pub committed_at: GitTimestamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitObservationState {
    Available,
    NoRepository,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitDiagnosticKind {
    MalformedGitFile,
    EscapedMetadataBoundary,
    AlternateObjectDirectoryDisabled,
    PackedReferencesTooLarge,
    MalformedHead,
    MalformedReference,
    ObjectDecodeFailed,
    ShallowRepository,
    PartialRepository,
    PermissionDenied,
    Io,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitDiagnostic {
    pub kind: GitDiagnosticKind,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitTemporalObservation {
    pub state: GitObservationState,
    pub head_name: Option<String>,
    pub head_target: Option<String>,
    pub references: Vec<GitReferenceObservation>,
    pub commits: Vec<GitCommitHeader>,
    pub refs_coverage: CoverageStatus,
    pub tags_coverage: CoverageStatus,
    pub commits_coverage: CoverageStatus,
    pub shallow: bool,
    pub partial: bool,
    pub diagnostics: Vec<GitDiagnostic>,
    pub boundary: String,
}

impl Default for GitTemporalObservation {
    fn default() -> Self {
        Self {
            state: GitObservationState::NoRepository,
            head_name: None,
            head_target: None,
            references: Vec::new(),
            commits: Vec::new(),
            refs_coverage: CoverageStatus::NotRequested,
            tags_coverage: CoverageStatus::NotRequested,
            commits_coverage: CoverageStatus::NotRequested,
            shallow: false,
            partial: false,
            diagnostics: Vec::new(),
            boundary: "Git-local observations are bounded local metadata. Timestamps do not indicate maintenance, abandonment, health, or repository quality.".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ScopeNodeId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScopeNodeKind {
    Repository,
    Workspace,
    Package,
    Documentation,
    Example,
    Fixture,
    Submodule,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceManifestKind {
    Cargo,
    Npm,
    Pyproject,
    GoWork,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManifestObservationStatus {
    Parsed,
    SourceTooLarge,
    InvalidUtf8,
    Malformed,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceManifestObservation {
    pub path: String,
    pub kind: WorkspaceManifestKind,
    pub status: ManifestObservationStatus,
    pub declares_workspace: bool,
    pub declares_package: bool,
    pub declared_members: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeNode {
    pub id: ScopeNodeId,
    pub kind: ScopeNodeKind,
    pub path: String,
    pub manifest: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScopeEdgeKind {
    Contains,
    DeclaresMember,
    PackageManifest,
    Documentation,
    Example,
    Fixture,
    SubmoduleBoundary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeEdge {
    pub from: ScopeNodeId,
    pub to: ScopeNodeId,
    pub kind: ScopeEdgeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IgnoredPathReason {
    GitMetadata,
    BuildOutput,
    DependencyTree,
    VirtualEnvironment,
    EditorState,
    DistributionOutput,
    UserConfigured,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IgnoredShallowRecord {
    pub path: String,
    pub kind: FileKind,
    pub reason: IgnoredPathReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryScopeGraph {
    pub nodes: Vec<ScopeNode>,
    pub edges: Vec<ScopeEdge>,
    pub manifests: Vec<WorkspaceManifestObservation>,
    pub ignored: Vec<IgnoredShallowRecord>,
    pub node_coverage: CoverageStatus,
    pub manifest_coverage: CoverageStatus,
    pub ignored_coverage: CoverageStatus,
    pub boundary: String,
}

impl Default for RepositoryScopeGraph {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            manifests: Vec::new(),
            ignored: Vec::new(),
            node_coverage: CoverageStatus::NotRequested,
            manifest_coverage: CoverageStatus::NotRequested,
            ignored_coverage: CoverageStatus::NotRequested,
            boundary: "Scope nodes preserve repository, workspace, package, documentation, example, fixture, and submodule boundaries. Package policy is not promoted to repository policy.".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryScopeReport {
    pub root: RepositoryScopeRoot,
    pub git: GitTemporalObservation,
    pub graph: RepositoryScopeGraph,
}

impl Default for RepositoryScopeReport {
    fn default() -> Self {
        Self {
            root: RepositoryScopeRoot {
                analysis_root: String::new(),
                worktree_root: None,
                git_dir: None,
                common_dir: None,
                kind: RepositoryRootKind::NoGit,
                scope: AnalysisScope::Repository,
            },
            git: GitTemporalObservation::default(),
            graph: RepositoryScopeGraph::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetReachabilityFreshness {
    pub repository_local_present: usize,
    pub repository_local_missing: usize,
    pub non_local_or_unknown: usize,
    pub coverage: CoverageStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemporalActivityFreshness {
    pub observed_commit_headers: usize,
    pub newest: Option<GitTimestamp>,
    pub oldest: Option<GitTimestamp>,
    pub coverage: CoverageStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecycleSignalFreshness {
    pub route_state: Option<RouteState>,
    pub evidence_ids: Vec<EvidenceId>,
    pub coverage: CoverageStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FreshnessReport {
    pub target_reachability: TargetReachabilityFreshness,
    pub temporal_activity: TemporalActivityFreshness,
    pub lifecycle_signal: LifecycleSignalFreshness,
    pub boundary: String,
}

impl Default for FreshnessReport {
    fn default() -> Self {
        Self {
            target_reachability: TargetReachabilityFreshness {
                repository_local_present: 0,
                repository_local_missing: 0,
                non_local_or_unknown: 0,
                coverage: CoverageStatus::NotRequested,
            },
            temporal_activity: TemporalActivityFreshness {
                observed_commit_headers: 0,
                newest: None,
                oldest: None,
                coverage: CoverageStatus::NotRequested,
            },
            lifecycle_signal: LifecycleSignalFreshness {
                route_state: None,
                evidence_ids: Vec::new(),
                coverage: CoverageStatus::NotRequested,
            },
            boundary: "Target reachability, local commit timestamps, and lifecycle wording are separate dimensions. No dimension establishes maintenance, abandonment, health, or quality.".to_string(),
        }
    }
}
