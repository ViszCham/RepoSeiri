use seiri_core::{
    RemoteEvidenceReport, RemoteRepositoryMetadata, RemoteRepositoryReference,
    RemoteUnavailableReason,
};
use serde::Deserialize;
use std::fmt::{Debug, Formatter};
use std::num::NonZeroUsize;

const DEFAULT_MAX_RESPONSE_BYTES: usize = 32 * 1024;

pub struct RemoteReadAuthorization(Box<str>);

impl RemoteReadAuthorization {
    #[must_use]
    pub fn new(token: impl Into<Box<str>>) -> Option<Self> {
        let token = token.into();
        (!token.trim().is_empty()).then_some(Self(token))
    }

    #[must_use]
    pub fn expose_for_transport(&self) -> &str {
        &self.0
    }
}

impl Debug for RemoteReadAuthorization {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("RemoteReadAuthorization(REDACTED)")
    }
}

pub struct RemoteReadRequest {
    repository: RemoteRepositoryReference,
    authorization: Option<RemoteReadAuthorization>,
    max_response_bytes: NonZeroUsize,
}

impl RemoteReadRequest {
    #[must_use]
    pub fn new(
        repository: RemoteRepositoryReference,
        authorization: Option<RemoteReadAuthorization>,
        max_response_bytes: usize,
    ) -> Option<Self> {
        Some(Self {
            repository,
            authorization,
            max_response_bytes: NonZeroUsize::new(max_response_bytes)?,
        })
    }

    #[must_use]
    pub fn repository(&self) -> &RemoteRepositoryReference {
        &self.repository
    }

    #[must_use]
    pub fn authorization(&self) -> Option<&RemoteReadAuthorization> {
        self.authorization.as_ref()
    }

    #[must_use]
    pub fn max_response_bytes(&self) -> usize {
        self.max_response_bytes.get()
    }
}

impl Debug for RemoteReadRequest {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RemoteReadRequest")
            .field("repository", &self.repository)
            .field(
                "authorization",
                &self.authorization.as_ref().map(|_| "REDACTED"),
            )
            .field("max_response_bytes", &self.max_response_bytes)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteRequestMode {
    NotRequested,
    ReadOnly,
}

pub struct RemoteEvidenceOptions {
    mode: RemoteRequestMode,
    request: Option<RemoteReadRequest>,
}

impl RemoteEvidenceOptions {
    #[must_use]
    pub fn not_requested() -> Self {
        Self {
            mode: RemoteRequestMode::NotRequested,
            request: None,
        }
    }

    #[must_use]
    pub fn read_only(request: RemoteReadRequest) -> Self {
        Self {
            mode: RemoteRequestMode::ReadOnly,
            request: Some(request),
        }
    }

    #[must_use]
    pub fn github_repository(
        owner: impl Into<String>,
        name: impl Into<String>,
        authorization: Option<RemoteReadAuthorization>,
    ) -> Option<Self> {
        let repository = RemoteRepositoryReference::try_new("api.github.com", owner, name).ok()?;
        let request =
            RemoteReadRequest::new(repository, authorization, DEFAULT_MAX_RESPONSE_BYTES)?;
        Some(Self::read_only(request))
    }

    #[must_use]
    pub fn mode(&self) -> RemoteRequestMode {
        self.mode.clone()
    }

    #[must_use]
    pub fn request(&self) -> Option<&RemoteReadRequest> {
        self.request.as_ref()
    }
}

impl Default for RemoteEvidenceOptions {
    fn default() -> Self {
        Self::not_requested()
    }
}

impl Debug for RemoteEvidenceOptions {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RemoteEvidenceOptions")
            .field("mode", &self.mode)
            .field("request", &self.request)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteTransportResponse {
    pub status: u16,
    pub retry_after_seconds: Option<u32>,
    pub rate_limited: bool,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteTransportError {
    Unavailable,
}

pub trait RemoteTransport {
    fn get_repository_metadata(
        &self,
        request: &RemoteReadRequest,
    ) -> Result<RemoteTransportResponse, RemoteTransportError>;
}

#[must_use]
pub fn collect_repository_evidence<T: RemoteTransport>(
    options: &RemoteEvidenceOptions,
    transport: &T,
) -> RemoteEvidenceReport {
    let Some(request) = options.request() else {
        return RemoteEvidenceReport::not_requested();
    };
    if options.mode != RemoteRequestMode::ReadOnly {
        return RemoteEvidenceReport::not_requested();
    }
    let repository = request.repository().clone();
    let response = match transport.get_repository_metadata(request) {
        Ok(response) => response,
        Err(RemoteTransportError::Unavailable) => {
            return RemoteEvidenceReport::unavailable(
                repository,
                RemoteUnavailableReason::Transport,
            );
        }
    };

    if response.status == 429 || response.rate_limited {
        return RemoteEvidenceReport::rate_limited(repository, response.retry_after_seconds);
    }
    match response.status {
        200..=299 => parse_success(repository, request.max_response_bytes(), &response.body),
        401 | 403 => RemoteEvidenceReport::denied(repository),
        404 => RemoteEvidenceReport::not_found(repository),
        status => RemoteEvidenceReport::unavailable(
            repository,
            RemoteUnavailableReason::UnexpectedStatus(status),
        ),
    }
}

fn parse_success(
    repository: RemoteRepositoryReference,
    max_response_bytes: usize,
    body: &[u8],
) -> RemoteEvidenceReport {
    if body.len() > max_response_bytes {
        return RemoteEvidenceReport::unavailable(
            repository,
            RemoteUnavailableReason::ResponseTooLarge,
        );
    }
    let wire = match serde_json::from_slice::<GithubRepositoryWire>(body) {
        Ok(wire) => wire,
        Err(_) => {
            return RemoteEvidenceReport::unavailable(
                repository,
                RemoteUnavailableReason::MalformedResponse,
            );
        }
    };
    RemoteEvidenceReport::observed(
        repository,
        RemoteRepositoryMetadata {
            default_branch: wire.default_branch,
            archived: wire.archived,
            license_spdx: wire.license.and_then(|license| license.spdx_id),
            web_url: wire.html_url,
        },
    )
}

#[derive(Debug, Deserialize)]
struct GithubRepositoryWire {
    #[serde(default)]
    default_branch: Option<String>,
    #[serde(default)]
    archived: Option<bool>,
    #[serde(default)]
    license: Option<GithubLicenseWire>,
    #[serde(default)]
    html_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GithubLicenseWire {
    #[serde(default)]
    spdx_id: Option<String>,
}
