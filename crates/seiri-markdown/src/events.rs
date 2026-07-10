use crate::{classify_route, looks_like_badge, DocumentScanOptions, MarkdownError};
use seiri_core::{
    DocumentDiagnostic, DocumentDiagnosticKind, DocumentEvent, DocumentScan, MarkdownBadge,
    MarkdownHeading, MarkdownLink, RouteCandidate, RouteKind, RouteSource, SourceSpan,
    TextDocumentBase,
};

pub(crate) fn scan_text(
    path: String,
    text: &str,
    options: &DocumentScanOptions,
) -> Result<DocumentScan, MarkdownError> {
    if text.len() > options.max_source_bytes {
        return Err(MarkdownError::SourceLimitExceeded {
            path,
            bytes: text.len(),
            limit: options.max_source_bytes,
        });
    }

    let mut events = Vec::new();
    let mut diagnostics = Vec::new();
    for line in markdown_lines(text) {
        diagnose_malformed_links(line, &path, options, &mut diagnostics)?;

        if let Some(heading) = parse_heading(line) {
            let route = classify_route(&heading.text, None);
            push_event(
                &path,
                options,
                &mut events,
                DocumentEvent::Heading(heading.clone()),
            )?;
            if route != RouteKind::Unknown {
                push_event(
                    &path,
                    options,
                    &mut events,
                    DocumentEvent::RouteCandidate(RouteCandidate {
                        route,
                        source: RouteSource::Heading,
                        text: heading.text,
                        target: None,
                        line: line.number,
                        span: heading.span,
                    }),
                )?;
            }
        }

        for image in parse_markdown_links(line, true) {
            if looks_like_badge(&image.text, &image.target) {
                let badge = MarkdownBadge {
                    alt: image.text.clone(),
                    target: image.target.clone(),
                    line: line.number,
                    span: image.span,
                };
                push_event(&path, options, &mut events, DocumentEvent::Badge(badge))?;
                push_event(
                    &path,
                    options,
                    &mut events,
                    DocumentEvent::RouteCandidate(RouteCandidate {
                        route: RouteKind::Automation,
                        source: RouteSource::Badge,
                        text: image.text,
                        target: Some(image.target),
                        line: line.number,
                        span: image.span,
                    }),
                )?;
            }
        }

        for mut link in parse_markdown_links(line, false) {
            let route = classify_route(&link.text, Some(&link.target));
            link.route = (route != RouteKind::Unknown).then_some(route);
            push_event(
                &path,
                options,
                &mut events,
                DocumentEvent::Link(link.clone()),
            )?;
            if route != RouteKind::Unknown {
                push_event(
                    &path,
                    options,
                    &mut events,
                    DocumentEvent::RouteCandidate(RouteCandidate {
                        route,
                        source: RouteSource::Link,
                        text: link.text,
                        target: Some(link.target),
                        line: line.number,
                        span: link.span,
                    }),
                )?;
            }
        }
    }

    events.sort_by_key(|event| {
        let span = event
            .span()
            .expect("document scanner events always carry spans");
        (span.byte_start, event.order_rank(), span.byte_end)
    });
    DocumentScan::new(
        path,
        TextDocumentBase::from_bytes(text.as_bytes()),
        events,
        diagnostics,
    )
    .map_err(MarkdownError::Invariant)
}

fn push_event(
    path: &str,
    options: &DocumentScanOptions,
    events: &mut Vec<DocumentEvent>,
    event: DocumentEvent,
) -> Result<(), MarkdownError> {
    if events.len() >= options.max_events {
        return Err(MarkdownError::EventLimitExceeded {
            path: path.into(),
            limit: options.max_events,
        });
    }
    events.push(event);
    Ok(())
}

fn push_diagnostic(
    path: &str,
    options: &DocumentScanOptions,
    diagnostics: &mut Vec<DocumentDiagnostic>,
    diagnostic: DocumentDiagnostic,
) -> Result<(), MarkdownError> {
    if diagnostics.len() >= options.max_diagnostics {
        return Err(MarkdownError::DiagnosticLimitExceeded {
            path: path.into(),
            limit: options.max_diagnostics,
        });
    }
    diagnostics.push(diagnostic);
    Ok(())
}

