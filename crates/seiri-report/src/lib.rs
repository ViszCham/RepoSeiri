#![forbid(unsafe_code)]

use seiri_core::{
    CalibrationProvider, CalibrationRun, ClaimRefIndex, ClaimStrength, NoCalibrationProvider,
    PatchPlan, ProfileKind, RepositoryAnalysis, RouteKind, WordingLintReport,
};
use serde::Serialize;
use std::fmt::{Display, Formatter};
use std::io;
use std::path::Path;

pub use seiri_codex::{CodexQueryKind, CodexQueryParseError};

mod claims;
mod evidence;
mod fixture_runner;
mod obligation_graph;
mod route_content;
mod route_priority;
mod wording;

use claims::build_content_claims;
use evidence::{build_evidence_kernel, build_route_assessments};
pub use fixture_runner::run_executable_pattern_pack;
use obligation_graph::{build_document_consistency_report, build_route_targets};
use route_content::build_route_content;
use route_priority::{build_missing_route_priority_report, build_review_priority_report};

#[derive(Debug)]
pub enum AuditError {
    Fs(seiri_fs::FsError),
    Markdown(seiri_markdown::MarkdownError),
    Calibration(seiri_calibration::CalibrationError),
    LocalPrior(seiri_calibration::LocalPriorLoadError),
    EvidenceKernel(seiri_core::EvidenceKernelError),
    DocumentIndex(seiri_core::DocumentIndexError),
    GithubLocal(seiri_github_local::GithubLocalParserError),
    Coverage(seiri_core::CoverageIndexError),
    RouteAssessment(seiri_core::RouteAssessmentError),
    DocumentConsistency(seiri_core::DocumentConsistencyError),
    GitLocal(seiri_git_local::GitLocalError),
    Delta(seiri_delta::DeltaError),
    Json(serde_json::Error),
    Io {
        path: std::path::PathBuf,
        source: io::Error,
    },
}

impl Display for AuditError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fs(error) => write!(f, "{error}"),
            Self::Markdown(error) => write!(f, "{error}"),
            Self::Calibration(error) => write!(f, "{error}"),
            Self::LocalPrior(error) => write!(f, "{error}"),
            Self::EvidenceKernel(error) => write!(f, "{error}"),
            Self::DocumentIndex(error) => write!(f, "{error}"),
            Self::GithubLocal(error) => write!(f, "{error}"),
            Self::Coverage(error) => write!(f, "{error}"),
            Self::RouteAssessment(error) => write!(f, "{error}"),
            Self::DocumentConsistency(error) => write!(f, "{error}"),
            Self::GitLocal(error) => write!(f, "{error}"),
            Self::Delta(error) => write!(f, "{error}"),
            Self::Json(error) => write!(f, "{error}"),
            Self::Io { path, source } => write!(f, "failed to read {}: {source}", path.display()),
        }
    }
}

impl std::error::Error for AuditError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Fs(error) => Some(error),
            Self::Markdown(error) => Some(error),
            Self::Calibration(error) => Some(error),
            Self::LocalPrior(error) => Some(error),
            Self::EvidenceKernel(error) => Some(error),
            Self::DocumentIndex(error) => Some(error),
            Self::GithubLocal(error) => Some(error),
            Self::Coverage(error) => Some(error),
            Self::RouteAssessment(error) => Some(error),
            Self::DocumentConsistency(error) => Some(error),
            Self::GitLocal(error) => Some(error),
            Self::Delta(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::Io { source, .. } => Some(source),
        }
    }
}

impl From<seiri_fs::FsError> for AuditError {
    fn from(value: seiri_fs::FsError) -> Self {
        Self::Fs(value)
    }
}

impl From<seiri_markdown::MarkdownError> for AuditError {
    fn from(value: seiri_markdown::MarkdownError) -> Self {
        Self::Markdown(value)
    }
}

impl From<seiri_calibration::CalibrationError> for AuditError {
    fn from(value: seiri_calibration::CalibrationError) -> Self {
        Self::Calibration(value)
    }
}

impl From<seiri_calibration::LocalPriorLoadError> for AuditError {
    fn from(value: seiri_calibration::LocalPriorLoadError) -> Self {
        Self::LocalPrior(value)
    }
}

impl From<seiri_core::EvidenceKernelError> for AuditError {
    fn from(value: seiri_core::EvidenceKernelError) -> Self {
        Self::EvidenceKernel(value)
    }
}

impl From<seiri_core::DocumentIndexError> for AuditError {
    fn from(value: seiri_core::DocumentIndexError) -> Self {
        Self::DocumentIndex(value)
    }
}

impl From<seiri_github_local::GithubLocalParserError> for AuditError {
    fn from(value: seiri_github_local::GithubLocalParserError) -> Self {
        Self::GithubLocal(value)
    }
}

impl From<seiri_core::CoverageIndexError> for AuditError {
    fn from(value: seiri_core::CoverageIndexError) -> Self {
        Self::Coverage(value)
    }
}

impl From<seiri_core::RouteAssessmentError> for AuditError {
    fn from(value: seiri_core::RouteAssessmentError) -> Self {
        Self::RouteAssessment(value)
    }
}

impl From<seiri_core::DocumentConsistencyError> for AuditError {
    fn from(value: seiri_core::DocumentConsistencyError) -> Self {
        Self::DocumentConsistency(value)
    }
}

impl From<seiri_git_local::GitLocalError> for AuditError {
    fn from(value: seiri_git_local::GitLocalError) -> Self {
        Self::GitLocal(value)
    }
}

impl From<seiri_delta::DeltaError> for AuditError {
    fn from(value: seiri_delta::DeltaError) -> Self {
        Self::Delta(value)
    }
}

pub fn portable_audit_snapshot(
    snapshot: &RepositoryAnalysis,
) -> Result<seiri_core::PortableAuditSnapshot, AuditError> {
    Ok(seiri_delta::portable_snapshot(snapshot)?)
}

pub fn diff_snapshots(
    before: &RepositoryAnalysis,
    after: &RepositoryAnalysis,
) -> Result<seiri_core::AuditDeltaReport, AuditError> {
    let before = seiri_delta::portable_snapshot(before)?;
    let after = seiri_delta::portable_snapshot(after)?;
    Ok(seiri_delta::compare(&before, &after))
}

