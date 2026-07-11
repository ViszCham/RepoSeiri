use seiri_core::{
    BaselineStatus, CalibrationProvider, CalibrationRun, ClaimBoundaryKind, ClaimId, ClaimRefIndex,
    ClaimStrength, CodexLinterContext, CodexNativeReviewContext, CodexQueryKind, CodexQueryView,
    CodexReviewContext, EvidenceId, NoCalibrationProvider, PatchEditContent, PatchPlan,
    PatchProposal, ProfileKind, RepoSnapshot, RouteKind, RouteState, WordingLintReport,
};
use std::fmt::{Display, Formatter};
use std::io;
use std::path::Path;

pub use seiri_codex::{CodexNativeV3QueryKind, CodexNativeV3QueryParseError};

mod claims;
mod evidence;
mod fixture_runner;
mod obligation_graph;
mod route_content;
mod route_priority;
mod wording;

use claims::build_content_claims;
use evidence::{
    build_evidence_kernel, build_route_assessments, legacy_evidence_ledger_view,
    legacy_evidence_view, legacy_route_state_views,
};
pub use fixture_runner::run_executable_pattern_pack;
use obligation_graph::{build_document_consistency_report, build_route_targets};
use route_content::{build_route_content, build_route_content_v2};
use route_priority::{build_missing_route_priority_report, build_review_priority_report};

