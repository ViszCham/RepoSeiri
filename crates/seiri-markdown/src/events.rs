use crate::{classify_route, looks_like_badge, DocumentScanOptions, MarkdownError};
use seiri_core::{
    DocumentDiagnostic, DocumentDiagnosticKind, DocumentEvent, DocumentScan, MarkdownBadge,
    MarkdownHeading, MarkdownLink, MarkdownLinkKind, RouteCandidate, RouteKind, RouteSource,
    SourceSpan, TextDocumentBase,
};
use std::collections::BTreeMap;

const MAX_HTML_TAG_BYTES: usize = 2_048;
const MAX_HTML_ATTRIBUTES: usize = 32;

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

    let references = collect_reference_definitions(text);
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

        for mut image in parse_markdown_links(line, true) {
            image.kind = MarkdownLinkKind::Image;
            let image_route = classify_route(&image.text, Some(&image.target));
            image.route = (image_route != RouteKind::Unknown).then_some(image_route);
            push_event(
                &path,
                options,
                &mut events,
                DocumentEvent::Link(image.clone()),
            )?;
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
            } else {
                emit_link_route(&path, options, &mut events, &image)?;
            }
        }

        let mut links = parse_markdown_links(line, false);
        links.extend(parse_reference_links(
            line,
            &references,
            &path,
            options,
            &mut diagnostics,
        )?);
        links.extend(parse_autolinks(line));
        links.extend(parse_html_anchor_links(
            line,
            &path,
            options,
            &mut diagnostics,
        )?);
        for link in links {
            emit_link(&path, options, &mut events, link)?;
        }
    }

    events.sort_by_key(|event| {
        let span = event
            .span()
            .expect("document scanner events always carry spans");
        (span.byte_start, event.order_rank(), span.byte_end)
    });
    diagnostics.sort_by_key(|diagnostic| {
        (
            diagnostic.span.byte_start,
            diagnostic_kind_rank(diagnostic.kind),
            diagnostic.span.byte_end,
        )
    });
    DocumentScan::new(
        path,
        TextDocumentBase::from_bytes(text.as_bytes()),
        events,
        diagnostics,
    )
    .map_err(MarkdownError::Invariant)
}

fn emit_link(
    path: &str,
    options: &DocumentScanOptions,
    events: &mut Vec<DocumentEvent>,
    mut link: MarkdownLink,
) -> Result<(), MarkdownError> {
    let route = classify_route(&link.text, Some(&link.target));
    link.route = (route != RouteKind::Unknown).then_some(route);
    push_event(path, options, events, DocumentEvent::Link(link.clone()))?;
    emit_link_route(path, options, events, &link)
}

fn emit_link_route(
    path: &str,
    options: &DocumentScanOptions,
    events: &mut Vec<DocumentEvent>,
    link: &MarkdownLink,
) -> Result<(), MarkdownError> {
    let route = classify_route(&link.text, Some(&link.target));
    if route == RouteKind::Unknown {
        return Ok(());
    }
    push_event(
        path,
        options,
        events,
        DocumentEvent::RouteCandidate(RouteCandidate {
            route,
            source: RouteSource::Link,
            text: link.text.clone(),
            target: Some(link.target.clone()),
            line: link.line,
            span: link.span,
        }),
    )
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
                kind: MarkdownLinkKind::Inline,
            });
        }
        cursor = target_end + 1;
    }
    out
}

fn collect_reference_definitions(text: &str) -> BTreeMap<String, String> {
    let mut definitions = BTreeMap::new();
    for line in markdown_lines(text) {
        let Some(start) = first_non_whitespace_byte(line.text) else {
            continue;
        };
        let Some(rest) = line.text.get(start..) else {
            continue;
        };
        let Some(label_end) = rest.find("]:") else {
            continue;
        };
        if !rest.starts_with('[') || label_end <= 1 {
            continue;
        }
        let label = normalize_reference_label(&rest[1..label_end]);
        let target = rest[label_end + 2..]
            .trim()
            .split_ascii_whitespace()
            .next()
            .unwrap_or_default()
            .trim_matches(['<', '>']);
        if !label.is_empty() && !target.is_empty() {
            definitions
                .entry(label)
                .or_insert_with(|| target.to_string());
        }
    }
    definitions
}