pub fn audit_delta_to_json(report: &seiri_core::AuditDeltaReport) -> Result<String, AuditError> {
    Ok(serde_json::to_string_pretty(report)?)
}

#[must_use]
pub fn audit_delta_to_markdown(report: &seiri_core::AuditDeltaReport) -> String {
    let mut output = String::new();
    output.push_str("# RepoSeiri Audit Delta\n\n");
    output.push_str(&format!("- Compatibility: `{:?}`\n", report.compatibility));
    output.push_str(&format!("- Route deltas: {}\n", report.routes.len()));
    output.push_str(&format!(
        "- Regression candidates: {}\n",
        report.regressions.len()
    ));
    output.push_str(&format!(
        "- Improvement candidates: {}\n\n",
        report.improvements.len()
    ));
    if !report.routes.is_empty() {
        output.push_str("## Routes\n\n| Route | State |\n|---|---|\n");
        for delta in &report.routes {
            output.push_str(&format!("| `{:?}` | `{:?}` |\n", delta.route, delta.state));
        }
        output.push('\n');
    }
    if !report.regressions.is_empty() {
        output.push_str("## Regression Candidates\n\n");
        for regression in &report.regressions {
            output.push_str(&format!(
                "- `{}` / `{}`: `{:?}`\n",
                regression.domain, regression.key, regression.state
            ));
        }
        output.push('\n');
    }
    if !report.improvements.is_empty() {
        output.push_str("## Improvement Candidates\n\n");
        for improvement in &report.improvements {
            output.push_str(&format!(
                "- `{}` / `{}`: `{:?}`\n",
                improvement.domain, improvement.key, improvement.state
            ));
        }
        output.push('\n');
    }
    output.push_str("## Boundary\n\n");
    output.push_str(&report.boundary);
    output.push('\n');
    output
}

impl From<serde_json::Error> for AuditError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

pub fn audit_repository(path: impl AsRef<Path>) -> Result<RepositoryAnalysis, AuditError> {
    audit_repository_with_profile(path, ProfileKind::Common)
}

pub fn audit_repository_subtree(path: impl AsRef<Path>) -> Result<RepositoryAnalysis, AuditError> {
    audit_repository_with_scope(
        path,
        ProfileKind::Common,
        seiri_core::AnalysisScope::Subtree,
    )
}

pub fn audit_repository_subtree_with_profile(
    path: impl AsRef<Path>,
    profile: ProfileKind,
) -> Result<RepositoryAnalysis, AuditError> {
    audit_repository_with_scope(path, profile, seiri_core::AnalysisScope::Subtree)
}

pub fn audit_repository_with_profile(
    path: impl AsRef<Path>,
    profile: ProfileKind,
) -> Result<RepositoryAnalysis, AuditError> {
    audit_repository_with_options_and_calibration(
        path,
        profile,
        &seiri_fs::ScanOptions::default(),
        &seiri_markdown::DocumentIndexOptions::default(),
        seiri_core::AnalysisScope::Repository,
        &NoCalibrationProvider,
    )
}

pub fn audit_repository_with_scope(
    path: impl AsRef<Path>,
    profile: ProfileKind,
    scope: seiri_core::AnalysisScope,
) -> Result<RepositoryAnalysis, AuditError> {
    audit_repository_with_options_and_calibration(
        path,
        profile,
        &seiri_fs::ScanOptions::default(),
        &seiri_markdown::DocumentIndexOptions::default(),
        scope,
        &NoCalibrationProvider,
    )
}

pub fn audit_repository_with_calibration_provider(
    path: impl AsRef<Path>,
    profile: ProfileKind,
    calibration: &dyn CalibrationProvider,
) -> Result<RepositoryAnalysis, AuditError> {
    audit_repository_with_options_and_calibration(
        path,
        profile,
        &seiri_fs::ScanOptions::default(),
        &seiri_markdown::DocumentIndexOptions::default(),
        seiri_core::AnalysisScope::Repository,
        calibration,
    )
}

pub fn audit_repository_with_local_calibration(
    path: impl AsRef<Path>,
    profile: ProfileKind,
    calibration_path: impl AsRef<Path>,
) -> Result<RepositoryAnalysis, AuditError> {
    let provider = seiri_calibration::load_local_calibration_provider(calibration_path)?;
    audit_repository_with_calibration_provider(path, profile, &provider)
}

pub fn audit_repository_with_local_calibration_and_scope(
    path: impl AsRef<Path>,
    profile: ProfileKind,
    calibration_path: impl AsRef<Path>,
    scope: seiri_core::AnalysisScope,
) -> Result<RepositoryAnalysis, AuditError> {
    let provider = seiri_calibration::load_local_calibration_provider(calibration_path)?;
    audit_repository_with_options_and_calibration(
        path,
        profile,
        &seiri_fs::ScanOptions::default(),
        &seiri_markdown::DocumentIndexOptions::default(),
        scope,
        &provider,
    )
}

pub fn audit_repository_with_remote<T: seiri_remote::RemoteTransport>(
    path: impl AsRef<Path>,
    profile: ProfileKind,
    remote_options: &seiri_remote::RemoteEvidenceOptions,
    transport: &T,
) -> Result<RepositoryAnalysis, AuditError> {
    let mut snapshot = audit_repository_with_profile(path, profile)?;
    snapshot.remote_evidence = seiri_remote::collect_repository_evidence(remote_options, transport);
    snapshot.coverage = snapshot.coverage.with_status(
        seiri_core::CoverageScope::RemoteMetadata,
        snapshot.remote_evidence.coverage,
    )?;
    Ok(snapshot)
}

pub fn audit_repository_with_options(
    path: impl AsRef<Path>,
    profile: ProfileKind,
    fs_options: &seiri_fs::ScanOptions,
    document_options: &seiri_markdown::DocumentIndexOptions,
) -> Result<RepositoryAnalysis, AuditError> {
    audit_repository_with_options_and_calibration(
        path,
        profile,
        fs_options,
        document_options,
        seiri_core::AnalysisScope::Repository,
        &NoCalibrationProvider,
    )
}

