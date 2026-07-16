use crate::{
    classify_route, context::mask_hidden_contexts, looks_like_badge, DocumentScanOptions,
    MarkdownError,
};
use pulldown_cmark::{Event, HeadingLevel, LinkType, Options, Parser, Tag, TagEnd};
use seiri_core::{
    DocumentDiagnostic, DocumentDiagnosticKind, DocumentEvent, DocumentScan, MarkdownBadge,
    MarkdownHeading, MarkdownLink, MarkdownLinkKind, RouteCandidate, RouteKind, RouteSource,
    SourceSpan, TextDocumentBase,
};
use std::collections::BTreeSet;
use std::ops::Range;

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

    let lines = LineIndex::new(text);
    let visible = mask_hidden_contexts(text);
    let mut events = semantic_events(&path, &visible, &lines, options)?;
    let references = collect_reference_definitions(&visible);
    let mut diagnostics = Vec::new();
    for line in markdown_lines(&visible) {
        diagnose_malformed_links(line, &path, options, &mut diagnostics)?;
        diagnose_unresolved_references(line, &references, &path, options, &mut diagnostics)?;
        for link in parse_html_anchor_links(line, &path, options, &mut diagnostics)? {
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
    diagnostics.dedup();
    DocumentScan::new(
        path,
        TextDocumentBase::from_bytes(text.as_bytes()),
        events,
        diagnostics,
    )
    .map_err(MarkdownError::Invariant)
}

fn semantic_events(
    path: &str,
    text: &str,
    lines: &LineIndex,
    options: &DocumentScanOptions,
) -> Result<Vec<DocumentEvent>, MarkdownError> {
    let parser = Parser::new_ext(text, Options::empty()).into_offset_iter();
    let mut output = Vec::new();
    let mut heading = None::<OpenHeading>;
    let mut links = Vec::<OpenLink>::new();

    for (event, range) in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                heading = Some(OpenHeading {
                    level: heading_level(level),
                    start: range.start,
                    text: String::new(),
                });
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some(open) = heading.take() {
                    let end = trim_trailing_line_ending(text, range.end);
                    let value = MarkdownHeading {
                        level: open.level,
                        text: normalized_visible_text(&open.text),
                        line: lines.line_for(open.start),
                        span: Some(lines.span(open.start..end)),
                    };
                    if !value.text.is_empty() {
                        let route = classify_route(&value.text, None);
                        push_event(
                            path,
                            options,
                            &mut output,
                            DocumentEvent::Heading(value.clone()),
                        )?;
                        if route != RouteKind::Unknown {
                            push_event(
                                path,
                                options,
                                &mut output,
                                DocumentEvent::RouteCandidate(RouteCandidate {
                                    route,
                                    source: RouteSource::Heading,
                                    text: value.text,
                                    target: None,
                                    line: value.line,
                                    span: value.span,
                                }),
                            )?;
                        }
                    }
                }
            }
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                ..
            }) => links.push(OpenLink::new(
                range.start,
                dest_url.into_string(),
                markdown_link_kind(link_type, false),
                false,
            )),
            Event::Start(Tag::Image {
                link_type,
                dest_url,
                ..
            }) => links.push(OpenLink::new(
                range.start,
                dest_url.into_string(),
                markdown_link_kind(link_type, true),
                true,
            )),
            Event::End(TagEnd::Link) => {
                if let Some(open) = take_open_link(&mut links, false) {
                    emit_open_link(path, options, &mut output, open, range.end, lines)?;
                }
            }
            Event::End(TagEnd::Image) => {
                if let Some(open) = take_open_link(&mut links, true) {
                    emit_open_link(path, options, &mut output, open, range.end, lines)?;
                }
            }
            Event::Text(value) => {
                if let Some(open) = heading.as_mut() {
                    open.text.push_str(&value);
                }
                for open in &mut links {
                    open.text.push_str(&value);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if let Some(open) = heading.as_mut() {
                    open.text.push(' ');
                }
                for open in &mut links {
                    open.text.push(' ');
                }
            }
            Event::Code(_)
            | Event::Html(_)
            | Event::InlineHtml(_)
            | Event::InlineMath(_)
            | Event::DisplayMath(_)
            | Event::Rule
            | Event::TaskListMarker(_)
            | Event::FootnoteReference(_)
            | Event::Start(_)
            | Event::End(_) => {}
        }
    }

    Ok(output)
}

