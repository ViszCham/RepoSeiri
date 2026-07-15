pub(crate) fn mask_hidden_contexts(text: &str) -> String {
    let mut hidden = vec![false; text.len()];
    mark_fenced_code(text.as_bytes(), &mut hidden);
    mark_html_comments(text.as_bytes(), &mut hidden);
    mark_code_spans(text.as_bytes(), &mut hidden);
    mark_escapes(text.as_bytes(), &mut hidden);

    let mut masked = text.as_bytes().to_vec();
    for (index, byte) in masked.iter_mut().enumerate() {
        if hidden[index] && !matches!(*byte, b'\r' | b'\n') {
            *byte = b' ';
        }
    }
    String::from_utf8(masked).expect("masking ASCII positions preserves UTF-8")
}

fn mark_fenced_code(bytes: &[u8], hidden: &mut [bool]) {
    let mut cursor = 0;
    let mut fence = None::<(u8, usize)>;
    while cursor < bytes.len() {
        let line_end = bytes[cursor..]
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(bytes.len(), |offset| cursor + offset + 1);
        let content_end = trim_line_ending(bytes, cursor, line_end);
        if let Some((marker, width)) = fence {
            mark(hidden, cursor, line_end);
            if is_closing_fence(&bytes[cursor..content_end], marker, width) {
                fence = None;
            }
        } else if let Some(opening) = opening_fence(&bytes[cursor..content_end]) {
            mark(hidden, cursor, line_end);
            fence = Some(opening);
        }
        cursor = line_end;
    }
}

fn opening_fence(line: &[u8]) -> Option<(u8, usize)> {
    let marker_start = leading_spaces(line)?;
    let marker = *line.get(marker_start)?;
    if !matches!(marker, b'`' | b'~') {
        return None;
    }
    let width = line[marker_start..]
        .iter()
        .take_while(|byte| **byte == marker)
        .count();
    (width >= 3).then_some((marker, width))
}

fn is_closing_fence(line: &[u8], marker: u8, opening_width: usize) -> bool {
    let Some(marker_start) = leading_spaces(line) else {
        return false;
    };
    let width = line[marker_start..]
        .iter()
        .take_while(|byte| **byte == marker)
        .count();
    width >= opening_width
        && line[marker_start + width..]
            .iter()
            .all(|byte| byte.is_ascii_whitespace())
}

fn leading_spaces(line: &[u8]) -> Option<usize> {
    let count = line.iter().take_while(|byte| **byte == b' ').count();
    (count <= 3).then_some(count)
}

fn mark_html_comments(bytes: &[u8], hidden: &mut [bool]) {
    let mut cursor = 0;
    while let Some(start) = find_visible(bytes, hidden, cursor, b"<!--") {
        let end = find_visible(bytes, hidden, start + 4, b"-->")
            .map_or(bytes.len(), |position| position + 3);
        mark(hidden, start, end);
        cursor = end;
    }
}

fn mark_code_spans(bytes: &[u8], hidden: &mut [bool]) {
    let mut cursor = 0;
    while cursor < bytes.len() {
        if hidden[cursor] || bytes[cursor] != b'`' {
            cursor += 1;
            continue;
        }
        let width = bytes[cursor..]
            .iter()
            .take_while(|byte| **byte == b'`')
            .count();
        let mut candidate = cursor + width;
        let mut closing = None;
        while candidate < bytes.len() {
            if hidden[candidate] || bytes[candidate] != b'`' {
                candidate += 1;
                continue;
            }
            let candidate_width = bytes[candidate..]
                .iter()
                .take_while(|byte| **byte == b'`')
                .count();
            if candidate_width == width {
                closing = Some(candidate + width);
                break;
            }
            candidate += candidate_width;
        }
        if let Some(end) = closing {
            mark(hidden, cursor, end);
            cursor = end;
        } else {
            cursor += width;
        }
    }
}

fn mark_escapes(bytes: &[u8], hidden: &mut [bool]) {
    let mut cursor = 0;
    while cursor + 1 < bytes.len() {
        if !hidden[cursor] && bytes[cursor] == b'\\' && bytes[cursor + 1].is_ascii_punctuation() {
            hidden[cursor + 1] = true;
            cursor += 2;
        } else {
            cursor += 1;
        }
    }
}

fn find_visible(bytes: &[u8], hidden: &[bool], from: usize, needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || from >= bytes.len() {
        return None;
    }
    (from..=bytes.len().saturating_sub(needle.len())).find(|start| {
        !hidden[*start..*start + needle.len()]
            .iter()
            .any(|value| *value)
            && &bytes[*start..*start + needle.len()] == needle
    })
}

fn trim_line_ending(bytes: &[u8], start: usize, mut end: usize) -> usize {
    if end > start && bytes[end - 1] == b'\n' {
        end -= 1;
    }
    if end > start && bytes[end - 1] == b'\r' {
        end -= 1;
    }
    end
}

fn mark(hidden: &mut [bool], start: usize, end: usize) {
    hidden[start..end].fill(true);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masking_preserves_bytes_and_newlines() {
        let source = "# Visible\n```md\n## Security\n[Security](SECURITY.md)\n```\n";
        let masked = mask_hidden_contexts(source);
        assert_eq!(masked.len(), source.len());
        assert_eq!(masked.matches('\n').count(), source.matches('\n').count());
        assert!(masked.contains("# Visible"));
        assert!(!masked.contains("Security"));
    }
}
