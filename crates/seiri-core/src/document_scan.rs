use crate::{
    MarkdownBadge, MarkdownHeading, MarkdownLink, RouteCandidate, SourceSpan, TextDocumentBase,
};
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum DocumentEvent {
    VisibleProse(MarkdownProse),
    Heading(MarkdownHeading),
    Link(MarkdownLink),
    Badge(MarkdownBadge),
    RouteCandidate(RouteCandidate),
}

impl DocumentEvent {
    #[must_use]
    pub const fn span(&self) -> Option<SourceSpan> {
        match self {
            Self::VisibleProse(value) => Some(value.span),
            Self::Heading(value) => value.span,
            Self::Link(value) => value.span,
            Self::Badge(value) => value.span,
            Self::RouteCandidate(value) => value.span,
        }
    }

    pub const fn order_rank(&self) -> u8 {
        match self {
            Self::VisibleProse(_) => 0,
            Self::Heading(_) => 1,
            Self::Link(_) => 2,
            Self::Badge(_) => 3,
            Self::RouteCandidate(_) => 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkdownProse {
    pub text: String,
    pub line: usize,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentDiagnosticKind {
    UnclosedLinkLabel,
    UnclosedLinkTarget,
    UnresolvedReferenceLink,
    UnsupportedHtml,
    HtmlAttributeLimitExceeded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentDiagnostic {
    pub kind: DocumentDiagnosticKind,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DocumentScan {
    path: String,
    source_bytes: usize,
    base: TextDocumentBase,
    events: Vec<DocumentEvent>,
    diagnostics: Vec<DocumentDiagnostic>,
}

impl DocumentScan {
    pub fn new(
        path: String,
        base: TextDocumentBase,
        events: Vec<DocumentEvent>,
        diagnostics: Vec<DocumentDiagnostic>,
    ) -> Result<Self, DocumentScanInvariantError> {
        let source_bytes = base.byte_len();
        validate_document_scan(&path, source_bytes, &events, &diagnostics)?;
        Ok(Self {
            path,
            source_bytes,
            base,
            events,
            diagnostics,
        })
    }

    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    #[must_use]
    pub const fn source_bytes(&self) -> usize {
        self.source_bytes
    }

    #[must_use]
    pub const fn base(&self) -> &TextDocumentBase {
        &self.base
    }

    #[must_use]
    pub fn events(&self) -> &[DocumentEvent] {
        &self.events
    }

    #[must_use]
    pub fn diagnostics(&self) -> &[DocumentDiagnostic] {
        &self.diagnostics
    }
}

impl<'de> Deserialize<'de> for DocumentScan {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct WireScan {
            path: String,
            source_bytes: usize,
            base: TextDocumentBase,
            events: Vec<DocumentEvent>,
            diagnostics: Vec<DocumentDiagnostic>,
        }

        let wire = WireScan::deserialize(deserializer)?;
        if wire.source_bytes != wire.base.byte_len() {
            return Err(D::Error::custom(
                "document source_bytes must match base byte_len",
            ));
        }
        Self::new(wire.path, wire.base, wire.events, wire.diagnostics).map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentScanInvariantError {
    EmptyPath,
    MissingEventSpan { event_index: usize },
    EventSpanOutOfBounds { event_index: usize },
    DiagnosticSpanOutOfBounds { diagnostic_index: usize },
    NonCanonicalEventOrder { event_index: usize },
    NonCanonicalDiagnosticOrder { diagnostic_index: usize },
}

impl Display for DocumentScanInvariantError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyPath => formatter.write_str("document scan path must not be empty"),
            Self::MissingEventSpan { event_index } => {
                write!(
                    formatter,
                    "document event {event_index} is missing a source span"
                )
            }
            Self::EventSpanOutOfBounds { event_index } => write!(
                formatter,
                "document event {event_index} has a span outside the source byte range"
            ),
            Self::DiagnosticSpanOutOfBounds { diagnostic_index } => write!(
                formatter,
                "document diagnostic {diagnostic_index} has a span outside the source byte range"
            ),
            Self::NonCanonicalEventOrder { event_index } => write!(
                formatter,
                "document event {event_index} is not in deterministic source order"
            ),
            Self::NonCanonicalDiagnosticOrder { diagnostic_index } => write!(
                formatter,
                "document diagnostic {diagnostic_index} is not in deterministic source order"
            ),
        }
    }
}

impl std::error::Error for DocumentScanInvariantError {}

fn validate_document_scan(
    path: &str,
    source_bytes: usize,
    events: &[DocumentEvent],
    diagnostics: &[DocumentDiagnostic],
) -> Result<(), DocumentScanInvariantError> {
    if path.is_empty() {
        return Err(DocumentScanInvariantError::EmptyPath);
    }

    let mut previous_key = None;
    for (event_index, event) in events.iter().enumerate() {
        let span = event
            .span()
            .ok_or(DocumentScanInvariantError::MissingEventSpan { event_index })?;
        if span.byte_end > source_bytes {
            return Err(DocumentScanInvariantError::EventSpanOutOfBounds { event_index });
        }
        let key = (span.byte_start, event.order_rank(), span.byte_end);
        if previous_key.is_some_and(|previous| previous > key) {
            return Err(DocumentScanInvariantError::NonCanonicalEventOrder { event_index });
        }
        previous_key = Some(key);
    }

    let mut previous_diagnostic_key = None;
    for (diagnostic_index, diagnostic) in diagnostics.iter().enumerate() {
        if diagnostic.span.byte_end > source_bytes {
            return Err(DocumentScanInvariantError::DiagnosticSpanOutOfBounds { diagnostic_index });
        }
        let key = (
            diagnostic.span.byte_start,
            diagnostic_kind_rank(diagnostic.kind),
            diagnostic.span.byte_end,
        );
        if previous_diagnostic_key.is_some_and(|previous| previous > key) {
            return Err(DocumentScanInvariantError::NonCanonicalDiagnosticOrder {
                diagnostic_index,
            });
        }
        previous_diagnostic_key = Some(key);
    }
    Ok(())
}

const fn diagnostic_kind_rank(kind: DocumentDiagnosticKind) -> u8 {
    match kind {
        DocumentDiagnosticKind::UnclosedLinkLabel => 0,
        DocumentDiagnosticKind::UnclosedLinkTarget => 1,
        DocumentDiagnosticKind::UnresolvedReferenceLink => 2,
        DocumentDiagnosticKind::UnsupportedHtml => 3,
        DocumentDiagnosticKind::HtmlAttributeLimitExceeded => 4,
    }
}
