use crate::{
    DocumentEvent, DocumentIndex, DocumentRole, DocumentRoleMask, MarkdownEvidenceKind, SourceSpan,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticEvent {
    document_path: String,
    document_role: DocumentRole,
    kind: MarkdownEvidenceKind,
    span: SourceSpan,
    normalized: Box<str>,
}

impl SemanticEvent {
    #[must_use]
    pub fn document_path(&self) -> &str {
        &self.document_path
    }

    #[must_use]
    pub const fn document_role(&self) -> DocumentRole {
        self.document_role
    }

    #[must_use]
    pub const fn kind(&self) -> MarkdownEvidenceKind {
        self.kind
    }

    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SemanticIndex {
    events: Vec<SemanticEvent>,
    token_postings: BTreeMap<Box<str>, Vec<u32>>,
}

impl SemanticIndex {
    #[must_use]
    pub fn build(documents: &DocumentIndex) -> Self {
        let mut events = Vec::new();
        let mut token_postings = BTreeMap::<Box<str>, Vec<u32>>::new();
        for entry in documents.scanned_documents() {
            let Some(scan) = entry.scan.as_ref() else {
                continue;
            };
            for event in scan.events() {
                let Some((kind, span, searchable)) = searchable_event(event) else {
                    continue;
                };
                let normalized = normalize_searchable(&searchable);
                if normalized.is_empty() {
                    continue;
                }
                let event_id = u32::try_from(events.len()).unwrap_or(u32::MAX);
                let unique_tokens = tokenize(&normalized).collect::<BTreeSet<_>>();
                for token in unique_tokens {
                    token_postings
                        .entry(token.into())
                        .or_default()
                        .push(event_id);
                }
                events.push(SemanticEvent {
                    document_path: entry.path.clone(),
                    document_role: entry.role,
                    kind,
                    span,
                    normalized: normalized.into(),
                });
            }
        }
        Self {
            events,
            token_postings,
        }
    }

    #[must_use]
    pub fn matching_events(
        &self,
        markers: &[&str],
        roles: DocumentRoleMask,
    ) -> Vec<&SemanticEvent> {
        let mut matches = BTreeSet::new();
        for marker in markers {
            let normalized = normalize_searchable(marker);
            let Some(first_token) = tokenize(&normalized).next() else {
                continue;
            };
            let Some(postings) = self.token_postings.get(first_token) else {
                continue;
            };
            for event_id in postings {
                let Some(event) = self.events.get(*event_id as usize) else {
                    continue;
                };
                if roles.contains(event.document_role)
                    && contains_bounded_phrase(&event.normalized, &normalized)
                {
                    matches.insert(*event_id);
                }
            }
        }
        matches
            .into_iter()
            .filter_map(|id| self.events.get(id as usize))
            .collect()
    }

    #[must_use]
    pub fn events(&self) -> &[SemanticEvent] {
        &self.events
    }
}

fn searchable_event(event: &DocumentEvent) -> Option<(MarkdownEvidenceKind, SourceSpan, String)> {
    match event {
        DocumentEvent::VisibleProse(value) => Some((
            MarkdownEvidenceKind::VisibleProse,
            value.span,
            value.text.clone(),
        )),
        DocumentEvent::Heading(value) => Some((
            MarkdownEvidenceKind::Heading,
            value.span?,
            value.text.clone(),
        )),
        DocumentEvent::Link(value) => Some((
            MarkdownEvidenceKind::Link,
            value.span?,
            format!("{} {}", value.text, value.target),
        )),
        DocumentEvent::Badge(value) => Some((
            MarkdownEvidenceKind::Badge,
            value.span?,
            format!("{} {}", value.alt, value.target),
        )),
        DocumentEvent::RouteCandidate(value) => Some((
            MarkdownEvidenceKind::RouteCandidate,
            value.span?,
            value.target.as_ref().map_or_else(
                || value.text.clone(),
                |target| format!("{} {target}", value.text),
            ),
        )),
    }
}

fn normalize_searchable(value: &str) -> String {
    value
        .chars()
        .flat_map(char::to_lowercase)
        .map(|character| {
            if character.is_alphanumeric() || character == '_' {
                character
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn tokenize(value: &str) -> impl Iterator<Item = &str> {
    value.split_whitespace()
}

fn contains_bounded_phrase(value: &str, marker: &str) -> bool {
    if value == marker {
        return true;
    }
    value.match_indices(marker).any(|(start, matched)| {
        let end = start + matched.len();
        let left_ok = start == 0 || value[..start].ends_with(' ');
        let right_ok = end == value.len() || value[end..].starts_with(' ');
        left_ok && right_ok
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_phrase_does_not_match_license_inside_licensed() {
        assert!(contains_bounded_phrase("license route", "license"));
        assert!(!contains_bounded_phrase("licensed under", "license"));
        assert!(!contains_bounded_phrase("sublicense", "license"));
    }
}