fn diagnose_malformed_links(
    line: MarkdownLine<'_>,
    path: &str,
    options: &DocumentScanOptions,
    diagnostics: &mut Vec<DocumentDiagnostic>,
) -> Result<(), MarkdownError> {
    let bytes = line.text.as_bytes();
    let mut cursor = 0;
    while cursor < bytes.len() {
        let is_image =
            bytes[cursor] == b'!' && cursor + 1 < bytes.len() && bytes[cursor + 1] == b'[';
        if bytes[cursor] != b'[' && !is_image {
            cursor += 1;
            continue;
        }

        let label_start = cursor + usize::from(is_image) + 1;
        let Some(label_end_offset) = line.text[label_start..].find(']') else {
            push_diagnostic(
                path,
                options,
                diagnostics,
                DocumentDiagnostic {
                    kind: DocumentDiagnosticKind::UnclosedLinkLabel,
                    span: source_span(line, cursor, line.text.len()),
                },
            )?;
            break;
        };
        let label_end = label_start + label_end_offset;
        let open_paren = label_end + 1;
        if bytes.get(open_paren) != Some(&b'(') {
            cursor = label_end + 1;
            continue;
        }
        let target_start = open_paren + 1;
        let Some(target_end_offset) = line.text[target_start..].find(')') else {
            push_diagnostic(
                path,
                options,
                diagnostics,
                DocumentDiagnostic {
                    kind: DocumentDiagnosticKind::UnclosedLinkTarget,
                    span: source_span(line, cursor, line.text.len()),
                },
            )?;
            break;
        };
        cursor = target_start + target_end_offset + 1;
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct MarkdownLine<'a> {
    number: usize,
    byte_start: usize,
    text: &'a str,
}

fn markdown_lines(text: &str) -> impl Iterator<Item = MarkdownLine<'_>> {
    let mut number = 0;
    let mut byte_start = 0;
    text.split_inclusive('\n').map(move |segment| {
        number += 1;
        let current_start = byte_start;
        byte_start += segment.len();
        let line = if let Some(line) = segment.strip_suffix('\n') {
            line.strip_suffix('\r').unwrap_or(line)
        } else {
            segment
        };
        MarkdownLine {
            number,
            byte_start: current_start,
            text: line,
        }
    })
}

fn parse_heading(line: MarkdownLine<'_>) -> Option<MarkdownHeading> {
    let marker_start = first_non_whitespace_byte(line.text)?;
    let trimmed = &line.text[marker_start..];
    let level = trimmed.chars().take_while(|value| *value == '#').count();
    if !(1..=6).contains(&level) {
        return None;
    }
    let rest = trimmed.get(level..)?;
    if !rest.starts_with(' ') {
        return None;
    }
    let text = rest.trim().trim_end_matches('#').trim();
    if text.is_empty() {
        return None;
    }
    Some(MarkdownHeading {
        level: level as u8,
        text: text.to_string(),
        line: line.number,
        span: Some(source_span(line, marker_start, line.text.len())),
    })
}

fn parse_markdown_links(line: MarkdownLine<'_>, images_only: bool) -> Vec<MarkdownLink> {
    let bytes = line.text.as_bytes();
    let mut cursor = 0;
    let mut out = Vec::new();

    while cursor < bytes.len() {
        let is_image =
            bytes[cursor] == b'!' && cursor + 1 < bytes.len() && bytes[cursor + 1] == b'[';
        let starts_link = bytes[cursor] == b'[' || is_image;
        if !starts_link {
            cursor += 1;
            continue;
        }
        if images_only != is_image {
            cursor += if is_image { 2 } else { 1 };
            continue;
        }

        let label_start = cursor + usize::from(is_image) + 1;
        let Some(label_end_offset) = line.text[label_start..].find(']') else {
            cursor += 1;
            continue;
        };
        let label_end = label_start + label_end_offset;
        let open_paren = label_end + 1;
        if bytes.get(open_paren) != Some(&b'(') {
            cursor = label_end + 1;
            continue;
        }
        let target_start = open_paren + 1;
        let Some(target_end_offset) = line.text[target_start..].find(')') else {
            cursor = target_start;
            continue;
        };
        let target_end = target_start + target_end_offset;
        let text = line.text[label_start..label_end].trim();
        let target = line.text[target_start..target_end].trim();
        if !text.is_empty() && !target.is_empty() {
            out.push(MarkdownLink {
                text: text.to_string(),
                target: target.to_string(),
                line: line.number,
                span: Some(source_span(line, cursor, target_end + 1)),
                route: None,
            });
        }
        cursor = target_end + 1;
    }
    out
}

fn first_non_whitespace_byte(line: &str) -> Option<usize> {
    line.char_indices()
        .find(|(_, character)| !character.is_whitespace())
        .map(|(index, _)| index)
}

fn source_span(line: MarkdownLine<'_>, start_in_line: usize, end_in_line: usize) -> SourceSpan {
    SourceSpan::new(
        line.number,
        column_for_byte(line.text, start_in_line),
        line.byte_start + start_in_line,
        line.byte_start + end_in_line,
    )
}

fn column_for_byte(line: &str, byte_index: usize) -> usize {
    line[..byte_index].chars().count() + 1
}
