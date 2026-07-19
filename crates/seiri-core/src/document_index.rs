use crate::{
    CoverageIncompleteReason, CoverageStatus, DocumentId, DocumentScan, PatchBaseDigest,
    PathClassification, TextDocumentBase, TextEncoding,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum DocumentRole {
    RootReadme,
    Documentation,
    SecurityPolicy,
    SupportPolicy,
    ContributionGuide,
    ReleaseNotes,
    Governance,
    GithubConfiguration,
    OtherMarkdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DocumentRoleMask(u16);

impl DocumentRoleMask {
    pub const NONE: Self = Self(0);
    pub const ALL: Self = Self((1 << DocumentRole::ALL.len()) - 1);

    #[must_use]
    pub const fn only(role: DocumentRole) -> Self {
        Self(1 << role as u8)
    }

    #[must_use]
    pub const fn with(self, role: DocumentRole) -> Self {
        Self(self.0 | (1 << role as u8))
    }

    #[must_use]
    pub const fn contains(self, role: DocumentRole) -> bool {
        self.0 & (1 << role as u8) != 0
    }
}

impl DocumentRole {
    pub const ALL: [Self; 9] = [
        Self::RootReadme,
        Self::Documentation,
        Self::SecurityPolicy,
        Self::SupportPolicy,
        Self::ContributionGuide,
        Self::ReleaseNotes,
        Self::Governance,
        Self::GithubConfiguration,
        Self::OtherMarkdown,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentScanStatus {
    Scanned,
    NotMarkdown,
    SkippedDocumentBudget,
    SkippedByteBudget,
    InvalidUtf8,
    ParseFailed,
    PermissionDenied,
}

impl DocumentScanStatus {
    #[must_use]
    pub const fn coverage_status(self) -> CoverageStatus {
        match self {
            Self::Scanned => CoverageStatus::Complete,
            Self::NotMarkdown => CoverageStatus::NotRequested,
            Self::SkippedDocumentBudget | Self::SkippedByteBudget => {
                CoverageStatus::Partial(CoverageIncompleteReason::LimitExceeded)
            }
            Self::InvalidUtf8 => CoverageStatus::Partial(CoverageIncompleteReason::InvalidUtf8),
            Self::ParseFailed => CoverageStatus::Partial(CoverageIncompleteReason::ParseFailed),
            Self::PermissionDenied => {
                CoverageStatus::Partial(CoverageIncompleteReason::PermissionDenied)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedDocument {
    pub path: String,
    pub role: DocumentRole,
    pub classification: PathClassification,
    pub declared_bytes: u64,
    pub document_id: Option<DocumentId>,
    pub status: DocumentScanStatus,
    pub digest: Option<PatchBaseDigest>,
    pub encoding: Option<TextEncoding>,
    pub scan: Option<DocumentScan>,
}

impl IndexedDocument {
    #[must_use]
    pub fn scanned(
        path: String,
        role: DocumentRole,
        classification: PathClassification,
        declared_bytes: u64,
        scan: DocumentScan,
    ) -> Self {
        Self {
            path,
            role,
            classification,
            declared_bytes,
            document_id: None,
            status: DocumentScanStatus::Scanned,
            digest: Some(scan.base().digest()),
            encoding: Some(scan.base().encoding()),
            scan: Some(scan),
        }
    }

    #[must_use]
    pub fn unavailable(
        path: String,
        role: DocumentRole,
        classification: PathClassification,
        declared_bytes: u64,
        status: DocumentScanStatus,
    ) -> Self {
        assert!(status != DocumentScanStatus::Scanned);
        Self {
            path,
            role,
            classification,
            declared_bytes,
            document_id: None,
            status,
            digest: None,
            encoding: None,
            scan: None,
        }
    }

    #[must_use]
    pub fn unparsed(
        path: String,
        role: DocumentRole,
        classification: PathClassification,
        declared_bytes: u64,
        base: TextDocumentBase,
    ) -> Self {
        Self {
            path,
            role,
            classification,
            declared_bytes,
            document_id: None,
            status: DocumentScanStatus::NotMarkdown,
            digest: Some(base.digest()),
            encoding: Some(base.encoding()),
            scan: None,
        }
    }

    #[must_use]
    pub fn is_markdown(&self) -> bool {
        self.status != DocumentScanStatus::NotMarkdown
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentRoleCoverage {
    pub role: DocumentRole,
    pub status: CoverageStatus,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentSelectionSummary {
    pub candidates: usize,
    pub selected: usize,
    pub skipped_document_budget: usize,
    pub skipped_byte_budget: usize,
    pub selected_source_bytes: u64,
    pub primary_candidates: usize,
    pub primary_selected: usize,
    pub primary_skipped_document_budget: usize,
    pub primary_skipped_byte_budget: usize,
    pub primary_selected_source_bytes: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentIndex {
    entries: Vec<IndexedDocument>,
    role_coverage: Vec<DocumentRoleCoverage>,
    selection: DocumentSelectionSummary,
}

impl DocumentIndex {
    pub fn try_new(
        entries: Vec<IndexedDocument>,
        role_coverage: Vec<DocumentRoleCoverage>,
    ) -> Result<Self, DocumentIndexError> {
        let mut paths = BTreeSet::new();
        let mut previous_path = None;
        for entry in &entries {
            if entry.path.is_empty() {
                return Err(DocumentIndexError::EmptyPath);
            }
            if !paths.insert(entry.path.as_str()) {
                return Err(DocumentIndexError::DuplicatePath(entry.path.clone()));
            }
            if previous_path.is_some_and(|previous: &str| previous > entry.path.as_str()) {
                return Err(DocumentIndexError::NonCanonicalEntryOrder);
            }
            previous_path = Some(entry.path.as_str());
            if (entry.status == DocumentScanStatus::Scanned) != entry.scan.is_some() {
                return Err(DocumentIndexError::ScanStatusMismatch(entry.path.clone()));
            }
        }

        let mut roles = BTreeSet::new();
        let mut previous_role = None;
        for coverage in &role_coverage {
            if !roles.insert(coverage.role) {
                return Err(DocumentIndexError::DuplicateRoleCoverage(coverage.role));
            }
            if previous_role.is_some_and(|previous| previous > coverage.role) {
                return Err(DocumentIndexError::NonCanonicalRoleOrder);
            }
            previous_role = Some(coverage.role);
        }
        let selection = DocumentSelectionSummary {
            candidates: entries.len(),
            selected: entries
                .iter()
                .filter(|entry| {
                    !matches!(
                        entry.status,
                        DocumentScanStatus::SkippedDocumentBudget
                            | DocumentScanStatus::SkippedByteBudget
                    )
                })
                .count(),
            skipped_document_budget: entries
                .iter()
                .filter(|entry| entry.status == DocumentScanStatus::SkippedDocumentBudget)
                .count(),
            skipped_byte_budget: entries
                .iter()
                .filter(|entry| entry.status == DocumentScanStatus::SkippedByteBudget)
                .count(),
            selected_source_bytes: entries
                .iter()
                .filter(|entry| {
                    !matches!(
                        entry.status,
                        DocumentScanStatus::SkippedDocumentBudget
                            | DocumentScanStatus::SkippedByteBudget
                    )
                })
                .map(|entry| entry.declared_bytes)
                .sum(),
            primary_candidates: entries
                .iter()
                .filter(|entry| entry.classification.is_primary_repository_content())
                .count(),
            primary_selected: entries
                .iter()
                .filter(|entry| entry.classification.is_primary_repository_content())
                .filter(|entry| {
                    !matches!(
                        entry.status,
                        DocumentScanStatus::SkippedDocumentBudget
                            | DocumentScanStatus::SkippedByteBudget
                    )
                })
                .count(),
            primary_skipped_document_budget: entries
                .iter()
                .filter(|entry| entry.classification.is_primary_repository_content())
                .filter(|entry| entry.status == DocumentScanStatus::SkippedDocumentBudget)
                .count(),
            primary_skipped_byte_budget: entries
                .iter()
                .filter(|entry| entry.classification.is_primary_repository_content())
                .filter(|entry| entry.status == DocumentScanStatus::SkippedByteBudget)
                .count(),
            primary_selected_source_bytes: entries
                .iter()
                .filter(|entry| entry.classification.is_primary_repository_content())
                .filter(|entry| {
                    !matches!(
                        entry.status,
                        DocumentScanStatus::SkippedDocumentBudget
                            | DocumentScanStatus::SkippedByteBudget
                    )
                })
                .map(|entry| entry.declared_bytes)
                .sum(),
        };
        Ok(Self {
            entries,
            role_coverage,
            selection,
        })
    }

    #[must_use]
    pub fn entries(&self) -> &[IndexedDocument] {
        &self.entries
    }

    pub fn scanned_documents(&self) -> impl Iterator<Item = &IndexedDocument> {
        self.entries.iter().filter(|entry| entry.scan.is_some())
    }

    #[must_use]
    pub fn get(&self, path: &str) -> Option<&IndexedDocument> {
        self.entries
            .binary_search_by(|entry| entry.path.as_str().cmp(path))
            .ok()
            .map(|index| &self.entries[index])
    }

    #[must_use]
    pub fn root_readme_document(&self) -> Option<&DocumentScan> {
        self.entries
            .iter()
            .find(|entry| entry.role == DocumentRole::RootReadme)
            .and_then(|entry| entry.scan.as_ref())
    }

    #[must_use]
    pub fn has_root_readme_candidate(&self) -> bool {
        self.entries
            .iter()
            .any(|entry| entry.role == DocumentRole::RootReadme)
    }

    #[must_use]
    pub fn role_coverage(&self) -> &[DocumentRoleCoverage] {
        &self.role_coverage
    }

    #[must_use]
    pub const fn selection(&self) -> DocumentSelectionSummary {
        self.selection
    }

    #[must_use]
    pub fn coverage_for_role(&self, role: DocumentRole) -> Option<CoverageStatus> {
        self.role_coverage
            .iter()
            .find(|coverage| coverage.role == role)
            .map(|coverage| coverage.status)
    }

    #[must_use]
    pub fn with_document_ids(mut self, document_ids: impl Fn(&str) -> Option<DocumentId>) -> Self {
        for entry in &mut self.entries {
            entry.document_id = document_ids(&entry.path);
        }
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentIndexError {
    EmptyPath,
    DuplicatePath(String),
    NonCanonicalEntryOrder,
    ScanStatusMismatch(String),
    DuplicateRoleCoverage(DocumentRole),
    NonCanonicalRoleOrder,
}

impl Display for DocumentIndexError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyPath => formatter.write_str("indexed document path must not be empty"),
            Self::DuplicatePath(path) => write!(formatter, "duplicate indexed document '{path}'"),
            Self::NonCanonicalEntryOrder => {
                formatter.write_str("indexed documents must be sorted by path")
            }
            Self::ScanStatusMismatch(path) => {
                write!(
                    formatter,
                    "indexed document scan status mismatches payload for '{path}'"
                )
            }
            Self::DuplicateRoleCoverage(role) => {
                write!(formatter, "duplicate coverage for {role:?}")
            }
            Self::NonCanonicalRoleOrder => {
                formatter.write_str("document role coverage must be sorted")
            }
        }
    }
}

impl std::error::Error for DocumentIndexError {}