#[derive(Debug)]
pub enum AuditError {
    Fs(seiri_fs::FsError),
    Markdown(seiri_markdown::MarkdownError),
    Calibration(seiri_calibration::CalibrationError),
    LocalPrior(seiri_calibration::LocalPriorLoadError),
    EvidenceKernel(seiri_core::EvidenceKernelError),
    EvidenceKernelV2(seiri_core::EvidenceKernelV2Error),
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
            Self::EvidenceKernelV2(error) => write!(f, "{error}"),
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
            Self::EvidenceKernelV2(error) => Some(error),
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

impl From<seiri_core::EvidenceKernelV2Error> for AuditError {
    fn from(value: seiri_core::EvidenceKernelV2Error) -> Self {
        Self::EvidenceKernelV2(value)
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
    snapshot: &RepoSnapshot,
) -> Result<seiri_core::PortableAuditSnapshot, AuditError> {
    Ok(seiri_delta::portable_snapshot(snapshot)?)
}

pub fn diff_snapshots(
    before: &RepoSnapshot,
    after: &RepoSnapshot,
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

pub fn audit_repository(path: impl AsRef<Path>) -> Result<RepoSnapshot, AuditError> {
    audit_repository_with_profile(path, ProfileKind::Common)
}

pub fn audit_repository_subtree(path: impl AsRef<Path>) -> Result<RepoSnapshot, AuditError> {
    audit_repository_with_scope(
        path,
        ProfileKind::Common,
        seiri_core::AnalysisScope::Subtree,
    )
}

pub fn audit_repository_subtree_with_profile(
    path: impl AsRef<Path>,
    profile: ProfileKind,
) -> Result<RepoSnapshot, AuditError> {
    audit_repository_with_scope(path, profile, seiri_core::AnalysisScope::Subtree)
}

pub fn audit_repository_with_profile(
    path: impl AsRef<Path>,
    profile: ProfileKind,
) -> Result<RepoSnapshot, AuditError> {
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
) -> Result<RepoSnapshot, AuditError> {
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
) -> Result<RepoSnapshot, AuditError> {
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
) -> Result<RepoSnapshot, AuditError> {
    let provider = seiri_calibration::load_local_calibration_provider(calibration_path)?;
    audit_repository_with_calibration_provider(path, profile, &provider)
}

pub fn audit_repository_with_local_calibration_and_scope(
    path: impl AsRef<Path>,
    profile: ProfileKind,
    calibration_path: impl AsRef<Path>,
    scope: seiri_core::AnalysisScope,
) -> Result<RepoSnapshot, AuditError> {
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
) -> Result<RepoSnapshot, AuditError> {
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
) -> Result<RepoSnapshot, AuditError> {
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
) -> Result<RepoSnapshot, AuditError> {
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
) -> Result<RepoSnapshot, AuditError> {
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
    let readme = readme_document.as_ref().map(|document| {
        seiri_markdown::summarize_readme_document(document, Some(&fs_scan.repo_root))
    });
    let repo_root = normalize_public_path(&fs_scan.repo_root);
    let mut snapshot = RepoSnapshot::new(repo_root);
    let repository_options = seiri_git_local::RepositoryAnalysisOptions {
        scope: analysis_scope,
        ..seiri_git_local::RepositoryAnalysisOptions::default()
    };
    snapshot.analysis_configuration = seiri_core::AnalysisConfiguration {
        schema_version: seiri_core::SCHEMA_VERSION.to_string(),
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
    snapshot.readme = readme;
    snapshot.evidence_kernel = build_evidence_kernel(&fs_scan, &snapshot.document_index)?;
    snapshot.evidence_kernel_v2 =
        seiri_core::EvidenceKernelV2::from_legacy(&snapshot.evidence_kernel)?;
    snapshot.document_index = snapshot
        .document_index
        .clone()
        .with_document_ids(|path| snapshot.evidence_kernel_v2.document_id_for_path(path));
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
        &snapshot.evidence_kernel_v2,
        &snapshot.document_index,
        &snapshot.github_local_documents,
    )?;
    snapshot.route_content = build_route_content(&snapshot.evidence_kernel, &snapshot.coverage);
    snapshot.evidence = legacy_evidence_view(&snapshot.evidence_kernel);
    snapshot.evidence_ledger = legacy_evidence_ledger_view(&snapshot.evidence_kernel);
    let baseline = seiri_patterns::evaluate_common_baseline(&snapshot);
    snapshot.pattern_matches = baseline.pattern_matches;
    snapshot.findings = baseline.findings;
    snapshot.baseline = Some(baseline.report);
    snapshot.route_assessments = build_route_assessments(
        snapshot.evidence_kernel.facts(),
        &snapshot.evidence_kernel_v2,
        &snapshot.pattern_matches,
        snapshot.readme.as_ref(),
    )?;
    snapshot.route_states = legacy_route_state_views(&snapshot.route_assessments);
    snapshot.freshness = build_freshness_report(&snapshot);
    snapshot.facets = seiri_profiles::evaluate_facets(&snapshot);
    let route_target_build = build_route_targets(&snapshot);
    snapshot.route_targets = route_target_build.targets;
    snapshot.document_consistency =
        build_document_consistency_report(&snapshot, route_target_build.truncated)?;
    snapshot.route_content_v2 = build_route_content_v2(
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
        build_review_priority_report(&snapshot.missing_route_priority, &snapshot.route_content_v2);
    snapshot.claims = build_content_claims(&snapshot);
    Ok(snapshot)
}

fn build_coverage_index(
    fs_scan: &seiri_fs::RepoFsScan,
    kernel: &seiri_core::EvidenceKernelV2,
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

fn build_freshness_report(snapshot: &RepoSnapshot) -> seiri_core::FreshnessReport {
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
    let lifecycle_state = snapshot
        .route_states
        .iter()
        .find(|state| state.route == RouteKind::Lifecycle)
        .map(|state| state.state);
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

pub fn to_json(snapshot: &RepoSnapshot) -> Result<String, AuditError> {
    Ok(serde_json::to_string_pretty(snapshot)?)
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

pub fn plan_repository_v5(
    path: impl AsRef<Path>,
    profile: ProfileKind,
    scope: seiri_core::AnalysisScope,
) -> Result<seiri_core::PlannerV5Report, AuditError> {
    let snapshot = audit_repository_with_scope(path, profile, scope)?;
    Ok(seiri_planner::plan_existing_route_links(&snapshot))
}

pub fn plan_repository_v5_with_local_calibration(
    path: impl AsRef<Path>,
    profile: ProfileKind,
    calibration_path: impl AsRef<Path>,
    scope: seiri_core::AnalysisScope,
) -> Result<seiri_core::PlannerV5Report, AuditError> {
    let snapshot =
        audit_repository_with_local_calibration_and_scope(path, profile, calibration_path, scope)?;
    Ok(seiri_planner::plan_existing_route_links(&snapshot))
}

pub fn planner_v5_to_json(report: &seiri_core::PlannerV5Report) -> Result<String, AuditError> {
    Ok(serde_json::to_string_pretty(report)?)
}

#[must_use]
pub fn planner_v5_to_markdown(report: &seiri_core::PlannerV5Report) -> String {
    let mut output = String::from("# RepoSeiri Patch Planner v5\n\n");
    output.push_str(&format!(
        "- Existing-target link operations: {}\n",
        report.operations.len()
    ));
    output.push_str(&format!("- Held routes: {}\n", report.held.len()));
    output.push_str("- Writes files: `false`\n\n");
    if !report.operations.is_empty() {
        output.push_str("## Operations\n\n");
        for operation in &report.operations {
            output.push_str(&format!(
                "- `{:?}` -> `{}` (paired language: `{}`)\n",
                operation.route, operation.target_path, operation.paired_language
            ));
        }
        output.push('\n');
    }
    if !report.held.is_empty() {
        output.push_str("## Held\n\n");
        for item in &report.held {
            output.push_str(&format!("- `{:?}`: `{:?}`\n", item.route, item.reason));
        }
        output.push('\n');
    }
    output.push_str("## Boundary\n\n");
    output.push_str(&report.boundary);
    output.push('\n');
    output
}

pub fn plan_repository_with_profile(
    path: impl AsRef<Path>,
    profile: ProfileKind,
) -> Result<PatchPlan, AuditError> {
    let snapshot = audit_repository_with_profile(path, profile)?;
    Ok(seiri_planner::plan_safe_patches(&snapshot))
}

pub fn plan_repository_subtree_with_profile(
    path: impl AsRef<Path>,
    profile: ProfileKind,
) -> Result<PatchPlan, AuditError> {
    let snapshot = audit_repository_subtree_with_profile(path, profile)?;
    Ok(seiri_planner::plan_safe_patches(&snapshot))
}

pub fn plan_repository_with_local_calibration(
    path: impl AsRef<Path>,
    profile: ProfileKind,
    calibration_path: impl AsRef<Path>,
) -> Result<PatchPlan, AuditError> {
    let snapshot = audit_repository_with_local_calibration(path, profile, calibration_path)?;
    Ok(seiri_planner::plan_safe_patches(&snapshot))
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
    Ok(seiri_calibration::calibrate_dataset(&dataset))
}

pub fn calibration_to_json(run: &CalibrationRun) -> Result<String, AuditError> {
    Ok(serde_json::to_string_pretty(
        &run.redacted_for_public_output(),
    )?)
}

pub fn codex_repository_with_profile(
    path: impl AsRef<Path>,
    profile: ProfileKind,
) -> Result<CodexReviewContext, AuditError> {
    Ok(codex_repository_kernel_with_profile(path, profile)?.compatibility_v1())
}

pub fn codex_repository_subtree_with_profile(
    path: impl AsRef<Path>,
    profile: ProfileKind,
) -> Result<CodexReviewContext, AuditError> {
    let path = path.as_ref();
    let snapshot = audit_repository_subtree_with_profile(path, profile)?;
    let plan = seiri_planner::plan_compatibility_safe_patches(&snapshot);
    let wording_lint = wording::lint_repository_with_profile(path, profile)?;
    Ok(seiri_codex::build_review_kernel(&snapshot, &plan, Some(&wording_lint)).compatibility_v1())
}

pub fn codex_native_repository_with_profile(
    path: impl AsRef<Path>,
    profile: ProfileKind,
) -> Result<CodexNativeReviewContext, AuditError> {
    Ok(codex_repository_kernel_with_profile(path, profile)?.native_v2())
}

pub fn codex_native_v3_query_repository_to_json(
    path: impl AsRef<Path>,
    profile: ProfileKind,
    query: CodexNativeV3QueryKind,
) -> Result<String, AuditError> {
    let path = path.as_ref();
    let snapshot = audit_repository_with_profile(path, profile)?;
    let plan = seiri_planner::plan_safe_patches(&snapshot);
    let wording_lint = if query == CodexNativeV3QueryKind::Linter {
        Some(wording::lint_repository_with_profile(path, profile)?)
    } else {
        None
    };
    let view = seiri_codex::CodexNativeV3View::new(&snapshot, &plan, wording_lint.as_ref());
    Ok(serde_json::to_string_pretty(&view.query(query))?)
}

pub fn codex_native_v3_query_repository_to_markdown(
    path: impl AsRef<Path>,
    profile: ProfileKind,
    query: CodexNativeV3QueryKind,
) -> Result<String, AuditError> {
    let path = path.as_ref();
    let snapshot = audit_repository_with_profile(path, profile)?;
    let plan = seiri_planner::plan_safe_patches(&snapshot);
    let wording_lint = if query == CodexNativeV3QueryKind::Linter {
        Some(wording::lint_repository_with_profile(path, profile)?)
    } else {
        None
    };
    let view = seiri_codex::CodexNativeV3View::new(&snapshot, &plan, wording_lint.as_ref());
    Ok(seiri_codex::render_native_v3_query_markdown(
        &view.query(query),
    ))
}

pub fn codex_query_repository_with_profile(
    path: impl AsRef<Path>,
    profile: ProfileKind,
    query: CodexQueryKind,
) -> Result<CodexQueryView, AuditError> {
    Ok(codex_repository_kernel_with_profile(path, profile)?.query(query))
}

pub fn codex_linter_repository_with_profile(
    path: impl AsRef<Path>,
    profile: ProfileKind,
) -> Result<CodexLinterContext, AuditError> {
    Ok(codex_repository_kernel_with_profile(path, profile)?.linter_context())
}

pub fn codex_repository_kernel_with_profile(
    path: impl AsRef<Path>,
    profile: ProfileKind,
) -> Result<seiri_codex::CodexReviewKernel, AuditError> {
    let path = path.as_ref();
    let snapshot = audit_repository_with_profile(path, profile)?;
    let plan = seiri_planner::plan_compatibility_safe_patches(&snapshot);
    let wording_lint = wording::lint_repository_with_profile(path, profile)?;
    Ok(seiri_codex::build_review_kernel(
        &snapshot,
        &plan,
        Some(&wording_lint),
    ))
}

pub fn codex_to_json(context: &CodexReviewContext) -> Result<String, AuditError> {
    Ok(serde_json::to_string_pretty(context)?)
}

pub fn codex_native_to_json(context: &CodexNativeReviewContext) -> Result<String, AuditError> {
    Ok(serde_json::to_string_pretty(context)?)
}

pub fn codex_query_to_json(view: &CodexQueryView) -> Result<String, AuditError> {
    Ok(serde_json::to_string_pretty(view)?)
}

pub fn codex_linter_context_to_json(context: &CodexLinterContext) -> Result<String, AuditError> {
    Ok(serde_json::to_string_pretty(context)?)
}

pub fn codex_pr_draft_to_json(context: &CodexReviewContext) -> Result<String, AuditError> {
    Ok(serde_json::to_string_pretty(&context.pr_draft)?)
}

#[must_use]
pub fn codex_to_markdown(context: &CodexReviewContext) -> String {
    seiri_codex::render_review_context_markdown(context)
}

#[must_use]
pub fn codex_native_to_markdown(context: &CodexNativeReviewContext) -> String {
    seiri_codex::render_native_context_markdown(context)
}

#[must_use]
pub fn codex_query_to_markdown(view: &CodexQueryView) -> String {
    seiri_codex::render_query_view_markdown(view)
}

#[must_use]
pub fn codex_linter_context_to_markdown(context: &CodexLinterContext) -> String {
    seiri_codex::render_linter_context_markdown(context)
}

#[must_use]
pub fn codex_pr_body_to_markdown(context: &CodexReviewContext) -> String {
    seiri_codex::render_pr_body(context)
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
    let mut out = String::new();
    out.push_str("# RepoSeiri Patch Plan\n\n");
    out.push_str(&format!("- Schema: `{}`\n", plan.schema_version));
    out.push_str(&format!("- Planner: `{}`\n", plan.planner_version));
    out.push_str(&format!("- Mode: `{:?}`\n", plan.mode));
    match plan.profile {
        Some(profile) => out.push_str(&format!("- Profile: `{profile}`\n")),
        None => out.push_str("- Profile: not selected\n"),
    }
    out.push_str(&format!(
        "- Total candidates: `{}`\n",
        plan.summary.total_candidates
    ));
    out.push_str(&format!(
        "- Safe operations: `{}`\n",
        plan.summary.safe_operations
    ));
    out.push_str(&format!(
        "- Safe blocked: `{}`\n",
        plan.summary.safe_blocked
    ));
    out.push_str(&format!(
        "- Guarded items: `{}`\n",
        plan.summary.guarded_items
    ));
    out.push_str(&format!(
        "- Manual items: `{}`\n",
        plan.summary.manual_items
    ));
    out.push_str(&format!(
        "- Preview-only operations: `{}`\n",
        plan.summary.preview_only_operations
    ));
    out.push_str(&format!(
        "- Preflight passed: `{}`\n",
        plan.summary.preflight_passed
    ));
    out.push_str(&format!(
        "- Preflight failed or blocked: `{}`\n",
        plan.summary.preflight_failed
    ));
    out.push_str(&format!(
        "- Safety policy: writes_files `{}` applies_patches `{}` safe_gate_only `{}` existing_targets `{}` unsafe_to_invent_blocked `{}`\n",
        plan.safety_policy.writes_files,
        plan.safety_policy.applies_patches,
        plan.safety_policy.safe_gate_only,
        plan.safety_policy.requires_existing_targets,
        plan.safety_policy.blocks_unsafe_to_invent
    ));
    out.push_str(&format!("- Boundary: {}\n\n", plan.claim_boundary));

    out.push_str("## Safe Fixes\n\n");
    if plan.operations.is_empty() {
        out.push_str("- No safe fixes generated.\n\n");
    } else {
        for operation in &plan.operations {
            out.push_str(&format!("### {}\n\n", operation.id));
            out.push_str(&format!("- Gate: `{:?}`\n", operation.gate));
            out.push_str(&format!("- Kind: `{:?}`\n", operation.kind));
            out.push_str(&format!("- Source: `{:?}`\n", operation.source));
            out.push_str(&format!("- Safety: `{:?}`\n", operation.safety));
            out.push_str(&format!("- Priority: `{:?}`\n", operation.priority));
            out.push_str(&format!("- Preview only: `{}`\n", operation.preview_only));
            out.push_str(&format!(
                "- Requires confirmation: `{}`\n",
                operation.requires_confirmation
            ));
            out.push_str(&format!("- Path: `{}`\n", operation.path));
            out.push_str(&format!("- Pattern: `{}`\n", operation.pattern_id));
            if let Some(finding_id) = &operation.finding_id {
                out.push_str(&format!("- Finding: `{finding_id}`\n"));
            }
            out.push_str(&format!("- Change: {}\n", operation.planned_change));
            out.push_str(&format!("- Rationale: {}\n\n", operation.rationale));
            render_patch_proposal(&mut out, &operation.proposal, "");
            out.push_str("Preflight:\n");
            for check in &operation.preflight {
                out.push_str(&format!(
                    "- `{:?}` `{:?}`: {}\n",
                    check.kind, check.status, check.detail
                ));
            }
            out.push('\n');
            out.push_str("```diff\n");
            for line in &operation.diff_preview {
                out.push_str(line);
                out.push('\n');
            }
            out.push_str("```\n\n");
        }
    }

    out.push_str("## Held or Rejected Safe Proposals\n\n");
    let safe_blocked = plan
        .blocked
        .iter()
        .filter(|item| item.gate == seiri_core::GateKind::Safe)
        .collect::<Vec<_>>();
    if safe_blocked.is_empty() {
        out.push_str("- No held or rejected Safe proposals.\n\n");
    } else {
        for item in safe_blocked {
            out.push_str(&format!(
                "- `{}` `{}`: {}\n",
                item.id, item.pattern_id, item.reason
            ));
            if let Some(proposal) = &item.proposal {
                render_patch_proposal(&mut out, proposal, "  ");
            }
            for check in &item.preflight {
                out.push_str(&format!(
                    "  Preflight `{:?}` `{:?}`: {}\n",
                    check.kind, check.status, check.detail
                ));
            }
        }
        out.push('\n');
    }

    out.push_str("## Guarded Drafts\n\n");
    let guarded_items = plan
        .blocked
        .iter()
        .filter(|item| item.gate == seiri_core::GateKind::Guarded)
        .collect::<Vec<_>>();
    if guarded_items.is_empty() {
        out.push_str("- No guarded drafts.\n\n");
    } else {
        for item in guarded_items {
            out.push_str(&format!(
                "- `{}` `{:?}` `{:?}` `{:?}` `{}`: {}\n",
                item.id, item.gate, item.source, item.safety, item.pattern_id, item.reason
            ));
            if let Some(kind) = item.suggested_kind {
                out.push_str(&format!("  Suggested kind: `{:?}`\n", kind));
            }
            if let Some(route) = item.route {
                out.push_str(&format!("  Route: `{:?}`\n", route));
            }
            for check in &item.preflight {
                out.push_str(&format!(
                    "  Preflight `{:?}` `{:?}`: {}\n",
                    check.kind, check.status, check.detail
                ));
            }
        }
        out.push('\n');
    }

    out.push_str("## Manual Decisions\n\n");
    let manual_items = plan
        .blocked
        .iter()
        .filter(|item| item.gate == seiri_core::GateKind::Manual)
        .collect::<Vec<_>>();
    if manual_items.is_empty() {
        out.push_str("- No manual decisions.\n");
    } else {
        for item in manual_items {
            out.push_str(&format!(
                "- `{}` `{:?}` `{:?}` `{}`: {}\n",
                item.id, item.source, item.safety, item.pattern_id, item.reason
            ));
            if let Some(kind) = item.suggested_kind {
                out.push_str(&format!("  Suggested kind: `{:?}`\n", kind));
            }
            if let Some(route) = item.route {
                out.push_str(&format!("  Route: `{:?}`\n", route));
            }
        }
    }

    out
}

fn render_patch_proposal(out: &mut String, proposal: &PatchProposal, indent: &str) {
    let structural = proposal.preflight_structure();
    out.push_str(&format!("{indent}Patch Proposal IR:\n"));
    out.push_str(&format!(
        "{indent}- Schema: `{}`\n{indent}- Proposal: `{}`\n{indent}- Path: `{}`\n",
        proposal.schema_version, proposal.id, proposal.path
    ));
    out.push_str(&format!(
        "{indent}- Base: `{}` `{:?}` `{:?}` `{}` bytes\n",
        proposal.base.digest(),
        proposal.base.encoding(),
        proposal.base.line_ending(),
        proposal.base.byte_len()
    ));
    out.push_str(&format!(
        "{indent}- Structural decision: `{:?}` with `{}` issue(s)\n",
        structural.decision,
        structural.issues.len()
    ));
    for edit in &proposal.edits {
        let content = match &edit.content {
            PatchEditContent::Literal(_) => "literal".to_string(),
            PatchEditContent::UnresolvedSlot(slot) => {
                format!("unresolved_slot:{:?}", slot.kind)
            }
        };
        out.push_str(&format!(
            "{indent}- Edit `{}`: bytes `{}..{}` content `{content}`\n",
            edit.id, edit.span.byte_start, edit.span.byte_end
        ));
    }
    out.push('\n');
}

#[must_use]
pub fn to_markdown(snapshot: &RepoSnapshot) -> String {
    let mut out = String::new();
    out.push_str("# RepoSeiri Report\n\n");
    out.push_str(&format!("- Schema: `{}`\n", snapshot.schema_version));
    out.push_str(&format!("- Repository: `{}`\n", snapshot.repo_root));
    out.push_str(&format!("- Entries scanned: `{}`\n", snapshot.entry_count));
    out.push_str(&format!(
        "- Pattern matches: `{}`\n",
        snapshot.pattern_matches.len()
    ));
    if let Some(baseline) = &snapshot.baseline {
        out.push_str(&format!(
            "- Baseline required: `{}` present / `{}` missing\n",
            baseline.summary.required_present, baseline.summary.required_missing
        ));
        out.push_str(&format!(
            "- Baseline optional: `{}` present / `{}` missing\n",
            baseline.summary.optional_present, baseline.summary.optional_missing
        ));
    }
    if let Some(profile) = &snapshot.profile {
        out.push_str(&format!(
            "- Profile fit: `{}` / `100` for `{}`\n",
            profile.score.score_x100, profile.profile
        ));
        if let (Some(top_profile), Some(confidence)) = (
            profile.branch_summary.top_profile,
            profile.branch_summary.top_confidence_x100,
        ) {
            out.push_str(&format!(
                "- Profile branch top: `{}` rank_score `{}` / `100` across `{}` candidates\n",
                top_profile, confidence, profile.branch_summary.emitted_profiles
            ));
        }
    }
    out.push_str(&format!(
        "- Document events: `{}` / diagnostics `{}`\n",
        snapshot
            .readme_document
            .as_ref()
            .map_or(0, |document| document.events().len()),
        snapshot
            .readme_document
            .as_ref()
            .map_or(0, |document| document.diagnostics().len())
    ));
    out.push_str(&format!(
        "- Evidence kernel facts: `{}`\n",
        snapshot.evidence_kernel.len()
    ));
    out.push_str(&format!(
        "- Evidence items: `{}`\n",
        snapshot.evidence.len()
    ));
    out.push_str(&format!(
        "- Evidence ledger records: `{}`\n",
        snapshot.evidence_ledger.len()
    ));
    out.push_str(&format!("- Content claims: `{}`\n", snapshot.claims.len()));
    out.push_str(&format!(
        "- Route assessments: `{}`\n",
        snapshot.route_assessments.len()
    ));
    out.push_str(&format!(
        "- Route states: `{}`\n",
        snapshot.route_states.len()
    ));
    let (strong_routes, weak_routes, missing_routes) = route_strength_counts(snapshot);
    out.push_str(&format!(
        "- Route review: strong `{strong_routes}` / weak `{weak_routes}` / missing `{missing_routes}`\n"
    ));
    if let (Some(route), Some(priority)) = (
        snapshot.missing_route_priority.summary.top_route,
        snapshot.missing_route_priority.summary.top_priority_x100,
    ) {
        out.push_str(&format!(
            "- Missing route top: `{:?}` priority `{}` / `100` across `{}` candidates\n",
            route, priority, snapshot.missing_route_priority.summary.candidates
        ));
    }
    out.push_str(&format!(
        "- Co-occurrence gaps: `{}`\n",
        snapshot.missing_route_priority.summary.co_occurrence_gaps
    ));
    out.push_str(&format!("- Findings: `{}`\n\n", snapshot.findings.len()));

    render_repository_scope(&mut out, snapshot);
    render_github_semantics(&mut out, snapshot);

    out.push_str(&route_content::render_route_content_contract_markdown(
        &snapshot.route_content_v2,
    ));
    out.push('\n');

    out.push_str("## Route Review v2\n\n");
    if snapshot.route_states.is_empty() {
        out.push_str("- No route states emitted.\n\n");
    } else {
        out.push_str("### Strong Routes\n\n");
        render_route_strength_group(&mut out, snapshot, RouteStrength::Strong);
        out.push_str("### Weak Routes\n\n");
        render_route_strength_group(&mut out, snapshot, RouteStrength::Weak);
        out.push_str("### Missing Routes\n\n");
        render_route_strength_group(&mut out, snapshot, RouteStrength::Missing);
    }
    out.push_str(&format!(
        "### Decision Gates\n\n- Missing route priorities: safe `{}` / guarded `{}` / manual `{}`\n\n",
        snapshot.missing_route_priority.summary.safe_gated,
        snapshot.missing_route_priority.summary.guarded_gated,
        snapshot.missing_route_priority.summary.manual_gated
    ));

    render_content_claims(&mut out, snapshot);

    out.push_str("## README\n\n");
    match &snapshot.readme {
        Some(readme) => {
            out.push_str(&format!("- Path: `{}`\n", readme.path));
            out.push_str(&format!("- Headings: `{}`\n", readme.headings.len()));
            out.push_str(&format!("- Links: `{}`\n", readme.links.len()));
            out.push_str(&format!("- Badges: `{}`\n", readme.badges.len()));
            out.push_str(&format!(
                "- Route candidates: `{}`\n\n",
                readme.route_candidates.len()
            ));
            out.push_str(&format!(
                "- Route map: `{}` routed / `{}` weak / `{}` conflicting / `{}` overloaded / `{}` stale / `{}` absent\n\n",
                readme.route_map.summary.routed,
                readme.route_map.summary.weak,
                readme.route_map.summary.conflicting,
                readme.route_map.summary.overloaded,
                readme.route_map.summary.stale,
                readme.route_map.summary.absent
            ));
        }
        None => out.push_str("- Path: not found\n\n"),
    }

    out.push_str("## README Route Map\n\n");
    match &snapshot.readme {
        Some(readme) => {
            for entry in &readme.route_map.entries {
                let gap = entry.gap_estimate.map_or_else(
                    || "n/a".to_string(),
                    |estimate| {
                        format!(
                            "{} / {}",
                            estimate.estimated_repositories, estimate.denominator
                        )
                    },
                );
                let targets = if entry.targets.is_empty() {
                    "none".to_string()
                } else {
                    entry
                        .targets
                        .iter()
                        .map(|target| {
                            format!(
                                "{} ({:?}, line {})",
                                target.target, target.status, target.line
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("; ")
                };
                out.push_str(&format!(
                    "- `{:?}` `{:?}` candidates `{}` targets `{}` stale `{}` conflicts `{}` estimated_gap `{}`: {}\n",
                    entry.route,
                    entry.state,
                    entry.candidate_count,
                    entry.target_count,
                    entry.stale_target_count,
                    entry.conflicting_target_count,
                    gap,
                    entry.reason
                ));
                out.push_str(&format!("  Targets: {targets}\n"));
            }
            out.push('\n');
        }
        None => out.push_str("- README was not found, so no route map was emitted.\n\n"),
    }

    out.push_str("## Important Files\n\n");
    if snapshot.important_files.is_empty() {
        out.push_str("- None detected\n\n");
    } else {
        for important in &snapshot.important_files {
            out.push_str(&format!("- `{:?}`: `{}`\n", important.kind, important.path));
        }
        out.push('\n');
    }

    out.push_str("## Common Baseline\n\n");
    match &snapshot.baseline {
        Some(baseline) => {
            out.push_str(&format!("- Profile: `{:?}`\n", baseline.profile));
            out.push_str(&format!(
                "- Required present: `{}`\n",
                baseline.summary.required_present
            ));
            out.push_str(&format!(
                "- Required missing: `{}`\n",
                baseline.summary.required_missing
            ));
            out.push_str(&format!(
                "- Optional present: `{}`\n",
                baseline.summary.optional_present
            ));
            out.push_str(&format!(
                "- Optional missing: `{}`\n\n",
                baseline.summary.optional_missing
            ));
            for rule in &baseline.rules {
                let status_marker = match rule.status {
                    BaselineStatus::Present => "present",
                    BaselineStatus::Missing => "missing",
                };
                out.push_str(&format!(
                    "- `{}` `{}` {:?}: {}\n",
                    rule.rule_id, status_marker, rule.requirement, rule.title
                ));
            }
            out.push('\n');
        }
        None => out.push_str("- Baseline was not evaluated.\n\n"),
    }

    out.push_str("## Route States\n\n");
    if snapshot.route_states.is_empty() {
        out.push_str("- No route states emitted.\n\n");
    } else {
        let claim_index = ClaimRefIndex::new(&snapshot.claims);
        for state in &snapshot.route_states {
            let evidence = if state.evidence_ids.is_empty() {
                "none".to_string()
            } else {
                join_evidence_ids(&state.evidence_ids)
            };
            let claim_ids = claim_index.claim_ids_for_route_state(state.route, state.state);
            out.push_str(&format!(
                "- `{:?}` `{:?}` confidence `{:?}` evidence `{}` claims {}: {}\n",
                state.route,
                state.state,
                state.confidence,
                evidence,
                claim_ids_or_none(&claim_ids),
                state.reason
            ));
        }
        out.push('\n');
    }

    out.push_str("## Route Assessment Axes\n\n");
    if snapshot.route_assessments.is_empty() {
        out.push_str("- No route assessments emitted.\n\n");
    } else {
        for assessment in &snapshot.route_assessments {
            let projection = assessment.legacy_projection();
            out.push_str(&format!(
                "- `{:?}` presence root `{}` inherited `{}` / README candidates `{}` targets `{}` / local present `{}` missing `{}` external `{}` anchor `{}` mail `{}` unknown `{}` / conflicts `{}` / freshness `{:?}` / legacy `{:?}`\n",
                assessment.route(),
                assessment.presence().root_structured(),
                assessment.presence().inherited(),
                assessment.readme().routing().candidate_count(),
                assessment.readme().routing().target_count(),
                assessment
                    .readme()
                    .target_reachability()
                    .repository_local_present(),
                assessment
                    .readme()
                    .target_reachability()
                    .repository_local_missing(),
                assessment.readme().target_reachability().external(),
                assessment.readme().target_reachability().anchor(),
                assessment.readme().target_reachability().mail(),
                assessment.readme().target_reachability().unknown(),
                assessment.readme().conflict().shared_target_count(),
                assessment.readme().freshness(),
                projection.state
            ));
        }
        out.push('\n');
    }

    out.push_str("## Missing Route Priority\n\n");
    out.push_str(&format!(
        "- Boundary: {}\n",
        snapshot.missing_route_priority.boundary
    ));
    out.push_str(&format!(
        "- Gates: safe `{}` / guarded `{}` / manual `{}`\n\n",
        snapshot.missing_route_priority.summary.safe_gated,
        snapshot.missing_route_priority.summary.guarded_gated,
        snapshot.missing_route_priority.summary.manual_gated
    ));
    if snapshot.missing_route_priority.priorities.is_empty() {
        out.push_str("- No missing or degraded route priorities emitted.\n\n");
    } else {
        out.push_str("### Route Priorities\n\n");
        let claim_index = ClaimRefIndex::new(&snapshot.claims);
        for priority in &snapshot.missing_route_priority.priorities {
            let score = decimal_confidence(priority.priority_score_x100);
            let estimate = priority.calibration_estimate.map_or_else(
                || "n/a".to_string(),
                |estimate| {
                    format!(
                        "{} / {}",
                        estimate.estimated_repositories, estimate.denominator
                    )
                },
            );
            let baseline = list_or_none(&priority.baseline_pattern_ids);
            let candidate = list_or_none(&priority.candidate_pattern_ids);
            let gaps = list_or_none(&priority.co_occurrence_gap_ids);
            let claim_ids = claim_index.claim_ids_for_route(priority.route);
            let boundary_kinds = claim_index.boundary_kinds_for_route(priority.route);
            out.push_str(&format!(
                "{}. `{:?}` `{:?}` priority `{}` gate `{:?}` estimated_missing `{}`: {}\n",
                priority.rank,
                priority.route,
                priority.priority,
                score,
                priority.gate,
                estimate,
                priority.reason
            ));
            out.push_str(&format!("   Baseline: {baseline}\n"));
            out.push_str(&format!("   Candidates: {candidate}\n"));
            out.push_str(&format!("   Co-occurrence: {gaps}\n"));
            out.push_str(&format!(
                "   Claim IDs: {}\n",
                claim_ids_or_none(&claim_ids)
            ));
            out.push_str(&format!(
                "   Boundary kinds: {}\n",
                boundary_kinds_or_none(&boundary_kinds)
            ));
        }
        out.push('\n');
    }
    if snapshot
        .missing_route_priority
        .co_occurrence_gaps
        .is_empty()
    {
        out.push_str("### Co-occurrence Gaps\n\n- No co-occurrence gaps emitted.\n\n");
    } else {
        out.push_str("### Co-occurrence Gaps\n\n");
        for gap in &snapshot.missing_route_priority.co_occurrence_gaps {
            let present_routes = routes_or_none(&gap.present_routes);
            let missing_routes = routes_or_none(&gap.missing_routes);
            let present_signals = list_or_none(&gap.present_signals);
            let missing_signals = list_or_none(&gap.missing_signals);
            out.push_str(&format!(
                "- `{}` {:?} calibration `{:?}` values `redacted` gate `{:?}`: {}\n",
                gap.id, gap.priority, gap.calibration_prior, gap.gate, gap.title
            ));
            out.push_str(&format!("  Present routes: {present_routes}\n"));
            out.push_str(&format!("  Missing routes: {missing_routes}\n"));
            out.push_str(&format!("  Present signals: {present_signals}\n"));
            out.push_str(&format!("  Missing signals: {missing_signals}\n"));
            out.push_str(&format!("  Reason: {}\n", gap.reason));
        }
        out.push('\n');
    }

    out.push_str("## Profile\n\n");
    match &snapshot.profile {
        Some(profile) => {
            out.push_str(&format!("- Selected profile: `{}`\n", profile.profile));
            out.push_str(&format!(
                "- Profile fit view: `{}` / `100`\n",
                profile.score.score_x100
            ));
            out.push_str(&format!(
                "- Weight: `{}` earned / `{}` total\n",
                profile.score.earned_weight, profile.score.total_weight
            ));
            out.push_str(&format!(
                "- Rules: `{}` present / `{}` missing\n",
                profile.score.present_rules, profile.score.missing_rules
            ));
            out.push_str(&format!("- Note: {}\n\n", profile.score.note));
            out.push_str("### Profile Branch Semantics\n\n");
            out.push_str(&format!(
                "- Ambiguous: `{}`\n",
                profile.branch_summary.ambiguous
            ));
            out.push_str(&format!(
                "- Boundary: {}\n\n",
                profile.branch_summary.boundary
            ));
            for branch in &profile.branches {
                let matched = if branch.matched_signals.is_empty() {
                    "none".to_string()
                } else {
                    branch.matched_signals.join("; ")
                };
                out.push_str(&format!(
                    "{}. `{}` rank_score `{}` fit `{}` evidence_match `{}` calibration `{:?}`: {}\n",
                    branch.rank,
                    branch.profile,
                    decimal_confidence(branch.semantics.rank_score.get()),
                    branch.semantics.fit.get(),
                    branch.semantics.evidence_match.get(),
                    branch.semantics.calibration_prior,
                    matched
                ));
            }
            out.push('\n');
            if profile.recommendations.is_empty() {
                out.push_str("- No profile recommendations.\n\n");
            } else {
                out.push_str("### Profile Recommendation Order\n\n");
                for recommendation in &profile.recommendations {
                    out.push_str(&format!(
                        "{}. `{}` {:?} weight `{}`: {}\n",
                        recommendation.rank,
                        recommendation.pattern_id,
                        recommendation.priority,
                        recommendation.weight,
                        recommendation.title
                    ));
                }
                out.push('\n');
            }
        }
        None => out.push_str("- Profile was not evaluated.\n\n"),
    }

    out.push_str("## Pattern Matches\n\n");
    if snapshot.pattern_matches.is_empty() {
        out.push_str("- No pattern matches emitted.\n\n");
    } else {
        for pattern_match in &snapshot.pattern_matches {
            out.push_str(&format!(
                "- `{}` {:?}: {}\n",
                pattern_match.pattern_id, pattern_match.outcome, pattern_match.title
            ));
        }
        out.push('\n');
    }

    out.push_str("## Findings\n\n");
    if snapshot.findings.is_empty() {
        out.push_str("- No findings emitted by the common baseline.\n");
    } else {
        for finding in &snapshot.findings {
            out.push_str(&format!("### {}\n\n", finding.id));
            out.push_str(&format!("- Severity: `{:?}`\n", finding.severity));
            out.push_str(&format!("- Title: {}\n", finding.title));
            out.push_str(&format!("- Message: {}\n", finding.message));
            if !finding.evidence_ids.is_empty() {
                out.push_str(&format!(
                    "- Evidence: `{}`\n",
                    join_evidence_ids(&finding.evidence_ids)
                ));
            }
            if let Some(recommendation) = &finding.recommendation {
                out.push_str(&format!("- Gate: `{:?}`\n", recommendation.gate));
                out.push_str(&format!("- Recommendation: {}\n", recommendation.message));
            }
            out.push('\n');
        }
    }

    out
}

fn render_repository_scope(out: &mut String, snapshot: &RepoSnapshot) {
    let scope = &snapshot.repository_scope;
    out.push_str("## Repository Scope Graph\n\n");
    out.push_str(&format!(
        "- Analysis scope: `{:?}`\n- Root kind: `{:?}`\n- Scope nodes: `{}` / edges `{}`\n- Workspace manifests: `{}`\n- Ignored shallow records: `{}`\n- Local Git refs: `{}` / commit headers `{}`\n",
        scope.root.scope,
        scope.root.kind,
        scope.graph.nodes.len(),
        scope.graph.edges.len(),
        scope.graph.manifests.len(),
        scope.graph.ignored.len(),
        scope.git.references.len(),
        scope.git.commits.len(),
    ));
    out.push_str(&format!(
        "- Scope coverage: nodes `{:?}` / manifests `{:?}` / ignored `{:?}`\n- Git coverage: refs `{:?}` / tags `{:?}` / commits `{:?}`\n- Boundary: {}\n\n",
        scope.graph.node_coverage,
        scope.graph.manifest_coverage,
        scope.graph.ignored_coverage,
        scope.git.refs_coverage,
        scope.git.tags_coverage,
        scope.git.commits_coverage,
        scope.graph.boundary,
    ));
    out.push_str("## Freshness Dimensions\n\n");
    out.push_str(&format!(
        "- Target reachability: local present `{}` / local missing `{}` / non-local-or-unknown `{}` / coverage `{:?}`\n- Temporal activity: commit headers `{}` / newest `{:?}` / oldest `{:?}` / coverage `{:?}`\n- Lifecycle signal: state `{:?}` / evidence `{}` / coverage `{:?}`\n- Boundary: {}\n\n",
        snapshot.freshness.target_reachability.repository_local_present,
        snapshot.freshness.target_reachability.repository_local_missing,
        snapshot.freshness.target_reachability.non_local_or_unknown,
        snapshot.freshness.target_reachability.coverage,
        snapshot.freshness.temporal_activity.observed_commit_headers,
        snapshot.freshness.temporal_activity.newest,
        snapshot.freshness.temporal_activity.oldest,
        snapshot.freshness.temporal_activity.coverage,
        snapshot.freshness.lifecycle_signal.route_state,
        snapshot.freshness.lifecycle_signal.evidence_ids.len(),
        snapshot.freshness.lifecycle_signal.coverage,
        snapshot.freshness.boundary,
    ));
}

fn render_github_semantics(out: &mut String, snapshot: &RepoSnapshot) {
    out.push_str("## Structured GitHub Semantics v2\n\n");
    out.push_str(&format!(
        "- Local configuration documents: `{}`\n- Critical-path coverage records: `{}`\n",
        snapshot.github_local_documents.documents().len(),
        snapshot.github_semantics.critical_paths.len()
    ));
    for coverage in &snapshot.github_semantics.critical_paths {
        out.push_str(&format!(
            "- `{:?}` at scope `{}`: observed `{}` / parsed `{}` / coverage `{:?}`\n",
            coverage.kind,
            coverage.scope_node.0,
            coverage.observed,
            coverage.parsed,
            coverage.coverage
        ));
    }
    out.push_str(
        "- Boundary: static syntax observations do not establish workflow success, owner validity, repository permissions, branch protection, or deployment safety.\n\n",
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RouteStrength {
    Strong,
    Weak,
    Missing,
}

fn route_strength_counts(snapshot: &RepoSnapshot) -> (usize, usize, usize) {
    let strong = snapshot
        .route_states
        .iter()
        .filter(|state| route_strength(state.state) == RouteStrength::Strong)
        .count();
    let weak = snapshot
        .route_states
        .iter()
        .filter(|state| route_strength(state.state) == RouteStrength::Weak)
        .count();
    let missing = snapshot
        .route_states
        .iter()
        .filter(|state| route_strength(state.state) == RouteStrength::Missing)
        .count();
    (strong, weak, missing)
}

fn render_route_strength_group(out: &mut String, snapshot: &RepoSnapshot, strength: RouteStrength) {
    let mut emitted = 0;
    let claim_index = ClaimRefIndex::new(&snapshot.claims);
    for state in &snapshot.route_states {
        if route_strength(state.state) != strength {
            continue;
        }
        emitted += 1;
        let priority = snapshot
            .missing_route_priority
            .priorities
            .iter()
            .find(|priority| priority.route == state.route);
        let score = priority.map_or_else(
            || "n/a".to_string(),
            |priority| priority.priority_score_x100.to_string(),
        );
        let gate = priority.map_or_else(
            || "n/a".to_string(),
            |priority| format!("{:?}", priority.gate),
        );
        let claim_ids = claim_index.claim_ids_for_route_state(state.route, state.state);
        let boundary_kinds = claim_index.boundary_kinds_for_claim_ids(&claim_ids);
        out.push_str(&format!(
            "- `{:?}` `{:?}` confidence `{:?}` priority `{}` gate `{}` claims {}: {}\n",
            state.route,
            state.state,
            state.confidence,
            score,
            gate,
            claim_ids_or_none(&claim_ids),
            state.reason
        ));
        out.push_str(&format!(
            "  Boundary kinds: {}\n",
            boundary_kinds_or_none(&boundary_kinds)
        ));
    }
    if emitted == 0 {
        out.push_str("- None.\n");
    }
    out.push('\n');
}

fn render_content_claims(out: &mut String, snapshot: &RepoSnapshot) {
    out.push_str("## Content Claims\n\n");
    if snapshot.claims.is_empty() {
        out.push_str("- No evidence-linked content claims were generated.\n\n");
        return;
    }

    let claim_index = ClaimRefIndex::new(&snapshot.claims);
    let observed = claim_index.strength_count(ClaimStrength::Observed);
    let inferred = claim_index.strength_count(ClaimStrength::Inferred);
    let suggested = claim_index.strength_count(ClaimStrength::Suggested);
    let blocked = claim_index.strength_count(ClaimStrength::Blocked);
    let boundary_kinds = claim_index.boundary_kinds();
    out.push_str(&format!(
        "- Summary: total `{}` / observed `{observed}` / inferred `{inferred}` / suggested `{suggested}` / blocked `{blocked}`\n",
        snapshot.claims.len()
    ));
    out.push_str(&format!(
        "- Boundary kinds: {}\n\n",
        boundary_kinds_or_none(&boundary_kinds)
    ));

    for claim in &snapshot.claims {
        out.push_str(&format!(
            "- `{}` `{:?}` route `{:?}` state `{:?}` evidence `{}`\n",
            claim.id,
            claim.strength,
            claim.route,
            claim.state,
            join_evidence_ids(&claim.evidence_ids)
        ));
        out.push_str(&format!(
            "  Allows: {}\n",
            debug_values_or_none(&claim.allowed_meanings)
        ));
        out.push_str(&format!(
            "  Boundaries: {}\n",
            debug_values_or_none(&claim.boundaries)
        ));
    }
    out.push('\n');
}

fn claim_ids_or_none(ids: &[ClaimId]) -> String {
    if ids.is_empty() {
        "none".to_string()
    } else {
        ids.iter()
            .map(|id| format!("`{id}`"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn join_evidence_ids(ids: &[EvidenceId]) -> String {
    ids.iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("`, `")
}

fn boundary_kinds_or_none(boundaries: &[ClaimBoundaryKind]) -> String {
    if boundaries.is_empty() {
        "none".to_string()
    } else {
        boundaries
            .iter()
            .map(|boundary| format!("`{boundary:?}`"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn route_strength(state: RouteState) -> RouteStrength {
    match state {
        RouteState::Routed
        | RouteState::Structured
        | RouteState::Verified
        | RouteState::Overridden => RouteStrength::Strong,
        RouteState::Absent | RouteState::UnsafeToInvent => RouteStrength::Missing,
        RouteState::Implicit
        | RouteState::Weak
        | RouteState::Inherited
        | RouteState::Conflicting
        | RouteState::Overloaded
        | RouteState::Stale => RouteStrength::Weak,
    }
}

fn decimal_confidence(value_x100: u8) -> String {
    if value_x100 >= 100 {
        "1.00".to_string()
    } else {
        format!("0.{value_x100:02}")
    }
}

fn list_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values.join(", ")
    }
}

fn routes_or_none(values: &[RouteKind]) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values
            .iter()
            .map(|route| format!("{route:?}"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn debug_values_or_none<T: std::fmt::Debug>(values: &[T]) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values
            .iter()
            .map(|value| format!("{value:?}"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}
