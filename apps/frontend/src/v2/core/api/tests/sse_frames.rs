//! The event-stream parser against the HTML standard's interpretation rules, including every way
//! a network read can split the bytes.

use super::*;

/// Parse `stream` in one chunk.
fn parse_whole(stream: &[u8]) -> Vec<SseItem> {
    SseParser::new().feed(stream)
}

/// Parse `stream` in the chunks `split_at` cuts it into.
fn parse_split(stream: &[u8], split_at: &[usize]) -> Vec<SseItem> {
    let mut parser = SseParser::new();
    let mut out = Vec::new();
    let mut start = 0;
    for &cut in split_at {
        out.extend(parser.feed(&stream[start..cut]));
        start = cut;
    }
    out.extend(parser.feed(&stream[start..]));
    out
}

/// A message item, for terse expectations.
fn message(event: &str, data: &str, last_event_id: &str) -> SseItem {
    SseItem::Message(SseMessage {
        event: event.into(),
        data: data.into(),
        last_event_id: last_event_id.into(),
    })
}

#[test]
fn sse_dispatches_one_message_per_blank_line() {
    let items = parse_whole(b"data: one\n\ndata: two\n\n");
    assert_eq!(
        items,
        vec![message("message", "one", ""), message("message", "two", "")]
    );
}

#[test]
fn sse_accepts_crlf_line_breaks() {
    let items = parse_whole(b"event: ready\r\nid: 7\r\ndata: {}\r\n\r\n");
    assert_eq!(items, vec![message("ready", "{}", "7")]);
}

#[test]
fn sse_accepts_lone_cr_line_breaks() {
    let items = parse_whole(b"event: reset\rid: 9\rdata: x\r\r");
    assert_eq!(items, vec![message("reset", "x", "9")]);
}

#[test]
fn sse_accepts_mixed_line_breaks_in_one_stream() {
    let items = parse_whole(b"data: a\r\ndata: b\rdata: c\n\r\n");
    assert_eq!(items, vec![message("message", "a\nb\nc", "")]);
}

#[test]
fn sse_joins_a_crlf_split_between_two_chunks_into_one_break() {
    let stream = b"data: a\r\ndata: b\r\n\r\n";
    // Cut right after each CR: the LF opening the next chunk must not read as a blank line.
    let cuts: Vec<usize> = stream
        .iter()
        .enumerate()
        .filter(|(_, b)| **b == b'\r')
        .map(|(i, _)| i + 1)
        .collect();
    for cut in &cuts {
        assert_eq!(
            parse_split(stream, &[*cut]),
            vec![message("message", "a\nb", "")],
            "a CRLF split at byte {cut} changed the parse"
        );
    }
    assert_eq!(
        parse_split(stream, &cuts),
        vec![message("message", "a\nb", "")]
    );
}

#[test]
fn sse_two_crs_are_two_line_breaks() {
    let items = parse_whole(b"data: a\r\rdata: b\r\r");
    assert_eq!(
        items,
        vec![message("message", "a", ""), message("message", "b", "")]
    );
}

#[test]
fn sse_joins_multi_line_data_with_line_feeds() {
    let items = parse_whole(b"data: first\ndata: second\ndata:\ndata: fourth\n\n");
    assert_eq!(
        items,
        vec![message("message", "first\nsecond\n\nfourth", "")]
    );
}

#[test]
fn sse_strips_exactly_one_leading_space_from_a_value() {
    let items = parse_whole(b"data:tight\n\ndata:  padded\n\ndata: a:b: c\n\n");
    assert_eq!(
        items,
        vec![
            message("message", "tight", ""),
            message("message", " padded", ""),
            message("message", "a:b: c", ""),
        ]
    );
}

#[test]
fn sse_a_field_line_without_a_colon_has_an_empty_value() {
    // `data` alone appends an empty line, so the frame dispatches with empty data.
    let items = parse_whole(b"data\n\n");
    assert_eq!(items, vec![message("message", "", "")]);
}

#[test]
fn sse_names_the_event_type_and_defaults_to_message() {
    let items = parse_whole(b"event: ready\ndata: 1\n\ndata: 2\n\n");
    assert_eq!(
        items,
        vec![message("ready", "1", ""), message("message", "2", "")]
    );
}

#[test]
fn sse_an_event_type_without_data_does_not_leak_into_the_next_frame() {
    let items = parse_whole(b"event: stale\n\ndata: fresh\n\n");
    assert_eq!(items, vec![message("message", "fresh", "")]);
}

#[test]
fn sse_the_last_event_id_persists_until_an_id_field_replaces_it() {
    let items = parse_whole(b"id: 5\ndata: a\n\ndata: b\n\nid: 6\ndata: c\n\nid\ndata: d\n\n");
    assert_eq!(
        items,
        vec![
            message("message", "a", "5"),
            message("message", "b", "5"),
            message("message", "c", "6"),
            message("message", "d", ""),
        ]
    );
}

#[test]
fn sse_an_id_holding_nul_is_ignored() {
    let items = parse_whole(b"id: 3\ndata: a\n\nid: 4\0x\ndata: b\n\n");
    assert_eq!(
        items,
        vec![message("message", "a", "3"), message("message", "b", "3")]
    );
}