pub fn audit_repository_with_options_and_scope(
    path: impl AsRef<Path>,
    profile: ProfileKind,
    fs_options: &seiri_fs::ScanOptions,
    document_options: &seiri_markdown::DocumentIndexOptions,
    scope: seiri_core::AnalysisScope,
) -> Result<RepositoryAnalysis, AuditError> {
    audit_repository_with_options_and_calibration(
        path,
        profile,
        fs_options,
        document_options,
        scope,
        &NoCalibrationProvider,
    )
}

fn audit_repository_with_options_and_calibration(
    path: impl AsRef<Path>,
    profile: ProfileKind,
    fs_options: &seiri_fs::ScanOptions,
    document_options: &seiri_markdown::DocumentIndexOptions,
    analysis_scope: seiri_core::AnalysisScope,
    calibration: &dyn CalibrationProvider,
) -> Result<RepositoryAnalysis, AuditError> {
    let discovered = seiri_git_local::discover_repository(path.as_ref(), analysis_scope)
        .map_err(seiri_git_local::GitLocalError::from)?;
    let fs_scan = seiri_fs::scan_repository_with_options(discovered.analysis_root(), fs_options)?;
    let repository_scope = seiri_git_local::analyze_discovered_repository(
        &discovered,
        &fs_scan.files,
        &fs_scan.ignored_shallow,
        fs_scan.walk_summary.ignored_records_truncated,
        seiri_git_local::RepositoryAnalysisOptions {
            scope: analysis_scope,
            ..seiri_git_local::RepositoryAnalysisOptions::default()
        },
        &seiri_git_local::GixReadBackend,
    );
    let document_index = seiri_markdown::scan_document_index_with_options(
        &fs_scan.repo_root,
        &fs_scan.files,
        fs_scan.walk_summary.completion.is_complete(),
        document_options,
    )?;
    let readme_document = document_index.root_readme_document().cloned();
    let readme_summary = readme_document.as_ref().map(|document| {
        seiri_markdown::summarize_readme_document(document, Some(&fs_scan.repo_root))
    });
    let repo_root = normalize_public_path(&fs_scan.repo_root);
    let mut snapshot = RepositoryAnalysis::new(repo_root);
    let repository_options = seiri_git_local::RepositoryAnalysisOptions {
        scope: analysis_scope,
        ..seiri_git_local::RepositoryAnalysisOptions::default()
    };
    snapshot.analysis_configuration = seiri_core::AnalysisConfiguration {
        schema_version: seiri_core::ANALYSIS_SCHEMA_VERSION.to_string(),
        scope: analysis_scope,
        profile,
        budgets: seiri_core::AnalysisBudgetConfiguration {
            filesystem_max_depth: fs_options.max_depth,
            filesystem_max_entries: fs_options.max_entries,
            filesystem_max_ignored_records: fs_options.max_ignored_records,
            filesystem_additional_ignored_names: fs_options
                .ignore_policy
                .additional_names()
                .to_vec(),
            document_max_documents: document_options.max_documents,
            document_max_total_source_bytes: document_options.max_total_source_bytes,
            document_max_source_bytes: document_options.document.max_source_bytes,
            document_max_events: document_options.document.max_events,
            document_max_diagnostics: document_options.document.max_diagnostics,
            git_max_refs: repository_options.git.max_refs,
            git_max_tags: repository_options.git.max_tags,
            git_max_commit_headers: repository_options.git.max_commit_headers,
            scope: repository_options.graph,
        },
        pattern_registry_fingerprint: seiri_patterns::common_pattern_pack()
            .fingerprint()
            .to_string(),
        visibility: match calibration.visibility() {
            None => seiri_core::AnalysisVisibility::Standard,
            Some(seiri_core::PriorVisibility::PublicSynthetic) => {
                seiri_core::AnalysisVisibility::PublicSyntheticCalibration
            }
            Some(seiri_core::PriorVisibility::LocalOnly) => {
                seiri_core::AnalysisVisibility::LocalPrivateCalibration
            }
            Some(seiri_core::PriorVisibility::Redacted) => {
                seiri_core::AnalysisVisibility::RedactedCalibration
            }
        },
        redacted_calibration_fingerprint: calibration.redacted_fingerprint().map(str::to_owned),
    };
    snapshot.entry_count = fs_scan.files.len();
    snapshot.files = fs_scan.files.clone();
    snapshot.important_files = fs_scan.important_files.clone();
    snapshot.repository_scope = repository_scope;
    snapshot.document_index = document_index;
    snapshot.readme_document = readme_document;
    snapshot.readme_summary = readme_summary;
    snapshot.evidence_kernel = build_evidence_kernel(&fs_scan, &snapshot.document_index)?;
    snapshot.document_index = snapshot
        .document_index
        .clone()
        .with_document_ids(|path| snapshot.evidence_kernel.document_id_for_path(path));
    snapshot.github_local_documents = seiri_github_local::parse_repository_github_documents(
        &fs_scan.repo_root,
        &snapshot.document_index,
    )?;
    snapshot.github_semantics = seiri_core::GithubSemanticsReport::build(
        &snapshot.github_local_documents,
        &snapshot.repository_scope.graph,
        &snapshot.important_files,
    );
    snapshot.coverage = build_coverage_index(
        &fs_scan,
        &snapshot.evidence_kernel,
        &snapshot.document_index,
        &snapshot.github_local_documents,
    )?;
    let baseline = seiri_patterns::evaluate_common_baseline(&snapshot);
    snapshot.pattern_matches = baseline.pattern_matches;
    snapshot.findings = baseline.findings;
    snapshot.baseline = Some(baseline.report);
    snapshot.route_assessments = build_route_assessments(&snapshot)?;
    snapshot.freshness = build_freshness_report(&snapshot);
    snapshot.facets = seiri_profiles::evaluate_facets(&snapshot);
    let route_target_build = build_route_targets(&snapshot);
    snapshot.route_targets = route_target_build.targets;
    snapshot.document_consistency =
        build_document_consistency_report(&snapshot, route_target_build.truncated)?;
    snapshot.route_content = build_route_content(
        &snapshot.evidence_kernel,
        &snapshot.coverage,
        &snapshot.document_index,
        &snapshot.facets,
        &snapshot.document_consistency,
    );
    snapshot.profile =
        seiri_profiles::evaluate_profile_with_calibration(&snapshot, profile, calibration);
    snapshot.missing_route_priority = build_missing_route_priority_report(&snapshot, calibration);
    snapshot.review_priority =
        build_review_priority_report(&snapshot.missing_route_priority, &snapshot.route_content);
    snapshot.claims = build_content_claims(&snapshot);
    Ok(snapshot)
}