#[derive(Debug)]
struct OpenHeading {
    level: u8,
    start: usize,
    text: String,
}

#[derive(Debug)]
struct OpenLink {
    start: usize,
    target: String,
    text: String,
    kind: MarkdownLinkKind,
    image: bool,
}

impl OpenLink {
    fn new(start: usize, target: String, kind: MarkdownLinkKind, image: bool) -> Self {
        Self {
            start,
            target,
            text: String::new(),
            kind,
            image,
        }
    }
}

fn take_open_link(links: &mut Vec<OpenLink>, image: bool) -> Option<OpenLink> {
    let index = links.iter().rposition(|open| open.image == image)?;
    Some(links.remove(index))
}

fn emit_open_link(
    path: &str,
    options: &DocumentScanOptions,
    events: &mut Vec<DocumentEvent>,
    open: OpenLink,
    end: usize,
    lines: &LineIndex,
) -> Result<(), MarkdownError> {
    let link = MarkdownLink {
        text: normalized_visible_text(&open.text),
        target: open.target,
        line: lines.line_for(open.start),
        span: Some(lines.span(open.start..end)),
        route: None,
        kind: open.kind,
    };
    if link.text.is_empty() || link.target.is_empty() {
        return Ok(());
    }
    if open.image {
        emit_image(path, options, events, link)
    } else {
        emit_link(path, options, events, link)
    }
}

