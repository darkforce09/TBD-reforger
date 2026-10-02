//! The link destinations of a Markdown document.
//!
//! **Role:** finds the byte span of every inline link and image destination (`[text](dest)`,
//! `![alt](<dest>)`) and every reference definition (`[label]: dest`), outside fenced code blocks
//! and inline code spans.
//!
//! **Position:** read by [`super::relative_references`], which resolves relative destinations
//! against the document's folder, and by [`super`], which opens only these spans in frozen
//! records.
//!
//! **Signals & state:** none; pure functions over the document text.
//!
//! **Invariants:** a destination span excludes its angle brackets and any link title; a `](`
//! inside a fenced block or a backtick code span opens no link.

use std::ops::Range;

/// The span of every link destination in `source`, in source order.
pub(crate) fn link_destinations(source: &str) -> Vec<Range<usize>> {
    let mut spans = Vec::new();
    let mut offset = 0;
    let mut fence: Option<(u8, usize)> = None;
    for line in source.split_inclusive('\n') {
        let line_start = offset;
        offset += line.len();
        let trimmed = line.trim_start();
        if let Some(marker) = fence_marker(trimmed) {
            match fence {
                None => fence = Some(marker),
                Some((byte, length)) if marker.0 == byte && marker.1 >= length => fence = None,
                Some(_) => {}
            }
            continue;
        }
        if fence.is_some() {
            continue;
        }
        let code = code_spans(line);
        spans.extend(reference_definition(line).map(|r| line_start + r.start..line_start + r.end));
        for found in inline_destinations(line) {
            if !code
                .iter()
                .any(|c| c.start <= found.start && found.start < c.end)
            {
                spans.push(line_start + found.start..line_start + found.end);
            }
        }
    }
    spans
}

/// The fence character and run length when `trimmed` opens or closes a fenced block.
fn fence_marker(trimmed: &str) -> Option<(u8, usize)> {
    let first = *trimmed.as_bytes().first()?;
    if first != b'`' && first != b'~' {
        return None;
    }
    let run = trimmed.bytes().take_while(|b| *b == first).count();
    (run >= 3).then_some((first, run))
}

/// The spans of backtick code spans on one line.
fn code_spans(line: &str) -> Vec<Range<usize>> {
    let bytes = line.as_bytes();
    let mut spans = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'`' {
            index += 1;
            continue;
        }
        let run = bytes[index..].iter().take_while(|b| **b == b'`').count();
        let body = index + run;
        let closing = (body..bytes.len()).find(|&at| {
            bytes[at..].iter().take_while(|b| **b == b'`').count() == run
                && (at == 0 || bytes[at - 1] != b'`')
        });
        match closing {
            Some(close) => {
                spans.push(index..close + run);
                index = close + run;
            }
            None => index = body,
        }
    }
    spans
}

/// The destination of a reference definition line, `[label]: destination "title"`.
fn reference_definition(line: &str) -> Option<Range<usize>> {
    let indent = line.len() - line.trim_start_matches(' ').len();
    if indent > 3 || !line[indent..].starts_with('[') {
        return None;
    }
    let close = line[indent..].find("]:")? + indent;
    let after = close + 2;
    let start = after + (line[after..].len() - line[after..].trim_start().len());
    destination_at(line, start)
}

/// The destinations of every `](` on one line.
fn inline_destinations(line: &str) -> Vec<Range<usize>> {
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(at) = line[from..].find("](") {
        let open = from + at + 2;
        let start = open + (line[open..].len() - line[open..].trim_start().len());
        if let Some(span) = destination_at(line, start) {
            found.push(span);
        }
        from = open;
    }
    found
}

/// The destination starting at `start`: `<…>` up to the closing bracket, otherwise up to
/// whitespace or an unbalanced `)`.
fn destination_at(line: &str, start: usize) -> Option<Range<usize>> {
    let bytes = line.as_bytes();
    if bytes.get(start) == Some(&b'<') {
        let close = line[start + 1..].find('>')? + start + 1;
        return Some(start + 1..close);
    }
    let mut depth = 0usize;
    let mut end = start;
    while end < bytes.len() {
        match bytes[end] {
            b'(' => depth += 1,
            b')' if depth == 0 => break,
            b')' => depth -= 1,
            byte if byte.is_ascii_whitespace() => break,
            _ => {}
        }
        end += 1;
    }
    (end > start).then_some(start..end)
}
