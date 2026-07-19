use std::fmt::{Display, Formatter};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
enum SourcePayload {
    Text(Arc<str>),
    Bytes(Arc<[u8]>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceDocument {
    path: String,
    payload: SourcePayload,
}

impl SourceDocument {
    #[must_use]
    pub fn from_text(path: String, text: Arc<str>) -> Self {
        Self {
            path,
            payload: SourcePayload::Text(text),
        }
    }

    #[must_use]
    pub fn from_bytes(path: String, bytes: Vec<u8>) -> Self {
        Self {
            path,
            payload: SourcePayload::Bytes(Arc::from(bytes)),
        }
    }

    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        match &self.payload {
            SourcePayload::Text(text) => text.as_bytes(),
            SourcePayload::Bytes(bytes) => bytes,
        }
    }

    #[must_use]
    pub fn text(&self) -> Option<&str> {
        match &self.payload {
            SourcePayload::Text(text) => Some(text),
            SourcePayload::Bytes(bytes) => std::str::from_utf8(bytes).ok(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceStore {
    documents: Vec<SourceDocument>,
    total_bytes: usize,
}

impl SourceStore {
    pub fn try_new(documents: Vec<SourceDocument>) -> Result<Self, SourceStoreError> {
        let mut previous = None;
        let mut total_bytes = 0usize;
        for document in &documents {
            if document.path.is_empty() {
                return Err(SourceStoreError::EmptyPath);
            }
            if previous.is_some_and(|path: &str| path >= document.path()) {
                return Err(if previous == Some(document.path()) {
                    SourceStoreError::DuplicatePath(document.path.clone())
                } else {
                    SourceStoreError::NonCanonicalOrder
                });
            }
            total_bytes = total_bytes
                .checked_add(document.bytes().len())
                .ok_or(SourceStoreError::TotalBytesOverflow)?;
            previous = Some(document.path());
        }
        Ok(Self {
            documents,
            total_bytes,
        })
    }

    #[must_use]
    pub fn documents(&self) -> &[SourceDocument] {
        &self.documents
    }

    #[must_use]
    pub fn get(&self, path: &str) -> Option<&SourceDocument> {
        self.documents
            .binary_search_by(|document| document.path().cmp(path))
            .ok()
            .map(|index| &self.documents[index])
    }

    #[must_use]
    pub const fn total_bytes(&self) -> usize {
        self.total_bytes
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceStoreError {
    EmptyPath,
    DuplicatePath(String),
    NonCanonicalOrder,
    TotalBytesOverflow,
}

impl Display for SourceStoreError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyPath => formatter.write_str("source document path must not be empty"),
            Self::DuplicatePath(path) => write!(formatter, "duplicate source document '{path}'"),
            Self::NonCanonicalOrder => {
                formatter.write_str("source documents must be sorted by path")
            }
            Self::TotalBytesOverflow => {
                formatter.write_str("source document total byte count overflowed")
            }
        }
    }
}

impl std::error::Error for SourceStoreError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_is_sorted_unique_and_borrows_bytes_without_serializing_them() {
        let store = SourceStore::try_new(vec![
            SourceDocument::from_bytes("a.yml".into(), b"a: 1".to_vec()),
            SourceDocument::from_text("readme.md".into(), Arc::from("# Readme")),
        ])
        .expect("source store");
        assert_eq!(store.total_bytes(), 12);
        assert_eq!(store.get("a.yml").expect("yaml").text(), Some("a: 1"));
        assert_eq!(store.get("readme.md").expect("readme").bytes(), b"# Readme");
        assert!(SourceStore::try_new(vec![
            SourceDocument::from_bytes("same".into(), vec![]),
            SourceDocument::from_bytes("same".into(), vec![]),
        ])
        .is_err());
    }
}
