use crate::{DocumentEvent, DocumentIndex, DocumentScan};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DocumentLanguage {
    Japanese,
    English,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageTopology {
    Monolingual(DocumentLanguage),
    Parallel {
        japanese_insertion: usize,
        english_insertion: usize,
    },
    Ambiguous,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentLanguageTopology {
    path: String,
    topology: LanguageTopology,
}

impl DocumentLanguageTopology {
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    #[must_use]
    pub const fn topology(&self) -> LanguageTopology {
        self.topology
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LanguageTopologyIndex {
    documents: Vec<DocumentLanguageTopology>,
}

impl LanguageTopologyIndex {
    #[must_use]
    pub fn build(documents: &DocumentIndex) -> Self {
        let mut topologies = documents
            .scanned_documents()
            .filter_map(|entry| {
                entry.scan.as_ref().map(|scan| DocumentLanguageTopology {
                    path: entry.path.clone(),
                    topology: detect_language_topology(scan),
                })
            })
            .collect::<Vec<_>>();
        topologies.sort_by(|left, right| left.path.cmp(&right.path));
        Self {
            documents: topologies,
        }
    }

    #[must_use]
    pub fn for_path(&self, path: &str) -> Option<LanguageTopology> {
        self.documents
            .binary_search_by(|entry| entry.path().cmp(path))
            .ok()
            .map(|index| self.documents[index].topology())
    }

    #[must_use]
    pub fn documents(&self) -> &[DocumentLanguageTopology] {
        &self.documents
    }
}

fn detect_language_topology(scan: &DocumentScan) -> LanguageTopology {
    let mut japanese_anchor = None;
    let mut english_anchor = None;
    let mut japanese_events = 0usize;
    let mut english_events = 0usize;
    let mut mixed_language_heading = false;
    for event in scan.events() {
        let text = match event {
            DocumentEvent::VisibleProse(value) => value.text.as_str(),
            DocumentEvent::Heading(value) => value.text.as_str(),
            DocumentEvent::Link(value) => value.text.as_str(),
            DocumentEvent::Badge(value) => value.alt.as_str(),
            DocumentEvent::RouteCandidate(value) => value.text.as_str(),
        };
        if contains_japanese(text) {
            japanese_events = japanese_events.saturating_add(1);
        } else if text
            .split(|character: char| !character.is_ascii_alphabetic())
            .any(|word| word.len() >= 2)
        {
            english_events = english_events.saturating_add(1);
        }
        if let DocumentEvent::Heading(heading) = event {
            let normalized = normalize_heading(&heading.text);
            mixed_language_heading |= contains_japanese(&heading.text)
                && normalized.split_whitespace().any(|word| word == "english");
            if japanese_anchor.is_none()
                && (heading.text.contains("日本語") || normalized == "japanese")
            {
                japanese_anchor = heading.span;
            }
            if english_anchor.is_none() && normalized == "english" {
                english_anchor = heading.span;
            }
        }
    }
    if mixed_language_heading {
        return LanguageTopology::Ambiguous;
    }
    match (japanese_anchor, english_anchor) {
        (Some(japanese_anchor), Some(english_anchor)) => {
            let source_end = scan.base().byte_len();
            if japanese_anchor.byte_start < english_anchor.byte_start {
                LanguageTopology::Parallel {
                    japanese_insertion: english_anchor.byte_start,
                    english_insertion: source_end,
                }
            } else {
                LanguageTopology::Parallel {
                    japanese_insertion: source_end,
                    english_insertion: japanese_anchor.byte_start,
                }
            }
        }
        (Some(_), None) if english_events <= 1 => {
            LanguageTopology::Monolingual(DocumentLanguage::Japanese)
        }
        (None, Some(_)) if japanese_events == 0 => {
            LanguageTopology::Monolingual(DocumentLanguage::English)
        }
        (None, None) if japanese_events > 0 && english_events <= 1 => {
            LanguageTopology::Monolingual(DocumentLanguage::Japanese)
        }
        (None, None) if english_events > 0 && japanese_events == 0 => {
            LanguageTopology::Monolingual(DocumentLanguage::English)
        }
        _ => LanguageTopology::Ambiguous,
    }
}

fn normalize_heading(value: &str) -> String {
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

fn contains_japanese(value: &str) -> bool {
    value.chars().any(|character| {
        matches!(
            character,
            '\u{3040}'..='\u{30ff}' | '\u{3400}'..='\u{4dbf}' | '\u{4e00}'..='\u{9fff}'
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MarkdownHeading, MarkdownProse, SourceSpan, TextDocumentBase};

    #[test]
    fn language_topology_finds_nonadjacent_japanese_and_english_sections() {
        let base = TextDocumentBase::from_bytes(b"# Repo\n## \xe6\x97\xa5\xe6\x9c\xac\xe8\xaa\x9e\ntext\n### Detail\n## English\ntext\n");
        let scan = DocumentScan::new(
            "README.md".into(),
            base,
            vec![
                DocumentEvent::Heading(MarkdownHeading {
                    level: 2,
                    text: "日本語".into(),
                    line: 2,
                    span: Some(SourceSpan::new(2, 4, 9, 18)),
                }),
                DocumentEvent::VisibleProse(MarkdownProse {
                    text: "説明".into(),
                    line: 3,
                    span: SourceSpan::new(3, 1, 19, 25),
                }),
                DocumentEvent::Heading(MarkdownHeading {
                    level: 3,
                    text: "Detail".into(),
                    line: 4,
                    span: Some(SourceSpan::new(4, 5, 29, 35)),
                }),
                DocumentEvent::Heading(MarkdownHeading {
                    level: 2,
                    text: "English".into(),
                    line: 5,
                    span: Some(SourceSpan::new(5, 4, 39, 46)),
                }),
            ],
            vec![],
        )
        .expect("scan");
        assert_eq!(
            detect_language_topology(&scan),
            LanguageTopology::Parallel {
                japanese_insertion: 39,
                english_insertion: 52,
            }
        );
    }

    #[test]
    fn japanese_document_tolerates_one_ascii_product_heading() {
        let base = TextDocumentBase::from_bytes(
            b"# RepoSeiri\n## \xe6\x97\xa5\xe6\x9c\xac\xe8\xaa\x9e\n\xe8\xaa\xac\xe6\x98\x8e\n",
        );
        let scan = DocumentScan::new(
            "README.md".into(),
            base,
            vec![
                DocumentEvent::Heading(MarkdownHeading {
                    level: 1,
                    text: "RepoSeiri".into(),
                    line: 1,
                    span: Some(SourceSpan::new(1, 3, 2, 11)),
                }),
                DocumentEvent::Heading(MarkdownHeading {
                    level: 2,
                    text: "日本語".into(),
                    line: 2,
                    span: Some(SourceSpan::new(2, 4, 15, 24)),
                }),
                DocumentEvent::VisibleProse(MarkdownProse {
                    text: "説明".into(),
                    line: 3,
                    span: SourceSpan::new(3, 1, 25, 31),
                }),
            ],
            vec![],
        )
        .expect("scan");

        assert_eq!(
            detect_language_topology(&scan),
            LanguageTopology::Monolingual(DocumentLanguage::Japanese)
        );
    }
}