fn build_coverage_index(
    fs_scan: &seiri_fs::RepoFsScan,
    kernel: &seiri_core::EvidenceKernel,
    document_index: &seiri_core::DocumentIndex,
    github_local_documents: &seiri_core::GithubLocalDocuments,
) -> Result<seiri_core::CoverageIndex, seiri_core::CoverageIndexError> {
    let repository_status = if fs_scan.walk_summary.completion.is_complete() {
        seiri_core::CoverageStatus::Complete
    } else {
        seiri_core::CoverageStatus::Partial(seiri_core::CoverageIncompleteReason::LimitExceeded)
    };
    let mut entries = vec![
        (
            seiri_core::CoverageScope::RepositoryFiles,
            repository_status,
        ),
        (
            seiri_core::CoverageScope::RootReadme,
            document_index
                .coverage_for_role(seiri_core::DocumentRole::RootReadme)
                .unwrap_or(seiri_core::CoverageStatus::NotRequested),
        ),
        (
            seiri_core::CoverageScope::MarkdownDocuments,
            markdown_coverage_status(document_index, repository_status),
        ),
        (
            seiri_core::CoverageScope::RemoteMetadata,
            seiri_core::CoverageStatus::NotRequested,
        ),
    ];
    for coverage in document_index.role_coverage() {
        entries.push((
            seiri_core::CoverageScope::DocumentRole(coverage.role),
            if coverage.role == seiri_core::DocumentRole::GithubConfiguration {
                github_local_documents.coverage_status(repository_status)
            } else {
                coverage.status
            },
        ));
    }
    for document in document_index.entries() {
        if let Some(document_id) = document
            .document_id
            .or_else(|| kernel.document_id_for_path(&document.path))
        {
            let status = if document.role == seiri_core::DocumentRole::GithubConfiguration {
                github_local_documents.status_for_document(document_id)
            } else if document.scan.is_some() {
                Some(document.status.coverage_status())
            } else {
                None
            };
            if let Some(status) = status {
                entries.push((seiri_core::CoverageScope::Document(document_id), status));
            }
        }
    }
    seiri_core::CoverageIndex::try_new(entries)
}

fn markdown_coverage_status(
    document_index: &seiri_core::DocumentIndex,
    repository_status: seiri_core::CoverageStatus,
) -> seiri_core::CoverageStatus {
    if repository_status != seiri_core::CoverageStatus::Complete {
        return repository_status;
    }
    document_index
        .entries()
        .iter()
        .filter(|entry| entry.is_markdown())
        .map(|entry| entry.status.coverage_status())
        .find(|status| *status != seiri_core::CoverageStatus::Complete)
        .unwrap_or(seiri_core::CoverageStatus::Complete)
}

fn normalize_public_path(path: &Path) -> String {
    let path = path.to_string_lossy().replace('\\', "/");
    if let Some(rest) = path.strip_prefix("//?/UNC/") {
        format!("//{rest}")
    } else if let Some(rest) = path.strip_prefix("//?/") {
        rest.to_string()
    } else {
        path
    }
}

fn build_freshness_report(snapshot: &RepositoryAnalysis) -> seiri_core::FreshnessReport {
    let mut local_present = 0usize;
    let mut local_missing = 0usize;
    let mut non_local_or_unknown = 0usize;
    for assessment in &snapshot.route_assessments {
        let reachability = assessment.readme().target_reachability();
        local_present += reachability.repository_local_present();
        local_missing += reachability.repository_local_missing();
        non_local_or_unknown += reachability.non_local_or_unknown();
    }
    let repository_coverage = snapshot
        .coverage
        .record(seiri_core::CoverageScope::RepositoryFiles)
        .map_or(seiri_core::CoverageStatus::NotRequested, |record| {
            record.status
        });

    let newest = snapshot
        .repository_scope
        .git
        .commits
        .iter()
        .map(|commit| commit.committed_at)
        .max_by_key(|timestamp| timestamp.seconds_since_epoch);
    let oldest = snapshot
        .repository_scope
        .git
        .commits
        .iter()
        .map(|commit| commit.committed_at)
        .min_by_key(|timestamp| timestamp.seconds_since_epoch);

    let lifecycle_assessment = snapshot
        .route_assessments
        .iter()
        .find(|assessment| assessment.route() == RouteKind::Lifecycle);
    let mut lifecycle_evidence = lifecycle_assessment
        .into_iter()
        .flat_map(|assessment| {
            assessment
                .evidence()
                .root_structural()
                .iter()
                .chain(assessment.evidence().readme_routing())
                .chain(assessment.evidence().inherited())
                .copied()
        })
        .collect::<Vec<_>>();
    lifecycle_evidence.sort_unstable();
    lifecycle_evidence.dedup();
    let lifecycle_state =
        lifecycle_assessment.map(|assessment| assessment.summary_projection().state);
    let lifecycle_coverage = snapshot
        .coverage
        .record(seiri_core::CoverageScope::MarkdownDocuments)
        .map_or(seiri_core::CoverageStatus::NotRequested, |record| {
            record.status
        });

    seiri_core::FreshnessReport {
        target_reachability: seiri_core::TargetReachabilityFreshness {
            repository_local_present: local_present,
            repository_local_missing: local_missing,
            non_local_or_unknown,
            coverage: repository_coverage,
        },
        temporal_activity: seiri_core::TemporalActivityFreshness {
            observed_commit_headers: snapshot.repository_scope.git.commits.len(),
            newest,
            oldest,
            coverage: snapshot.repository_scope.git.commits_coverage,
        },
        lifecycle_signal: seiri_core::LifecycleSignalFreshness {
            route_state: lifecycle_state,
            evidence_ids: lifecycle_evidence,
            coverage: lifecycle_coverage,
        },
        ..seiri_core::FreshnessReport::default()
    }
}

