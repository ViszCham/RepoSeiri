use crate::{events, DocumentScanOptions, MarkdownError};
use seiri_core::{DocumentIndex, DocumentScan, DocumentScanStatus};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceDocument {
    path: String,
    text: Arc<str>,
}

impl SourceDocument {
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    pub(crate) fn new(path: String, text: Arc<str>) -> Self {
        Self { path, text }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSourceSession {
    index: DocumentIndex,
    sources: Vec<SourceDocument>,
}

impl DocumentSourceSession {
    pub(crate) fn new(index: DocumentIndex, sources: Vec<SourceDocument>) -> Self {
        Self { index, sources }
    }

    #[must_use]
    pub const fn index(&self) -> &DocumentIndex {
        &self.index
    }

    #[must_use]
    pub fn sources(&self) -> &[SourceDocument] {
        &self.sources
    }

    #[must_use]
    pub fn into_index(self) -> DocumentIndex {
        self.index
    }

    #[must_use]
    pub fn into_parts(self) -> (DocumentIndex, Vec<SourceDocument>) {
        (self.index, self.sources)
    }
}

pub(crate) enum PreparedMarkdown {
    Scanned {
        scan: DocumentScan,
        source: Arc<str>,
    },
    Unavailable(DocumentScanStatus),
}

pub(crate) fn prepare_markdown_document(
    full_path: &Path,
    relative_path: &str,
    options: &DocumentScanOptions,
) -> PreparedMarkdown {
    match scan_document_file_and_source(full_path, relative_path, options) {
        Ok((scan, source)) => PreparedMarkdown::Scanned { scan, source },
        Err(error) => PreparedMarkdown::Unavailable(scan_status_for_error(&error)),
    }
}

pub(crate) fn scan_document_file_and_source(
    full_path: &Path,
    relative_path: &str,
    options: &DocumentScanOptions,
) -> Result<(DocumentScan, Arc<str>), MarkdownError> {
    let bytes = read_bounded_file(full_path, relative_path, options.max_source_bytes)?;
    let text = String::from_utf8(bytes).map_err(|error| MarkdownError::InvalidUtf8 {
        path: full_path.to_path_buf(),
        valid_up_to: error.utf8_error().valid_up_to(),
    })?;
    let scan = events::scan_text(relative_path.to_string(), &text, options)?;
    Ok((scan, Arc::from(text)))
}

pub(crate) fn read_bounded_file(
    full_path: &Path,
    relative_path: &str,
    limit: usize,
) -> Result<Vec<u8>, MarkdownError> {
    let declared_bytes = fs::metadata(full_path)
        .map_err(|source| MarkdownError::Io {
            path: full_path.to_path_buf(),
            source,
        })?
        .len();
    if declared_bytes > u64::try_from(limit).unwrap_or(u64::MAX) {
        return Err(MarkdownError::SourceLimitExceeded {
            path: relative_path.to_string(),
            bytes: usize::try_from(declared_bytes).unwrap_or(usize::MAX),
            limit,
        });
    }

    let file = File::open(full_path).map_err(|source| MarkdownError::Io {
        path: full_path.to_path_buf(),
        source,
    })?;
    let read_limit = u64::try_from(limit).unwrap_or(u64::MAX).saturating_add(1);
    let mut bytes = Vec::with_capacity(limit.min(64 * 1024));
    file.take(read_limit)
        .read_to_end(&mut bytes)
        .map_err(|source| MarkdownError::Io {
            path: full_path.to_path_buf(),
            source,
        })?;
    if bytes.len() > limit {
        return Err(MarkdownError::SourceLimitExceeded {
            path: relative_path.to_string(),
            bytes: bytes.len(),
            limit,
        });
    }
    Ok(bytes)
}

pub(crate) fn scan_status_for_error(error: &MarkdownError) -> DocumentScanStatus {
    match error {
        MarkdownError::InvalidUtf8 { .. } => DocumentScanStatus::InvalidUtf8,
        MarkdownError::Io { source, .. } if source.kind() == io::ErrorKind::PermissionDenied => {
            DocumentScanStatus::PermissionDenied
        }
        MarkdownError::SourceLimitExceeded { .. }
        | MarkdownError::EventLimitExceeded { .. }
        | MarkdownError::DiagnosticLimitExceeded { .. } => DocumentScanStatus::SkippedByteBudget,
        MarkdownError::Io { .. } | MarkdownError::Invariant(_) => DocumentScanStatus::ParseFailed,
    }
}