fn parse_reference_links(
    line: MarkdownLine<'_>,
    definitions: &BTreeMap<String, String>,
    path: &str,
    options: &DocumentScanOptions,
    diagnostics: &mut Vec<DocumentDiagnostic>,
) -> Result<Vec<MarkdownLink>, MarkdownError> {
    let bytes = line.text.as_bytes();
    let mut cursor = 0usize;
    let mut links = Vec::new();
    while cursor < bytes.len() {
        let is_image =
            bytes[cursor] == b'!' && cursor + 1 < bytes.len() && bytes[cursor + 1] == b'[';
        if bytes[cursor] != b'[' && !is_image {
            cursor += 1;
            continue;
        }
        if !is_image && cursor > 0 && bytes[cursor - 1] == b'!' {
            cursor += 1;
            continue;
        }
        let label_start = cursor + usize::from(is_image) + 1;
        let Some(label_end_offset) = line.text[label_start..].find(']') else {
            break;
        };
        let label_end = label_start + label_end_offset;
        let reference_open = label_end + 1;
        if bytes.get(reference_open) != Some(&b'[') {
            if !matches!(bytes.get(reference_open).copied(), Some(b'(' | b':')) {
                let text = line.text[label_start..label_end].trim();
                let reference = normalize_reference_label(text);
                if let Some(target) = definitions.get(&reference) {
                    links.push(MarkdownLink {
                        text: text.to_string(),
                        target: target.clone(),
                        line: line.number,
                        span: Some(source_span(line, cursor, label_end + 1)),
                        route: None,
                        kind: if is_image {
                            MarkdownLinkKind::Image
                        } else {
                            MarkdownLinkKind::Reference
                        },
                    });
                }
            }
            cursor = label_end + 1;
            continue;
        }
        let reference_start = reference_open + 1;
        let Some(reference_end_offset) = line.text[reference_start..].find(']') else {
            cursor = reference_start;
            continue;
        };
        let reference_end = reference_start + reference_end_offset;
        let text = line.text[label_start..label_end].trim();
        let raw_reference = line.text[reference_start..reference_end].trim();
        let reference = normalize_reference_label(if raw_reference.is_empty() {
            text
        } else {
            raw_reference
        });
        match definitions.get(&reference) {
            Some(target) if !text.is_empty() => links.push(MarkdownLink {
                text: text.to_string(),
                target: target.clone(),
                line: line.number,
                span: Some(source_span(line, cursor, reference_end + 1)),
                route: None,
                kind: if is_image {
                    MarkdownLinkKind::Image
                } else {
                    MarkdownLinkKind::Reference
                },
            }),
            _ => push_diagnostic(
                path,
                options,
                diagnostics,
                DocumentDiagnostic {
                    kind: DocumentDiagnosticKind::UnresolvedReferenceLink,
                    span: source_span(line, cursor, reference_end + 1),
                },
            )?,
        }
        cursor = reference_end + 1;
    }
    Ok(links)
}

fn parse_autolinks(line: MarkdownLine<'_>) -> Vec<MarkdownLink> {
    let mut cursor = 0usize;
    let mut links = Vec::new();
    while let Some(open_offset) = line.text[cursor..].find('<') {
        let open = cursor + open_offset;
        let Some(close_offset) = line.text[open + 1..].find('>') else {
            break;
        };
        let close = open + 1 + close_offset;
        let value = line.text[open + 1..close].trim();
        if !value.is_empty()
            && !value.contains(char::is_whitespace)
            && (value.starts_with("http://")
                || value.starts_with("https://")
                || value.starts_with("mailto:")
                || value.contains('@'))
        {
            let target =
                if value.contains('@') && !value.contains("://") && !value.starts_with("mailto:") {
                    format!("mailto:{value}")
                } else {
                    value.to_string()
                };
            links.push(MarkdownLink {
                text: value.to_string(),
                target,
                line: line.number,
                span: Some(source_span(line, open, close + 1)),
                route: None,
                kind: MarkdownLinkKind::Autolink,
            });
        }
        cursor = close + 1;
    }
    links
}