pub fn to_json(snapshot: &RepositoryAnalysis) -> Result<String, AuditError> {
    Ok(serde_json::to_string_pretty(&AuditWire::from(snapshot))?)
}

#[derive(Serialize)]
struct AuditWire<'a> {
    schema_version: &'a str,
    tool: &'a str,
    repo_root: &'a str,
    entry_count: usize,
    files: &'a [seiri_core::FileRecord],
    important_files: &'a [seiri_core::ImportantFile],
    analysis_configuration: &'a seiri_core::AnalysisConfiguration,
    document_index: &'a seiri_core::DocumentIndex,
    github_local_documents: &'a seiri_core::GithubLocalDocuments,
    github_semantics: &'a seiri_core::GithubSemanticsReport,
    readme_document: Option<&'a seiri_core::DocumentScan>,
    evidence_kernel: &'a seiri_core::EvidenceKernel,
    coverage: &'a seiri_core::CoverageIndex,
    route_content: &'a seiri_core::RouteContentReport,
    facets: &'a seiri_core::FacetReport,
    document_consistency: &'a seiri_core::DocumentConsistencyReport,
    route_targets: &'a [seiri_core::RouteTargetRef],
    remote_evidence: &'a seiri_core::RemoteEvidenceReport,
    repository_scope: &'a seiri_core::RepositoryScopeReport,
    freshness: &'a seiri_core::FreshnessReport,
    pattern_matches: &'a [seiri_core::PatternMatch],
    route_assessments: &'a [seiri_core::RouteAssessment],
    missing_route_priority: &'a seiri_core::MissingRoutePriorityReport,
    review_priority: &'a seiri_core::ReviewPriorityReport,
    claims: &'a [seiri_core::ContentClaim],
    baseline: Option<&'a seiri_core::BaselineReport>,
    profile: Option<&'a seiri_core::ProfileReport>,
    findings: &'a [seiri_core::Finding],
}

impl<'a> From<&'a RepositoryAnalysis> for AuditWire<'a> {
    fn from(value: &'a RepositoryAnalysis) -> Self {
        Self {
            schema_version: &value.schema_version,
            tool: &value.tool,
            repo_root: &value.repo_root,
            entry_count: value.entry_count,
            files: &value.files,
            important_files: &value.important_files,
            analysis_configuration: &value.analysis_configuration,
            document_index: &value.document_index,
            github_local_documents: &value.github_local_documents,
            github_semantics: &value.github_semantics,
            readme_document: value.readme_document.as_ref(),
            evidence_kernel: &value.evidence_kernel,
            coverage: &value.coverage,
            route_content: &value.route_content,
            facets: &value.facets,
            document_consistency: &value.document_consistency,
            route_targets: &value.route_targets,
            remote_evidence: &value.remote_evidence,
            repository_scope: &value.repository_scope,
            freshness: &value.freshness,
            pattern_matches: &value.pattern_matches,
            route_assessments: &value.route_assessments,
            missing_route_priority: &value.missing_route_priority,
            review_priority: &value.review_priority,
            claims: &value.claims,
            baseline: value.baseline.as_ref(),
            profile: value.profile.as_ref(),
            findings: &value.findings,
        }
    }
}

pub fn pattern_registry_to_json() -> Result<String, AuditError> {
    Ok(seiri_patterns::common_registry_to_json()?)
}

#[must_use]
pub fn pattern_registry_to_markdown() -> String {
    seiri_patterns::render_common_registry_markdown()
}

pub fn plan_repository(path: impl AsRef<Path>) -> Result<PatchPlan, AuditError> {
    plan_repository_with_profile(path, ProfileKind::Common)
}

pub fn plan_repository_with_profile(
    path: impl AsRef<Path>,
    profile: ProfileKind,
) -> Result<PatchPlan, AuditError> {
    let snapshot = audit_repository_with_profile(path, profile)?;
    Ok(seiri_planner::plan_patches(&snapshot))
}

pub fn plan_repository_subtree_with_profile(
    path: impl AsRef<Path>,
    profile: ProfileKind,
) -> Result<PatchPlan, AuditError> {
    let snapshot = audit_repository_subtree_with_profile(path, profile)?;
    Ok(seiri_planner::plan_patches(&snapshot))
}

pub fn plan_repository_with_scope(
    path: impl AsRef<Path>,
    profile: ProfileKind,
    scope: seiri_core::AnalysisScope,
) -> Result<PatchPlan, AuditError> {
    let analysis = audit_repository_with_scope(path, profile, scope)?;
    Ok(seiri_planner::plan_patches(&analysis))
}

pub fn plan_repository_with_local_calibration(
    path: impl AsRef<Path>,
    profile: ProfileKind,
    calibration_path: impl AsRef<Path>,
) -> Result<PatchPlan, AuditError> {
    let snapshot = audit_repository_with_local_calibration(path, profile, calibration_path)?;
    Ok(seiri_planner::plan_patches(&snapshot))
}

pub fn plan_repository_with_local_calibration_and_scope(
    path: impl AsRef<Path>,
    profile: ProfileKind,
    calibration_path: impl AsRef<Path>,
    scope: seiri_core::AnalysisScope,
) -> Result<PatchPlan, AuditError> {
    let analysis =
        audit_repository_with_local_calibration_and_scope(path, profile, calibration_path, scope)?;
    Ok(seiri_planner::plan_patches(&analysis))
}

pub fn plan_to_json(plan: &PatchPlan) -> Result<String, AuditError> {
    Ok(serde_json::to_string_pretty(plan)?)
}

pub fn lint_wording_repository(path: impl AsRef<Path>) -> Result<WordingLintReport, AuditError> {
    lint_wording_repository_with_profile(path, ProfileKind::Common)
}

pub fn lint_wording_repository_with_profile(
    path: impl AsRef<Path>,
    profile: ProfileKind,
) -> Result<WordingLintReport, AuditError> {
    wording::lint_repository_with_profile(path, profile)
}

pub fn wording_lint_to_json(report: &WordingLintReport) -> Result<String, AuditError> {
    Ok(serde_json::to_string_pretty(report)?)
}

#[must_use]
pub fn wording_lint_to_markdown(report: &WordingLintReport) -> String {
    wording::render_markdown(report)
}

