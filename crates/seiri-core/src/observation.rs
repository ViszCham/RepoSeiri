use crate::{DocumentId, DocumentRole, EvidenceId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};
use std::num::NonZeroU32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CoverageId(NonZeroU32);

impl CoverageId {
    fn from_ordinal(ordinal: usize) -> Option<Self> {
        u32::try_from(ordinal)
            .ok()
            .and_then(NonZeroU32::new)
            .map(Self)
    }

    #[must_use]
    pub const fn ordinal(self) -> u32 {
        self.0.get()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum CoverageScope {
    RepositoryFiles,
    RootReadme,
    MarkdownDocuments,
    DocumentRole(DocumentRole),
    Document(DocumentId),
    RemoteMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageIncompleteReason {
    LimitExceeded,
    InvalidUtf8,
    ParseFailed,
    UnsupportedSyntax,
    PermissionDenied,
    RateLimited,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "reason", rename_all = "snake_case")]
pub enum CoverageStatus {
    Complete,
    Partial(CoverageIncompleteReason),
    NotRequested,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoverageRecord {
    pub id: CoverageId,
    pub scope: CoverageScope,
    pub status: CoverageStatus,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoverageIndex {
    records: Vec<CoverageRecord>,
}

impl CoverageIndex {
    pub fn try_new(
        entries: impl IntoIterator<Item = (CoverageScope, CoverageStatus)>,
    ) -> Result<Self, CoverageIndexError> {
        let mut scopes = BTreeSet::new();
        let mut records = Vec::new();
        for (index, (scope, status)) in entries.into_iter().enumerate() {
            if !scopes.insert(scope) {
                return Err(CoverageIndexError::DuplicateScope(scope));
            }
            let id =
                CoverageId::from_ordinal(index + 1).ok_or(CoverageIndexError::TooManyRecords)?;
            records.push(CoverageRecord { id, scope, status });
        }
        Ok(Self { records })
    }

    #[must_use]
    pub fn records(&self) -> &[CoverageRecord] {
        &self.records
    }

    #[must_use]
    pub fn record(&self, scope: CoverageScope) -> Option<&CoverageRecord> {
        self.records.iter().find(|record| record.scope == scope)
    }

    #[must_use]
    pub fn observe_absence<T>(&self, scope: CoverageScope) -> Observation<T> {
        match self.record(scope) {
            Some(record) if record.status == CoverageStatus::Complete => Observation::Absent {
                coverage: record.id,
            },
            Some(record) => Observation::Unknown(UnknownReason::from_status(record.status)),
            None => Observation::Unknown(UnknownReason::NotRequested),
        }
    }

    pub fn with_status(
        mut self,
        scope: CoverageScope,
        status: CoverageStatus,
    ) -> Result<Self, CoverageIndexError> {
        let record = self
            .records
            .iter_mut()
            .find(|record| record.scope == scope)
            .ok_or(CoverageIndexError::UnknownScope(scope))?;
        record.status = status;
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverageIndexError {
    TooManyRecords,
    DuplicateScope(CoverageScope),
    UnknownScope(CoverageScope),
}

impl Display for CoverageIndexError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManyRecords => {
                formatter.write_str("coverage record count exceeds non-zero u32")
            }
            Self::DuplicateScope(scope) => write!(formatter, "duplicate coverage scope {scope:?}"),
            Self::UnknownScope(scope) => write!(formatter, "unknown coverage scope {scope:?}"),
        }
    }
}

impl std::error::Error for CoverageIndexError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EvidenceSet(Box<[EvidenceId]>);

impl EvidenceSet {
    pub fn try_new(mut ids: Vec<EvidenceId>) -> Result<Self, ObservationError> {
        ids.sort_unstable();
        ids.dedup();
        if ids.is_empty() {
            return Err(ObservationError::EmptyEvidenceSet);
        }
        Ok(Self(ids.into_boxed_slice()))
    }

    #[must_use]
    pub fn as_slice(&self) -> &[EvidenceId] {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnknownReason {
    NotRequested,
    LimitExceeded,
    InvalidUtf8,
    ParseFailed,
    UnsupportedSyntax,
    PermissionDenied,
    RateLimited,
    Unavailable,
}

impl UnknownReason {
    const fn from_status(status: CoverageStatus) -> Self {
        match status {
            CoverageStatus::Complete => Self::ParseFailed,
            CoverageStatus::NotRequested => Self::NotRequested,
            CoverageStatus::Partial(reason) => match reason {
                CoverageIncompleteReason::LimitExceeded => Self::LimitExceeded,
                CoverageIncompleteReason::InvalidUtf8 => Self::InvalidUtf8,
                CoverageIncompleteReason::ParseFailed => Self::ParseFailed,
                CoverageIncompleteReason::UnsupportedSyntax => Self::UnsupportedSyntax,
                CoverageIncompleteReason::PermissionDenied => Self::PermissionDenied,
                CoverageIncompleteReason::RateLimited => Self::RateLimited,
                CoverageIncompleteReason::Unavailable => Self::Unavailable,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", content = "data", rename_all = "snake_case")]
pub enum Observation<T> {
    Present { value: T, evidence: EvidenceSet },
    Absent { coverage: CoverageId },
    Unknown(UnknownReason),
    Conflict { alternatives: EvidenceSet },
}

impl<T> Observation<T> {
    pub fn present(value: T, evidence: Vec<EvidenceId>) -> Result<Self, ObservationError> {
        Ok(Self::Present {
            value,
            evidence: EvidenceSet::try_new(evidence)?,
        })
    }

    pub fn conflict(alternatives: Vec<EvidenceId>) -> Result<Self, ObservationError> {
        Ok(Self::Conflict {
            alternatives: EvidenceSet::try_new(alternatives)?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationError {
    EmptyEvidenceSet,
}

impl Display for ObservationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("present and conflict observations require evidence")
    }
}

impl std::error::Error for ObservationError {}
