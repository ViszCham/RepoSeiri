use crate::{CoverageIncompleteReason, CoverageStatus};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RemoteRepositoryReference {
    pub host: String,
    pub owner: String,
    pub name: String,
}

impl RemoteRepositoryReference {
    pub fn try_new(
        host: impl Into<String>,
        owner: impl Into<String>,
        name: impl Into<String>,
    ) -> Result<Self, RemoteRepositoryReferenceError> {
        let reference = Self {
            host: host.into(),
            owner: owner.into(),
            name: name.into(),
        };
        if reference.host.trim().is_empty()
            || reference.owner.trim().is_empty()
            || reference.name.trim().is_empty()
        {
            return Err(RemoteRepositoryReferenceError::EmptyComponent);
        }
        if [
            reference.host.len(),
            reference.owner.len(),
            reference.name.len(),
        ]
        .into_iter()
        .any(|length| length > 255)
        {
            return Err(RemoteRepositoryReferenceError::ComponentTooLong);
        }
        Ok(reference)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteRepositoryReferenceError {
    EmptyComponent,
    ComponentTooLong,
}

impl Display for RemoteRepositoryReferenceError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyComponent => {
                formatter.write_str("remote repository host, owner, and name must not be empty")
            }
            Self::ComponentTooLong => {
                formatter.write_str("remote repository components must be at most 255 bytes")
            }
        }
    }
}

impl std::error::Error for RemoteRepositoryReferenceError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemoteUnavailableReason {
    Transport,
    ResponseTooLarge,
    MalformedResponse,
    UnexpectedStatus(u16),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum RemoteEvidenceStatus {
    NotRequested,
    Denied,
    NotFound,
    RateLimited { retry_after_seconds: Option<u32> },
    Unavailable(RemoteUnavailableReason),
    Observed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RemoteRepositoryMetadata {
    pub default_branch: Option<String>,
    pub archived: Option<bool>,
    pub license_spdx: Option<String>,
    pub web_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteEvidenceReport {
    pub repository: Option<RemoteRepositoryReference>,
    pub status: RemoteEvidenceStatus,
    pub metadata: Option<RemoteRepositoryMetadata>,
    pub coverage: CoverageStatus,
    pub boundary: String,
}

impl RemoteEvidenceReport {
    #[must_use]
    pub fn not_requested() -> Self {
        Self {
            repository: None,
            status: RemoteEvidenceStatus::NotRequested,
            metadata: None,
            coverage: CoverageStatus::NotRequested,
            boundary: remote_boundary(),
        }
    }

    #[must_use]
    pub fn denied(repository: RemoteRepositoryReference) -> Self {
        Self::with_status(
            repository,
            RemoteEvidenceStatus::Denied,
            None,
            CoverageStatus::Partial(CoverageIncompleteReason::PermissionDenied),
        )
    }

    #[must_use]
    pub fn not_found(repository: RemoteRepositoryReference) -> Self {
        Self::with_status(
            repository,
            RemoteEvidenceStatus::NotFound,
            None,
            CoverageStatus::Complete,
        )
    }

    #[must_use]
    pub fn rate_limited(
        repository: RemoteRepositoryReference,
        retry_after_seconds: Option<u32>,
    ) -> Self {
        Self::with_status(
            repository,
            RemoteEvidenceStatus::RateLimited {
                retry_after_seconds,
            },
            None,
            CoverageStatus::Partial(CoverageIncompleteReason::RateLimited),
        )
    }

    #[must_use]
    pub fn unavailable(
        repository: RemoteRepositoryReference,
        reason: RemoteUnavailableReason,
    ) -> Self {
        Self::with_status(
            repository,
            RemoteEvidenceStatus::Unavailable(reason),
            None,
            CoverageStatus::Partial(CoverageIncompleteReason::Unavailable),
        )
    }

    #[must_use]
    pub fn observed(
        repository: RemoteRepositoryReference,
        metadata: RemoteRepositoryMetadata,
    ) -> Self {
        Self::with_status(
            repository,
            RemoteEvidenceStatus::Observed,
            Some(metadata),
            CoverageStatus::Complete,
        )
    }

    fn with_status(
        repository: RemoteRepositoryReference,
        status: RemoteEvidenceStatus,
        metadata: Option<RemoteRepositoryMetadata>,
        coverage: CoverageStatus,
    ) -> Self {
        Self {
            repository: Some(repository),
            status,
            metadata,
            coverage,
            boundary: remote_boundary(),
        }
    }
}

impl Default for RemoteEvidenceReport {
    fn default() -> Self {
        Self::not_requested()
    }
}

fn remote_boundary() -> String {
    "Remote evidence is opt-in and read-only. This report contains no authorization token, does not establish repository truth, trust, ownership, security, availability, or policy compliance, and does not enable GitHub mutation.".to_string()
}