pub fn calibrate_dataset_path(path: impl AsRef<Path>) -> Result<CalibrationRun, AuditError> {
    let path = path.as_ref();
    if path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("jsonl"))
    {
        return Ok(seiri_calibration::calibrate_jsonl_path(path)?);
    }
    let dataset = seiri_calibration::load_dataset(path)?;
    Ok(seiri_calibration::calibrate_dataset(&dataset)?)
}

pub fn calibration_to_json(run: &CalibrationRun) -> Result<String, AuditError> {
    Ok(serde_json::to_string_pretty(
        &run.redacted_for_public_output(),
    )?)
}

pub fn codex_query_repository_to_json(
    path: impl AsRef<Path>,
    profile: ProfileKind,
    scope: seiri_core::AnalysisScope,
    query: CodexQueryKind,
) -> Result<String, AuditError> {
    let path = path.as_ref();
    let analysis = audit_repository_with_scope(path, profile, scope)?;
    let plan = seiri_planner::plan_patches(&analysis);
    let wording_lint = if query == CodexQueryKind::Linter {
        Some(wording::lint_repository_with_profile(path, profile)?)
    } else {
        None
    };
    let view = seiri_codex::CodexView::new(&analysis, &plan, wording_lint.as_ref());
    Ok(serde_json::to_string_pretty(&view.query(query))?)
}

pub fn codex_query_repository_to_markdown(
    path: impl AsRef<Path>,
    profile: ProfileKind,
    scope: seiri_core::AnalysisScope,
    query: CodexQueryKind,
) -> Result<String, AuditError> {
    let path = path.as_ref();
    let analysis = audit_repository_with_scope(path, profile, scope)?;
    let plan = seiri_planner::plan_patches(&analysis);
    let wording_lint = if query == CodexQueryKind::Linter {
        Some(wording::lint_repository_with_profile(path, profile)?)
    } else {
        None
    };
    let view = seiri_codex::CodexView::new(&analysis, &plan, wording_lint.as_ref());
    Ok(seiri_codex::render_query_markdown(&view.query(query)))
}