fn parse_html_anchor_links(
    line: MarkdownLine<'_>,
    path: &str,
    options: &DocumentScanOptions,
    diagnostics: &mut Vec<DocumentDiagnostic>,
) -> Result<Vec<MarkdownLink>, MarkdownError> {
    let mut cursor = 0usize;
    let mut links = Vec::new();
    while let Some(open) = find_ascii_case_insensitive(line.text, "<a", cursor) {
        let boundary = line.text.as_bytes().get(open + 2).copied();
        if !boundary.is_some_and(|byte| byte.is_ascii_whitespace() || byte == b'>') {
            cursor = open + 2;
            continue;
        }
        let Some(relative_end) = line.text[open..].find('>') else {
            push_html_diagnostic(
                line,
                open,
                line.text.len(),
                DocumentDiagnosticKind::UnsupportedHtml,
                path,
                options,
                diagnostics,
            )?;
            break;
        };
        let tag_end = open + relative_end;
        if tag_end + 1 - open > MAX_HTML_TAG_BYTES {
            push_html_diagnostic(
                line,
                open,
                tag_end + 1,
                DocumentDiagnosticKind::HtmlAttributeLimitExceeded,
                path,
                options,
                diagnostics,
            )?;
            cursor = tag_end + 1;
            continue;
        }
        let href = match scan_html_href(&line.text[open + 2..tag_end]) {
            Ok(value) => value,
            Err(kind) => {
                push_html_diagnostic(line, open, tag_end + 1, kind, path, options, diagnostics)?;
                cursor = tag_end + 1;
                continue;
            }
        };
        let Some(close_start) = find_ascii_case_insensitive(line.text, "</a>", tag_end + 1) else {
            push_html_diagnostic(
                line,
                open,
                tag_end + 1,
                DocumentDiagnosticKind::UnsupportedHtml,
                path,
                options,
                diagnostics,
            )?;
            cursor = tag_end + 1;
            continue;
        };
        let text = line.text[tag_end + 1..close_start].trim();
        if let Some(target) = href.filter(|target| !target.is_empty()) {
            if !text.is_empty() && !text.contains('<') {
                links.push(MarkdownLink {
                    text: text.to_string(),
                    target,
                    line: line.number,
                    span: Some(source_span(line, open, close_start + 4)),
                    route: None,
                    kind: MarkdownLinkKind::HtmlAnchor,
                });
            } else {
                push_html_diagnostic(
                    line,
                    open,
                    close_start + 4,
                    DocumentDiagnosticKind::UnsupportedHtml,
                    path,
                    options,
                    diagnostics,
                )?;
            }
        }
        cursor = close_start + 4;
    }
    Ok(links)
}

fn scan_html_href(attributes: &str) -> Result<Option<String>, DocumentDiagnosticKind> {
    let bytes = attributes.as_bytes();
    let mut cursor = 0usize;
    let mut count = 0usize;
    let mut href = None;
    while cursor < bytes.len() {
        while bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
            cursor += 1;
        }
        if cursor >= bytes.len() {
            break;
        }
        count += 1;
        if count > MAX_HTML_ATTRIBUTES {
            return Err(DocumentDiagnosticKind::HtmlAttributeLimitExceeded);
        }
        let name_start = cursor;
        while bytes
            .get(cursor)
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            cursor += 1;
        }
        if name_start == cursor {
            return Err(DocumentDiagnosticKind::UnsupportedHtml);
        }
        let name = &attributes[name_start..cursor];
        while bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
            cursor += 1;
        }
        if bytes.get(cursor) != Some(&b'=') {
            continue;
        }
        cursor += 1;
        while bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
            cursor += 1;
        }
        let (value_start, value_end) = match bytes.get(cursor).copied() {
            Some(quote @ (b'\'' | b'"')) => {
                cursor += 1;
                let start = cursor;
                while bytes.get(cursor).copied() != Some(quote) {
                    if cursor >= bytes.len() {
                        return Err(DocumentDiagnosticKind::UnsupportedHtml);
                    }
                    cursor += 1;
                }
                let end = cursor;
                cursor += 1;
                (start, end)
            }
            Some(_) => {
                let start = cursor;
                while bytes
                    .get(cursor)
                    .is_some_and(|byte| !byte.is_ascii_whitespace())
                {
                    cursor += 1;
                }
                (start, cursor)
            }
            None => return Err(DocumentDiagnosticKind::UnsupportedHtml),
        };
        if name.eq_ignore_ascii_case("href") {
            href = Some(attributes[value_start..value_end].to_string());
        }
    }
    Ok(href)
}

fn push_html_diagnostic(
    line: MarkdownLine<'_>,
    start: usize,
    end: usize,
    kind: DocumentDiagnosticKind,
    path: &str,
    options: &DocumentScanOptions,
    diagnostics: &mut Vec<DocumentDiagnostic>,
) -> Result<(), MarkdownError> {
    push_diagnostic(
        path,
        options,
        diagnostics,
        DocumentDiagnostic {
            kind,
            span: source_span(line, start, end),
        },
    )
}

fn normalize_reference_label(value: &str) -> String {
    value
        .split_ascii_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn find_ascii_case_insensitive(haystack: &str, needle: &str, from: usize) -> Option<usize> {
    let haystack = haystack.as_bytes();
    let needle = needle.as_bytes();
    if needle.is_empty() || from > haystack.len() {
        return None;
    }
    haystack[from..]
        .windows(needle.len())
        .position(|window| {
            window
                .iter()
                .zip(needle)
                .all(|(left, right)| left.eq_ignore_ascii_case(right))
        })
        .map(|offset| from + offset)
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