#[test]
fn sse_an_id_only_frame_dispatches_nothing_but_sets_the_id_of_later_frames() {
    let mut parser = SseParser::new();
    assert!(parser.feed(b"id: 41\n\n").is_empty());
    assert_eq!(
        parser.feed(b"data: x\n\n"),
        vec![message("message", "x", "41")]
    );
}

#[test]
fn sse_an_id_takes_effect_for_the_frame_it_sits_in() {
    let mut parser = SseParser::new();
    assert_eq!(
        parser.feed(b"id: 1\ndata: a\n\ndata: b\nid: 2\n"),
        vec![message("message", "a", "1")]
    );
    // The id line after the data line still belongs to the unfinished frame.
    assert_eq!(parser.feed(b"\n"), vec![message("message", "b", "2")]);
}

#[test]
fn sse_reads_retry_only_when_it_is_all_digits() {
    let items = parse_whole(
        b"retry: 2500\nretry: 1.5\nretry: -1\nretry:\nretry: 99999999999999999999999\n",
    );
    assert_eq!(items, vec![SseItem::Retry(2500), SseItem::Retry(u64::MAX)]);
}

#[test]
fn sse_surfaces_comments_and_keep_alives() {
    let items = parse_whole(b":\n\n: ping\ndata: x\n\n");
    assert_eq!(
        items,
        vec![
            SseItem::Comment,
            SseItem::Comment,
            message("message", "x", ""),
        ]
    );
}

#[test]
fn sse_ignores_unknown_fields_and_matches_field_names_by_case() {
    let items = parse_whole(b"Data: shout\nEvent: loud\nfoo: bar\ndata: quiet\n\n");
    assert_eq!(items, vec![message("message", "quiet", "")]);
}

#[test]
fn sse_blank_lines_without_data_dispatch_nothing() {
    assert!(parse_whole(b"\n\n\r\n\r\r").is_empty());
}

#[test]
fn sse_never_dispatches_a_frame_the_stream_ends_inside() {
    assert!(parse_whole(b"data: half").is_empty());
    assert!(parse_whole(b"data: whole line\n").is_empty());
}

#[test]
fn sse_decodes_a_character_split_across_chunks() {
    let stream = "data: é € 😀\n\n".as_bytes();
    let expected = vec![message("message", "é € 😀", "")];
    for cut in 0..=stream.len() {
        assert_eq!(
            parse_split(stream, &[cut]),
            expected,
            "a split at byte {cut} corrupted a multi-byte character"
        );
    }
    let every_byte: Vec<usize> = (1..stream.len()).collect();
    assert_eq!(parse_split(stream, &every_byte), expected);
}

#[test]
fn sse_replaces_malformed_utf8_with_the_replacement_character() {
    let items = parse_whole(b"data: a\xFFb\xC3\n\n");
    assert_eq!(items, vec![message("message", "a\u{FFFD}b\u{FFFD}", "")]);
}

#[test]
fn sse_an_incomplete_sequence_completed_by_the_next_chunk_is_not_malformed() {
    let mut parser = SseParser::new();
    assert!(parser.feed(b"data: \xE2\x82").is_empty());
    assert_eq!(parser.feed(b"\xAC\n\n"), vec![message("message", "€", "")]);
}

#[test]
fn sse_drops_one_leading_byte_order_mark_even_when_split() {
    let stream = b"\xEF\xBB\xBFdata: x\n\n";
    for cut in 0..=3 {
        assert_eq!(
            parse_split(stream, &[cut]),
            vec![message("message", "x", "")]
        );
    }
    // Only the first character of the stream is a byte order mark; a later one is text, so the
    // second mark makes the field name `\u{FEFF}data`, which is unknown and ignored.
    let items = parse_whole("\u{FEFF}\u{FEFF}data: y\n\ndata: \u{FEFF}z\n\n".as_bytes());
    assert_eq!(items, vec![message("message", "\u{FEFF}z", "")]);
}

#[test]
fn sse_every_split_of_a_mixed_stream_parses_like_the_whole() {
    let stream = "\u{FEFF}: open\r\nevent: ready\r\nid: 10\r\ndata: {\"resume_after\":10}\r\n\r\n\
                  id: 11\rdata: ünïcode ✓\rdata: line two\r\r\
                  :\n\nretry: 1000\nevent: reset\nid: 12\ndata: {}\n\n"
        .as_bytes();
    let whole = parse_whole(stream);
    assert_eq!(
        whole,
        vec![
            SseItem::Comment,
            message("ready", "{\"resume_after\":10}", "10"),
            message("message", "ünïcode ✓\nline two", "11"),
            SseItem::Comment,
            SseItem::Retry(1000),
            message("reset", "{}", "12"),
        ]
    );
    for cut in 0..=stream.len() {
        assert_eq!(parse_split(stream, &[cut]), whole, "split at {cut}");
    }
    for a in (0..stream.len()).step_by(3) {
        for b in (a..stream.len()).step_by(5) {
            assert_eq!(parse_split(stream, &[a, b]), whole, "splits at {a} and {b}");
        }
    }
    let every_byte: Vec<usize> = (1..stream.len()).collect();
    assert_eq!(parse_split(stream, &every_byte), whole);
}

#[test]
fn sse_empty_chunks_change_nothing() {
    let mut parser = SseParser::new();
    assert!(parser.feed(b"").is_empty());
    assert!(parser.feed(b"data: a\n").is_empty());
    assert!(parser.feed(b"").is_empty());
    assert_eq!(parser.feed(b"\n"), vec![message("message", "a", "")]);
}