#[must_use]
pub fn calibration_to_markdown(run: &CalibrationRun) -> String {
    let source_visibility = run.source_visibility_summary();
    let run = run.redacted_for_public_output();
    let mut out = String::new();
    out.push_str("# RepoSeiri Calibration Report\n\n");
    out.push_str(&format!("- Schema: `{}`\n", run.schema_version));
    out.push_str(&format!("- Dataset: `{}`\n", run.dataset_id));
    if let Some(pack) = &run.pattern_pack {
        out.push_str(&format!(
            "- Pattern pack: `{}` `{}` / condition `{}` / denominator eligible `{}` excluded `{}`\n",
            pack.id,
            pack.version,
            pack.condition,
            pack.eligible_records,
            pack.excluded_records
        ));
        out.push_str(&format!(
            "- Registry fingerprint: `{}`\n",
            pack.registry_fingerprint
        ));
    }
    out.push_str(&format!("- Records: `{}`\n", run.summary.records));
    out.push_str(&format!("- Sources: `{}`\n", run.summary.sources));
    out.push_str(&format!(
        "- Source visibility: public `{}` / local_only `{}` / redacted `{}`\n",
        source_visibility.public_sources,
        source_visibility.local_only_sources,
        source_visibility.redacted_sources
    ));
    out.push_str(&format!(
        "- Source review status: pending_review `{}` / adopted `{}` / deferred `{}` / rejected `{}`\n",
        source_visibility.pending_review,
        source_visibility.adopted,
        source_visibility.deferred,
        source_visibility.rejected
    ));
    out.push_str(&format!(
        "- Known pattern stats: `{}`\n",
        run.summary.known_pattern_stats
    ));
    out.push_str(&format!(
        "- Route requirements: `{}`\n",
        run.summary.route_requirements
    ));
    out.push_str(&format!(
        "- Profile branches: `{}`\n",
        run.summary.profile_branches
    ));
    out.push_str(&format!(
        "- Pending patterns: `{}`\n",
        run.summary.pending_patterns
    ));
    out.push_str(&format!(
        "- Weight suggestions: `{}`\n",
        run.summary.weight_suggestions
    ));
    out.push_str(&format!(
        "- Aggregation: `{:?}` / identity `{:?}`\n",
        run.resource_trace.aggregation_mode, run.resource_trace.record_identity
    ));
    out.push_str(&format!(
        "- Resource trace: records `{}` / retained records `{}` / retained repository-id entries `{}` / per-pattern repository sets `{}`\n",
        run.resource_trace.records_seen,
        run.resource_trace.retained_records,
        run.resource_trace.retained_repository_id_entries,
        run.resource_trace.per_pattern_repository_sets
    ));
    out.push_str(&format!(
        "- Aggregate slots: patterns `{}` / routes `{}` / profiles `{}` / co-occurrences `{}` / pending `{}` / metadata sources `{}`\n",
        run.resource_trace.known_pattern_slots,
        run.resource_trace.route_slots,
        run.resource_trace.profile_slots,
        run.resource_trace.co_occurrence_slots,
        run.resource_trace.pending_pattern_slots,
        run.resource_trace.metadata_source_slots
    ));
    out.push_str(&format!(
        "- Peak record buffer: bytes `{}` / patterns `{}`\n",
        run.resource_trace.max_buffered_line_bytes, run.resource_trace.max_patterns_per_record
    ));
    let replay_digest = run.resource_trace.replay_digest.map_or_else(
        || "redacted_or_unavailable".to_string(),
        |digest| digest.to_string(),
    );
    out.push_str(&format!("- Replay digest: `{replay_digest}`\n"));
    out.push_str("- Resource boundary: structural retained-state counts and replay digests are diagnostic evidence, not measured memory, throughput, or performance guarantees.\n");
    out.push_str(&format!("- Boundary: {}\n", run.claim_boundary.summary));
    out.push_str(&format!(
        "- Boundary gates: review_required `{}` runtime_rule_adoption `{}` automatic_weight_adoption `{}` guarantees `{}`\n\n",
        run.claim_boundary.review_required,
        run.claim_boundary.runtime_rule_adoption_allowed,
        run.claim_boundary.automatic_weight_adoption_allowed,
        run.claim_boundary.guarantee_allowed
    ));

    out.push_str("## Calibration Sources\n\n");
    if run.sources.is_empty() {
        out.push_str("- No calibration sources recorded.\n\n");
    } else {
        for source in &run.sources {
            out.push_str(&format!(
                "- `{}` kind `{:?}` visibility `{:?}` label `{}` records `{}` scale `{:?}` status `{:?}`\n",
                source.id,
                source.kind,
                source.visibility,
                source.label,
                source.records,
                source.scale,
                source.review_status
            ));
        }
        out.push('\n');
    }

    out.push_str("## Pattern Stats\n\n");
    if run.stats.is_empty() {
        out.push_str("- No known pattern stats generated.\n\n");
    } else {
        for stat in &run.stats {
            out.push_str(&format!("### `{}`\n\n", stat.pattern_id));
            match stat.route {
                Some(route) => out.push_str(&format!("- Route: `{:?}`\n", route)),
                None => out.push_str("- Route: none\n"),
            }
            out.push_str(&format!("- Repositories: `{}`\n", stat.repositories));
            out.push_str(&format!("- Observations: `{}`\n", stat.observations));
            out.push_str(&format!("- Frequency x1000: `{}`\n", stat.frequency_x1000));
            out.push_str(&format!("- Sources: `{}`\n", stat.source_ids.join("`, `")));
            out.push_str(&format!("- Confidence: `{:?}`\n", stat.confidence));
            out.push_str(&format!("- Review status: `{:?}`\n", stat.review_status));
            out.push_str(&format!("- Note: {}\n\n", stat.confidence_note));
        }
    }

    out.push_str("## Route Requirements\n\n");
    if run.route_requirements.is_empty() {
        out.push_str("- No route requirement candidates generated.\n\n");
    } else {
        for requirement in &run.route_requirements {
            out.push_str(&format!(
                "- `{}` route `{:?}` repositories `{}` frequency `{}` requirement `{:?}` priority `{:?}` confidence `{:?}` status `{:?}`\n",
                requirement.id,
                requirement.route,
                requirement.supporting_repositories,
                requirement.frequency_x1000,
                requirement.suggested_requirement,
                requirement.priority,
                requirement.confidence,
                requirement.review_status
            ));
        }
        out.push('\n');
    }

    out.push_str("## Profile Branches\n\n");
    if run.profile_branches.is_empty() {
        out.push_str("- No profile branch candidates generated.\n\n");
    } else {
        for branch in &run.profile_branches {
            out.push_str(&format!(
                "- rank `{}` profile `{}` fit `{}` evidence_match `{}` rank_score `{}` calibration `{:?}`\n",
                branch.rank,
                branch.profile,
                branch.semantics.fit.get(),
                branch.semantics.evidence_match.get(),
                branch.semantics.rank_score.get(),
                branch.semantics.calibration_prior
            ));
        }
        out.push('\n');
    }

    out.push_str("## Pending Pattern Candidates\n\n");
    if run.pending_patterns.is_empty() {
        out.push_str("- No pending pattern candidates.\n\n");
    } else {
        for candidate in &run.pending_patterns {
            out.push_str(&format!(
                "- `{}` `{}` repositories `{}` observations `{}` status `{:?}`\n",
                candidate.id,
                candidate.raw_label,
                candidate.observed_repositories,
                candidate.observations,
                candidate.review_status
            ));
        }
        out.push('\n');
    }

    out.push_str("## Weight Suggestions\n\n");
    if run.weight_suggestions.is_empty() {
        out.push_str("- No weight suggestions generated.\n");
    } else {
        for suggestion in &run.weight_suggestions {
            let current = suggestion
                .current_weight
                .map_or_else(|| "none".to_string(), |weight| weight.to_string());
            let route = suggestion
                .route
                .map_or_else(|| "none".to_string(), |route| format!("{route:?}"));
            out.push_str(&format!(
                "- `{}` `{}` route `{}` profile `{}` current `{}` suggested `{}` delta `{}` confidence `{:?}` status `{:?}`\n",
                suggestion.id,
                suggestion.pattern_id,
                route,
                suggestion.profile,
                current,
                suggestion.suggested_weight,
                suggestion.suggested_delta,
                suggestion.confidence,
                suggestion.review_status
            ));
        }
    }

    out
}