fn emit_image(
    path: &str,
    options: &DocumentScanOptions,
    events: &mut Vec<DocumentEvent>,
    mut link: MarkdownLink,
) -> Result<(), MarkdownError> {
    link.kind = MarkdownLinkKind::Image;
    let route = classify_route(&link.text, Some(&link.target));
    link.route = (route != RouteKind::Unknown).then_some(route);
    push_event(path, options, events, DocumentEvent::Link(link.clone()))?;
    if looks_like_badge(&link.text, &link.target) {
        let badge = MarkdownBadge {
            alt: link.text.clone(),
            target: link.target.clone(),
            line: link.line,
            span: link.span,
        };
        push_event(path, options, events, DocumentEvent::Badge(badge))?;
        push_event(
            path,
            options,
            events,
            DocumentEvent::RouteCandidate(RouteCandidate {
                route: RouteKind::Automation,
                source: RouteSource::Badge,
                text: link.text,
                target: Some(link.target),
                line: link.line,
                span: link.span,
            }),
        )
    } else {
        emit_link_route(path, options, events, &link)
    }
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

fn diagnose_malformed_links(
    line: MarkdownLine<'_>,
    path: &str,
    options: &DocumentScanOptions,
    diagnostics: &mut Vec<DocumentDiagnostic>,
) -> Result<(), MarkdownError> {
    let bytes = line.text.as_bytes();
    let mut cursor = 0;
    while cursor < bytes.len() {
        let image = bytes[cursor] == b'!' && bytes.get(cursor + 1) == Some(&b'[');
        if bytes[cursor] != b'[' && !image {
            cursor += 1;
            continue;
        }
        let label_start = cursor + usize::from(image) + 1;
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
        if bytes.get(label_end + 1) != Some(&b'(') {
            cursor = label_end + 1;
            continue;
        }
        let target_start = label_end + 2;
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

fn collect_reference_definitions(text: &str) -> BTreeSet<String> {
    markdown_lines(text)
        .filter_map(|line| {
            let rest = line.text.strip_prefix('[')?;
            let end = rest.find("]: ").or_else(|| rest.find("]:"))?;
            let label = normalize_reference_label(&rest[..end]);
            (!label.is_empty()).then_some(label)
        })
        .collect()
}

fn diagnose_unresolved_references(
    line: MarkdownLine<'_>,
    definitions: &BTreeSet<String>,
    path: &str,
    options: &DocumentScanOptions,
    diagnostics: &mut Vec<DocumentDiagnostic>,
) -> Result<(), MarkdownError> {
    let bytes = line.text.as_bytes();
    let mut cursor = 0;
    while cursor < bytes.len() {
        let image = bytes[cursor] == b'!' && bytes.get(cursor + 1) == Some(&b'[');
        if bytes[cursor] != b'[' && !image {
            cursor += 1;
            continue;
        }
        let label_start = cursor + usize::from(image) + 1;
        let Some(label_end_offset) = line.text[label_start..].find(']') else {
            break;
        };
        let label_end = label_start + label_end_offset;
        if bytes.get(label_end + 1) != Some(&b'[') {
            cursor = label_end + 1;
            continue;
        }
        let reference_start = label_end + 2;
        let Some(reference_end_offset) = line.text[reference_start..].find(']') else {
            break;
        };
        let reference_end = reference_start + reference_end_offset;
        let raw = line.text[reference_start..reference_end].trim();
        let label = if raw.is_empty() {
            &line.text[label_start..label_end]
        } else {
            raw
        };
        if !definitions.contains(&normalize_reference_label(label)) {
            push_diagnostic(
                path,
                options,
                diagnostics,
                DocumentDiagnostic {
                    kind: DocumentDiagnosticKind::UnresolvedReferenceLink,
                    span: source_span(line, cursor, reference_end + 1),
                },
            )?;
        }
        cursor = reference_end + 1;
    }
    Ok(())
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

fn markdown_link_kind(link_type: LinkType, image: bool) -> MarkdownLinkKind {
    if image {
        return MarkdownLinkKind::Image;
    }
    match link_type {
        LinkType::Inline | LinkType::WikiLink { .. } => MarkdownLinkKind::Inline,
        LinkType::Autolink | LinkType::Email => MarkdownLinkKind::Autolink,
        LinkType::Reference
        | LinkType::ReferenceUnknown
        | LinkType::Collapsed
        | LinkType::CollapsedUnknown
        | LinkType::Shortcut
        | LinkType::ShortcutUnknown => MarkdownLinkKind::Reference,
    }
}

const fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn normalized_visible_text(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn trim_trailing_line_ending(source: &str, mut end: usize) -> usize {
    if end > 0 && source.as_bytes()[end - 1] == b'\n' {
        end -= 1;
    }
    if end > 0 && source.as_bytes()[end - 1] == b'\r' {
        end -= 1;
    }
    end
}

fn normalize_reference_label(value: &str) -> String {
    normalized_visible_text(value).to_ascii_lowercase()
}

fn find_ascii_case_insensitive(haystack: &str, needle: &str, from: usize) -> Option<usize> {
    let haystack = haystack.as_bytes();
    let needle = needle.as_bytes();
    if needle.is_empty() || from > haystack.len() || needle.len() > haystack.len() - from {
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

fn source_span(line: MarkdownLine<'_>, start: usize, end: usize) -> SourceSpan {
    SourceSpan::new(
        line.number,
        line.text[..start].chars().count() + 1,
        line.byte_start + start,
        line.byte_start + end,
    )
}

struct LineIndex<'a> {
    source: &'a str,
    starts: Vec<usize>,
}

impl<'a> LineIndex<'a> {
    fn new(source: &'a str) -> Self {
        let mut starts = vec![0];
        starts.extend(source.match_indices('\n').map(|(index, _)| index + 1));
        Self { source, starts }
    }

    fn line_for(&self, byte: usize) -> usize {
        self.starts.partition_point(|start| *start <= byte).max(1)
    }

    fn span(&self, range: Range<usize>) -> SourceSpan {
        let line = self.line_for(range.start);
        let line_start = self.starts[line - 1];
        SourceSpan::new(
            line,
            self.source[line_start..range.start].chars().count() + 1,
            range.start,
            range.end,
        )
    }
}
