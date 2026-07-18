use crate::{audit_repository_subtree, lint_wording_repository_with_profile, AuditError};
use seiri_core::{LocalSupportInterval, ProfileKind, RouteKind, RouteState};
use seiri_digest::{Digest32, StableHasher};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::{Read, Take};
use std::path::{Component, Path, PathBuf};
use std::time::Instant;

pub const HOLDOUT_CORPUS_SCHEMA_VERSION: &str = "seiri.calibration-corpus.v1";
pub const HOLDOUT_REPORT_SCHEMA_VERSION: &str = "seiri.calibration-holdout.v1";
pub const MINIMUM_HOLDOUT_CASES_PER_TASK: usize = 20;
pub const HOLDOUT_SPLIT_METHOD: &str = "fixed_case_ids_no_random_reshuffle_disjoint_task_fixtures";

const MAX_CORPUS_BYTES: u64 = 1024 * 1024;
const MAX_CASES: usize = 512;
const CORPUS_DIGEST_DOMAIN: &[u8] = b"seiri.calibration-corpus.public.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationTask {
    Route,
    Wording,
    Consistency,
    Profile,
    Planner,
}

impl CalibrationTask {
    pub const ALL: [Self; 5] = [
        Self::Route,
        Self::Wording,
        Self::Consistency,
        Self::Profile,
        Self::Planner,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CalibrationSplit {
    Train,
    Holdout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorpusVisibility {
    PublicSynthetic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CalibrationCorpus {
    schema_version: String,
    corpus_id: String,
    visibility: CorpusVisibility,
    split_method: String,
    cases: Vec<CalibrationCase>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CalibrationCase {
    id: String,
    split: CalibrationSplit,
    fixture: RelativeFixturePath,
    expectation: CalibrationExpectation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "task", rename_all = "snake_case")]
enum CalibrationExpectation {
    Route {
        route: RouteKind,
        expected_present: bool,
    },
    Wording {
        expected_finding: bool,
    },
    Consistency {
        expected_conflict: bool,
    },
    Profile {
        profile: ProfileKind,
        expected_top: bool,
    },
    Planner {
        expected_operation: bool,
    },
}

impl CalibrationExpectation {
    const fn task(&self) -> CalibrationTask {
        match self {
            Self::Route { .. } => CalibrationTask::Route,
            Self::Wording { .. } => CalibrationTask::Wording,
            Self::Consistency { .. } => CalibrationTask::Consistency,
            Self::Profile { .. } => CalibrationTask::Profile,
            Self::Planner { .. } => CalibrationTask::Planner,
        }
    }

    const fn expected(&self) -> bool {
        match self {
            Self::Route {
                expected_present, ..
            } => *expected_present,
            Self::Wording { expected_finding } => *expected_finding,
            Self::Consistency { expected_conflict } => *expected_conflict,
            Self::Profile { expected_top, .. } => *expected_top,
            Self::Planner { expected_operation } => *expected_operation,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RelativeFixturePath(Box<str>);

impl RelativeFixturePath {
    fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for RelativeFixturePath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        let path = Path::new(&value);
        if value.is_empty()
            || value.len() > 512
            || path.is_absolute()
            || path.components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            return Err(serde::de::Error::custom(
                "fixture path must be a bounded repository-relative path",
            ));
        }
        Ok(Self(value.into_boxed_str()))
    }
}

impl Serialize for RelativeFixturePath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfusionCounts {
    pub true_positive: usize,
    pub true_negative: usize,
    pub false_positive: usize,
    pub false_negative: usize,
}

impl ConfusionCounts {
    fn record(&mut self, expected: bool, actual: bool) {
        match (expected, actual) {
            (true, true) => self.true_positive += 1,
            (false, false) => self.true_negative += 1,
            (false, true) => self.false_positive += 1,
            (true, false) => self.false_negative += 1,
        }
    }

    #[must_use]
    pub const fn samples(self) -> usize {
        self.true_positive + self.true_negative + self.false_positive + self.false_negative
    }

    const fn correct(self) -> usize {
        self.true_positive + self.true_negative
    }

    const fn positives(self) -> usize {
        self.true_positive + self.false_negative
    }

    const fn negatives(self) -> usize {
        self.true_negative + self.false_positive
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmpiricalCalibrationStatus {
    InsufficientSample,
    Calibrated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PeakAllocationMeasurement {
    NotMeasured { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskResourceMeasurement {
    pub runtime_micros: u64,
    pub peak_allocation: PeakAllocationMeasurement,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskCalibrationMetrics {
    pub task: CalibrationTask,
    pub train: ConfusionCounts,
    pub holdout: ConfusionCounts,
    pub independent_holdout_cases: usize,
    pub precision_x1000: Option<u16>,
    pub recall_x1000: Option<u16>,
    pub false_positives: usize,
    pub false_negatives: usize,
    pub coverage_x1000: u16,
    pub accuracy_interval: LocalSupportInterval,
    pub minimum_holdout_cases: usize,
    pub status: EmpiricalCalibrationStatus,
    pub resources: TaskResourceMeasurement,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoldoutCalibrationReport {
    pub schema_version: String,
    pub corpus_id: String,
    pub corpus_digest: Digest32,
    pub visibility: CorpusVisibility,
    pub split_method: String,
    pub task_metrics: Vec<TaskCalibrationMetrics>,
    pub status: EmpiricalCalibrationStatus,
    pub private_overlay: String,
    pub boundary: String,
}

#[derive(Debug)]
pub enum HoldoutError {
    Io,
    SymlinkCorpus,
    CorpusTooLarge,
    InvalidJson,
    UnsupportedSchema,
    InvalidCorpus,
    FixtureEscape,
    Audit,
}

impl Display for HoldoutError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io => formatter.write_str("failed to read the public holdout corpus"),
            Self::SymlinkCorpus => formatter.write_str("holdout corpus must not be a symlink"),
            Self::CorpusTooLarge => formatter.write_str("holdout corpus exceeds its byte limit"),
            Self::InvalidJson => formatter.write_str("holdout corpus is not valid JSON"),
            Self::UnsupportedSchema => formatter.write_str("holdout corpus schema is unsupported"),
            Self::InvalidCorpus => formatter.write_str("holdout corpus invariants are invalid"),
            Self::FixtureEscape => {
                formatter.write_str("holdout fixture escapes the public fixture root")
            }
            Self::Audit => formatter.write_str("holdout fixture evaluation failed"),
        }
    }
}

impl std::error::Error for HoldoutError {}

impl From<AuditError> for HoldoutError {
    fn from(_: AuditError) -> Self {
        Self::Audit
    }
}

#[derive(Default)]
struct TaskAccumulator {
    train: ConfusionCounts,
    holdout: ConfusionCounts,
    holdout_fixtures: BTreeSet<Box<str>>,
    evaluated_holdout: usize,
    runtime_micros: u64,
}

pub fn evaluate_public_holdout(
    corpus_path: impl AsRef<Path>,
    fixture_root: impl AsRef<Path>,
) -> Result<HoldoutCalibrationReport, HoldoutError> {
    let bytes = read_bounded_corpus(corpus_path.as_ref())?;
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| HoldoutError::InvalidJson)?;
    let corpus: CalibrationCorpus =
        serde_json::from_value(value).map_err(|_| HoldoutError::InvalidCorpus)?;
    validate_corpus(&corpus)?;
    let fixture_root = std::fs::canonicalize(fixture_root).map_err(|_| HoldoutError::Io)?;
    let mut accumulators = BTreeMap::<CalibrationTask, TaskAccumulator>::new();

    for case in &corpus.cases {
        let fixture = resolve_fixture(&fixture_root, &case.fixture)?;
        let started = Instant::now();
        let actual = evaluate_expectation(&fixture, &case.expectation)?;
        let elapsed = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
        let accumulator = accumulators.entry(case.expectation.task()).or_default();
        accumulator.runtime_micros = accumulator.runtime_micros.saturating_add(elapsed);
        match case.split {
            CalibrationSplit::Train => accumulator
                .train
                .record(case.expectation.expected(), actual),
            CalibrationSplit::Holdout => {
                accumulator
                    .holdout_fixtures
                    .insert(case.fixture.as_str().into());
                accumulator
                    .holdout
                    .record(case.expectation.expected(), actual);
                accumulator.evaluated_holdout += 1;
            }
        }
    }

    let task_metrics = CalibrationTask::ALL
        .into_iter()
        .map(|task| metrics_for(task, accumulators.remove(&task).unwrap_or_default()))
        .collect::<Vec<_>>();
    let status = if task_metrics
        .iter()
        .all(|metric| metric.status == EmpiricalCalibrationStatus::Calibrated)
    {
        EmpiricalCalibrationStatus::Calibrated
    } else {
        EmpiricalCalibrationStatus::InsufficientSample
    };
    let mut digest = StableHasher::new(CORPUS_DIGEST_DOMAIN, 1);
    digest.field(1, &bytes);
    Ok(HoldoutCalibrationReport {
        schema_version: HOLDOUT_REPORT_SCHEMA_VERSION.to_string(),
        corpus_id: corpus.corpus_id,
        corpus_digest: digest.finish(),
        visibility: corpus.visibility,
        split_method: corpus.split_method,
        task_metrics,
        status,
        private_overlay: "not_included".to_string(),
        boundary: "Holdout metrics are local measurements over a tracked public synthetic corpus. Calibration requires the declared minimum number of distinct holdout fixtures per task, with no task-local fixture reuse across train and holdout. These metrics do not establish general performance, popularity, trust, security, quality, legal fitness, or publication readiness. Private corpus bodies and exact priors are not accepted by this surface.".to_string(),
    })
}

fn read_bounded_corpus(path: &Path) -> Result<Vec<u8>, HoldoutError> {
    let metadata = std::fs::symlink_metadata(path).map_err(|_| HoldoutError::Io)?;
    if metadata.file_type().is_symlink() {
        return Err(HoldoutError::SymlinkCorpus);
    }
    if !metadata.is_file() || metadata.len() > MAX_CORPUS_BYTES {
        return Err(HoldoutError::CorpusTooLarge);
    }
    let file = File::open(path).map_err(|_| HoldoutError::Io)?;
    let mut reader: Take<File> = file.take(MAX_CORPUS_BYTES + 1);
    let mut bytes = Vec::with_capacity(
        usize::try_from(metadata.len())
            .unwrap_or(0)
            .min(MAX_CORPUS_BYTES as usize),
    );
    reader
        .read_to_end(&mut bytes)
        .map_err(|_| HoldoutError::Io)?;
    if bytes.len() as u64 > MAX_CORPUS_BYTES {
        return Err(HoldoutError::CorpusTooLarge);
    }
    Ok(bytes)
}

fn validate_corpus(corpus: &CalibrationCorpus) -> Result<(), HoldoutError> {
    if corpus.schema_version != HOLDOUT_CORPUS_SCHEMA_VERSION {
        return Err(HoldoutError::UnsupportedSchema);
    }
    if corpus.corpus_id.trim().is_empty()
        || corpus.split_method != HOLDOUT_SPLIT_METHOD
        || corpus.cases.is_empty()
        || corpus.cases.len() > MAX_CASES
    {
        return Err(HoldoutError::InvalidCorpus);
    }
    let mut ids = BTreeSet::new();
    let mut train_tasks = BTreeSet::new();
    let mut holdout_tasks = BTreeSet::new();
    let mut train_fixtures = BTreeSet::new();
    let mut holdout_fixtures = BTreeSet::new();
    for case in &corpus.cases {
        if case.id.trim().is_empty() || !ids.insert(case.id.as_str()) {
            return Err(HoldoutError::InvalidCorpus);
        }
        match case.split {
            CalibrationSplit::Train => {
                train_tasks.insert(case.expectation.task());
                train_fixtures.insert((case.expectation.task(), case.fixture.as_str().to_string()));
            }
            CalibrationSplit::Holdout => {
                holdout_tasks.insert(case.expectation.task());
                holdout_fixtures
                    .insert((case.expectation.task(), case.fixture.as_str().to_string()));
            }
        }
    }
    if CalibrationTask::ALL
        .iter()
        .any(|task| !train_tasks.contains(task) || !holdout_tasks.contains(task))
    {
        return Err(HoldoutError::InvalidCorpus);
    }
    if train_fixtures
        .iter()
        .any(|fixture| holdout_fixtures.contains(fixture))
    {
        return Err(HoldoutError::InvalidCorpus);
    }
    Ok(())
}

fn resolve_fixture(
    fixture_root: &Path,
    relative: &RelativeFixturePath,
) -> Result<PathBuf, HoldoutError> {
    let fixture = std::fs::canonicalize(fixture_root.join(relative.as_str()))
        .map_err(|_| HoldoutError::Io)?;
    if !fixture.starts_with(fixture_root) || !fixture.is_dir() {
        return Err(HoldoutError::FixtureEscape);
    }
    Ok(fixture)
}

fn evaluate_expectation(
    fixture: &Path,
    expectation: &CalibrationExpectation,
) -> Result<bool, HoldoutError> {
    match expectation {
        CalibrationExpectation::Route { route, .. } => {
            let analysis = audit_repository_subtree(fixture)?;
            Ok(analysis.route_assessments.iter().any(|assessment| {
                assessment.route() == *route
                    && !matches!(
                        assessment.summary_projection().state,
                        RouteState::Absent | RouteState::UnsafeToInvent
                    )
            }))
        }
        CalibrationExpectation::Wording { .. } => Ok(!lint_wording_repository_with_profile(
            fixture,
            ProfileKind::Common,
        )?
        .findings
        .is_empty()),
        CalibrationExpectation::Consistency { .. } => {
            let analysis = audit_repository_subtree(fixture)?;
            Ok(!analysis.document_consistency.conflicts.is_empty()
                || !analysis
                    .document_consistency
                    .proposition_conflicts
                    .is_empty())
        }
        CalibrationExpectation::Profile { profile, .. } => {
            let analysis = audit_repository_subtree(fixture)?;
            Ok(analysis
                .profile
                .as_ref()
                .and_then(|report| report.branch_summary.top_profile)
                == Some(*profile))
        }
        CalibrationExpectation::Planner { .. } => {
            let analysis = audit_repository_subtree(fixture)?;
            Ok(!seiri_planner::plan_patches(&analysis).operations.is_empty())
        }
    }
}

fn metrics_for(task: CalibrationTask, accumulator: TaskAccumulator) -> TaskCalibrationMetrics {
    let holdout = accumulator.holdout;
    let independent_holdout_cases = accumulator.holdout_fixtures.len();
    let precision_x1000 = ratio_x1000(
        holdout.true_positive,
        holdout.true_positive + holdout.false_positive,
    );
    let recall_x1000 = ratio_x1000(holdout.true_positive, holdout.positives());
    let coverage_x1000 = ratio_x1000(accumulator.evaluated_holdout, holdout.samples()).unwrap_or(0);
    let status = if independent_holdout_cases >= MINIMUM_HOLDOUT_CASES_PER_TASK
        && holdout.positives() > 0
        && holdout.negatives() > 0
    {
        EmpiricalCalibrationStatus::Calibrated
    } else {
        EmpiricalCalibrationStatus::InsufficientSample
    };
    TaskCalibrationMetrics {
        task,
        train: accumulator.train,
        holdout,
        independent_holdout_cases,
        precision_x1000,
        recall_x1000,
        false_positives: holdout.false_positive,
        false_negatives: holdout.false_negative,
        coverage_x1000,
        accuracy_interval: seiri_calibration::wilson_95_interval(
            holdout.correct(),
            holdout.samples(),
        ),
        minimum_holdout_cases: MINIMUM_HOLDOUT_CASES_PER_TASK,
        status,
        resources: TaskResourceMeasurement {
            runtime_micros: accumulator.runtime_micros,
            peak_allocation: PeakAllocationMeasurement::NotMeasured {
                reason: "No allocator instrumentation is active; RepoSeiri does not report an estimated value as a measured peak.".to_string(),
            },
        },
    }
}

fn ratio_x1000(numerator: usize, denominator: usize) -> Option<u16> {
    if denominator == 0 {
        return None;
    }
    let scaled = numerator
        .saturating_mul(1000)
        .checked_div(denominator)
        .unwrap_or(0)
        .min(1000);
    Some(u16::try_from(scaled).unwrap_or(1000))
}