#[must_use]
pub fn plan_to_markdown(plan: &PatchPlan) -> String {
    let mut out = String::from("# RepoSeiri Patch Plan\n\n");
    out.push_str(&format!("- Schema: `{}`\n", plan.schema_version));
    out.push_str(&format!(
        "- Dry-run operations: `{}`\n",
        plan.operations.len()
    ));
    out.push_str(&format!("- Held routes: `{}`\n", plan.held.len()));
    out.push_str(&format!("- Writes files: `{}`\n\n", plan.writes_files));

    if !plan.operations.is_empty() {
        out.push_str("## Operations\n\n");
        for operation in &plan.operations {
            out.push_str(&format!(
                "- `{:?}` -> `{}`; document `{}`; paired language `{}`; proposal `{}`\n",
                operation.route,
                operation.target_path,
                operation.document.ordinal(),
                operation.paired_language,
                operation.proposal.id,
            ));
        }
        out.push('\n');
    }

    if !plan.held.is_empty() {
        out.push_str("## Held\n\n");
        for item in &plan.held {
            let target = item.target_path.as_deref().unwrap_or("none");
            out.push_str(&format!(
                "- `{:?}`: `{:?}`; target `{}`\n",
                item.route, item.reason, target
            ));
        }
        out.push('\n');
    }

    out.push_str("## Boundary\n\n");
    out.push_str(&plan.boundary);
    out.push('\n');
    out
}
#[must_use]
pub fn to_markdown(analysis: &RepositoryAnalysis) -> String {
    let mut out = String::from("# RepoSeiri Audit\n\n");
    out.push_str(&format!("- Schema: `{}`\n", analysis.schema_version));
    out.push_str(&format!("- Repository: `{}`\n", analysis.repo_root));
    out.push_str(&format!(
        "- Analysis scope: `{:?}`\n",
        analysis.analysis_configuration.scope
    ));
    out.push_str(&format!("- Entries: `{}`\n", analysis.entry_count));
    out.push_str(&format!(
        "- Documents: `{}`; evidence facts: `{}`\n",
        analysis.document_index.entries().len(),
        analysis.evidence_kernel.len(),
    ));
    out.push_str(&format!(
        "- Routes: `{}`; content slots: `{}`; findings: `{}`\n",
        analysis.route_assessments.len(),
        analysis.route_content.assessments.len(),
        analysis.findings.len(),
    ));
    out.push_str(&format!("- Content claims: `{}`\n", analysis.claims.len()));
    if let Some(profile) = &analysis.profile {
        out.push_str(&format!(
            "- Profile: `{}`; fit score: `{}` / `100`\n",
            profile.profile, profile.score.score_x100
        ));
    }
    out.push_str(&format!(
        "- Scope nodes: `{}`; Git commit headers: `{}`; GitHub critical paths: `{}`\n\n",
        analysis.repository_scope.graph.nodes.len(),
        analysis.repository_scope.git.commits.len(),
        analysis.github_semantics.critical_paths.len(),
    ));

    out.push_str("## Routes\n\n");
    let claim_refs = ClaimRefIndex::new(&analysis.claims);
    for assessment in &analysis.route_assessments {
        let summary = assessment.summary_projection();
        let claim_ids = claim_refs
            .claim_ids_for_route_state(assessment.route(), summary.state)
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("`, `");
        out.push_str(&format!(
            "- `{:?}`: `{:?}`; root `{}`; README `{}`; inherited `{}`; evidence `{}`; claims `{}`\n",
            assessment.route(),
            summary.state,
            assessment.presence().root_structured(),
            assessment.readme().routing().is_present(),
            assessment.presence().inherited(),
            assessment.summary_evidence_ids().len(),
            claim_ids,
        ));
    }
    out.push('\n');

    out.push_str(&route_content::render_route_content_contract_markdown(
        &analysis.route_content,
    ));
    out.push('\n');

    out.push_str("## Freshness Dimensions\n\n");
    out.push_str(&format!(
        "- Target reachability: present `{}`; missing `{}`; non-local or unknown `{}`; coverage `{:?}`\n",
        analysis.freshness.target_reachability.repository_local_present,
        analysis.freshness.target_reachability.repository_local_missing,
        analysis.freshness.target_reachability.non_local_or_unknown,
        analysis.freshness.target_reachability.coverage,
    ));
    out.push_str(&format!(
        "- Temporal activity: commit headers `{}`; coverage `{:?}`\n",
        analysis.freshness.temporal_activity.observed_commit_headers,
        analysis.freshness.temporal_activity.coverage,
    ));
    out.push_str(&format!(
        "- Lifecycle signal: `{:?}`; evidence `{}`; coverage `{:?}`\n",
        analysis.freshness.lifecycle_signal.route_state,
        analysis.freshness.lifecycle_signal.evidence_ids.len(),
        analysis.freshness.lifecycle_signal.coverage,
    ));
    out.push_str(&format!("- Boundary: {}\n\n", analysis.freshness.boundary));

    out.push_str("## Structured GitHub Semantics\n\n");
    if analysis.github_semantics.critical_paths.is_empty() {
        out.push_str("- No critical paths were observed.\n\n");
    } else {
        for path in &analysis.github_semantics.critical_paths {
            out.push_str(&format!(
                "- `{:?}`: observed `{}`; parsed `{}`; coverage `{:?}`\n",
                path.kind, path.observed, path.parsed, path.coverage
            ));
        }
        out.push_str("- Parsed local workflow documents do not establish workflow success, remote execution, or policy effectiveness.\n\n");
    }

    out.push_str("## Review Priority\n\n");
    if analysis.missing_route_priority.priorities.is_empty() {
        out.push_str("- No missing-route priorities.\n\n");
    } else {
        for priority in &analysis.missing_route_priority.priorities {
            let claim_ids = claim_refs
                .claim_ids_for_route(priority.route)
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("`, `");
            let boundaries = claim_refs
                .boundary_kinds_for_route(priority.route)
                .iter()
                .map(|boundary| format!("{boundary:?}"))
                .collect::<Vec<_>>()
                .join("`, `");
            out.push_str(&format!(
                "- `#{}` `{:?}`: score `{}` / `100`; gate `{:?}`; {} Claim IDs: `{}`. Boundary kinds: `{}`.\n",
                priority.rank,
                priority.route,
                priority.priority_score_x100,
                priority.gate,
                priority.reason,
                claim_ids,
                boundaries,
            ));
        }
        out.push('\n');
    }

    out.push_str("## Content Claims\n\n");
    out.push_str(&format!(
        "- Summary: total `{}`; observed `{}`; inferred `{}`; suggested `{}`; blocked `{}`.\n",
        analysis.claims.len(),
        claim_refs.strength_count(ClaimStrength::Observed),
        claim_refs.strength_count(ClaimStrength::Inferred),
        claim_refs.strength_count(ClaimStrength::Suggested),
        claim_refs.strength_count(ClaimStrength::Blocked),
    ));
    for claim in &analysis.claims {
        let boundaries = claim
            .boundaries
            .iter()
            .map(|boundary| format!("{boundary:?}"))
            .collect::<Vec<_>>()
            .join("`, `");
        out.push_str(&format!(
            "- `{}`: route `{:?}`; state `{:?}`; strength `{:?}`; evidence `{}`. Boundaries: `{}`.\n",
            claim.id,
            claim.route,
            claim.state,
            claim.strength,
            claim.evidence_ids.len(),
            boundaries,
        ));
    }
    out.push('\n');

    out.push_str("## Findings\n\n");
    if analysis.findings.is_empty() {
        out.push_str("- No findings.\n\n");
    } else {
        for finding in &analysis.findings {
            out.push_str(&format!(
                "- `{}` `{:?}`: {}\n",
                finding.id, finding.severity, finding.message
            ));
        }
        out.push('\n');
    }

    out.push_str("## Boundaries\n\n");
    out.push_str("- Absence is emitted only from complete coverage; otherwise the observation remains Unknown.\n");
    out.push_str("- Private calibration content is not included in this report.\n");
    out.push_str("- RepoSeiri does not establish popularity, trust, security, quality, policy adoption, or publication readiness.\n");
    out
}
