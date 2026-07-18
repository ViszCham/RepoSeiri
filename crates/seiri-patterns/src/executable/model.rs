use super::error::PatternPackLoadError;
use crate::{PatternAdoptionStage, PatternFixtureKind, PredicateProgram};
use seiri_core::{
    ClaimBoundaryKind, CoverageScope, CoverageStatus, EvidenceId, PatternGroup, PatternOutcome,
    ReviewGapKind,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

pub const EXECUTABLE_PATTERN_PACK_SCHEMA_VERSION: &str = "seiri.executable-pattern-pack.v2";
pub const MAX_EXECUTABLE_FIXTURES: usize = 512;
pub const MAX_FIXTURE_EXPECTATIONS: usize = 64;
pub const MAX_DATA_PATTERN_DEFINITIONS: usize = 256;
pub(super) const MAX_FIXTURE_ENTRIES: usize = 100_000;
pub(super) const MAX_FIXTURE_DEPTH: usize = 32;
pub(super) const MAX_FIXTURE_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub(super) const MAX_FIXTURE_TOTAL_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct RelativeFixturePath(Box<str>);

impl RelativeFixturePath {
    pub fn try_new(value: impl Into<String>) -> Result<Self, PatternPackLoadError> {
        let value = value.into();
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
            return Err(PatternPackLoadError::InvalidFixturePath);
        }
        Ok(Self(value.into_boxed_str()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for RelativeFixturePath {
    type Error = PatternPackLoadError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

impl From<RelativeFixturePath> for String {
    fn from(value: RelativeFixturePath) -> Self {
        value.0.into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureScanBudget {
    pub max_depth: usize,
    pub max_entries: usize,
    pub max_file_bytes: u64,
    pub max_total_bytes: u64,
}

impl Default for FixtureScanBudget {
    fn default() -> Self {
        Self {
            max_depth: 16,
            max_entries: 10_000,
            max_file_bytes: 1024 * 1024,
            max_total_bytes: 8 * 1024 * 1024,
        }
    }
}

impl FixtureScanBudget {
    pub(super) fn validate(self) -> Result<(), PatternPackLoadError> {
        if self.max_depth == 0
            || self.max_depth > MAX_FIXTURE_DEPTH
            || self.max_entries == 0
            || self.max_entries > MAX_FIXTURE_ENTRIES
            || self.max_file_bytes == 0
            || self.max_file_bytes > MAX_FIXTURE_FILE_BYTES
            || self.max_total_bytes == 0
            || self.max_total_bytes > MAX_FIXTURE_TOTAL_BYTES
        {
            return Err(PatternPackLoadError::InvalidScanBudget);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum EvidenceExpectation {
    #[default]
    Any,
    AtLeast(u16),
    Exact(Vec<EvidenceId>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FixtureExpectation {
    Pattern {
        pattern: String,
        outcome: PatternOutcome,
        #[serde(default)]
        evidence: EvidenceExpectation,
    },
    Coverage {
        scope: CoverageScope,
        status: CoverageStatus,
    },
    Gap {
        gap: ReviewGapKind,
        minimum: u16,
        maximum: u16,
    },
    ClaimBoundary {
        boundary: ClaimBoundaryKind,
        present: bool,
    },
    Diagnostic {
        minimum: u16,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutableFixtureSpec {
    pub id: String,
    pub kind: PatternFixtureKind,
    pub group: PatternGroup,
    pub root: RelativeFixturePath,
    pub expectations: Vec<FixtureExpectation>,
    #[serde(default)]
    pub scan_budget: FixtureScanBudget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DataPatternDefinition {
    pub id: String,
    pub group: PatternGroup,
    pub predicate: PredicateProgram,
    pub boundaries: Vec<ClaimBoundaryKind>,
    pub adoption_stage: PatternAdoptionStage,
    #[serde(default = "enabled_by_default")]
    pub enabled: bool,
}

const fn enabled_by_default() -> bool {
    true
}

pub struct ExecutablePatternPack {
    pub(super) schema_version: Box<str>,
    pub(super) id: Box<str>,
    pub(super) version: Box<str>,
    pub(super) definitions: Box<[DataPatternDefinition]>,
    pub(super) fixtures: Box<[ExecutableFixtureSpec]>,
    pub(super) fixture_roots: BTreeMap<String, PathBuf>,
    pub(super) fingerprint: Box<str>,
}

impl ExecutablePatternPack {
    #[must_use]
    pub fn schema_version(&self) -> &str {
        &self.schema_version
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    #[must_use]
    pub fn definitions(&self) -> &[DataPatternDefinition] {
        &self.definitions
    }

    #[must_use]
    pub fn fixtures(&self) -> &[ExecutableFixtureSpec] {
        &self.fixtures
    }

    #[must_use]
    pub fn definition(&self, id: &str) -> Option<&DataPatternDefinition> {
        self.definitions
            .iter()
            .find(|definition| definition.id == id)
    }

    #[must_use]
    pub fn fixture_root(&self, id: &str) -> Option<&Path> {
        self.fixture_roots.get(id).map(PathBuf::as_path)
    }

    #[must_use]
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FixtureExpectationActual {
    Pattern {
        outcome: Option<PatternOutcome>,
        evidence_ids: Vec<EvidenceId>,
    },
    Coverage {
        status: Option<CoverageStatus>,
    },
    Count {
        value: usize,
    },
    Boundary {
        present: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FixtureExpectationResult {
    pub index: usize,
    pub passed: bool,
    pub actual: FixtureExpectationActual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FixtureExecutionStatus {
    Passed,
    Failed,
    AuditError,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FixtureExecutionResult {
    pub fixture_id: String,
    pub kind: PatternFixtureKind,
    pub status: FixtureExecutionStatus,
    pub expectations: Vec<FixtureExpectationResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FixtureSuiteReport {
    pub schema_version: &'static str,
    pub pack_fingerprint: String,
    pub results: Vec<FixtureExecutionResult>,
    pub subprocesses_started: usize,
    pub network_requests_started: usize,
}

impl FixtureSuiteReport {
    #[must_use]
    pub fn all_passed(&self) -> bool {
        !self.results.is_empty()
            && self
                .results
                .iter()
                .all(|result| result.status == FixtureExecutionStatus::Passed)
            && self.subprocesses_started == 0
            && self.network_requests_started == 0
    }
}
