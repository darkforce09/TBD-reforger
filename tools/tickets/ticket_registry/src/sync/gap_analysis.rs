//! The ticket column of the editor gap-analysis tables.
//!
//! **Role:** parses the gap-analysis document's tables (header `| eden_id | … priority |`),
//! proves they round-trip byte for byte, and rewrites each row's ticket column from the
//! registry and the corpus pins.
//! **Position:** under `ticket sync`; [`sync_gap_analysis_ticket_column`] runs from
//! `crate::sync::cmd_sync` and [`test_gap_analysis_round_trip`] from `ticket check`.
//! **Signals & state:** none; reads and writes
//! [`repository_layout::documentation::GAP_ANALYSIS`].
//! **Invariants:** the column is rewritten only after the unchanged tables round-trip byte for
//! byte; text outside the tables is never touched; an absent document is skipped.

use crate::error::{Error, Result};
use regex::Regex;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

use crate::corpus_pins::{self, CorpusPins, EditorGapRowId};
use crate::registry::{str_field, tickets};
use repository_layout::documentation::GAP_ANALYSIS;

/// A checkmark followed by a ticket id captures the id's parent: `T-` and three or more digits,
/// without any dotted child suffix or trailing text.
static CHECKMARK_TICKET: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"✅\s*(T-[0-9]{3,})").expect("the checkmark pattern compiles"));
/// A Markdown table separator row: a pipe, dashes, a pipe.
static SEP_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\|\s*-+\s*\|").expect("the separator pattern compiles"));

struct GapTable {
    start_line: usize,
    end_line: usize,
    header_line: String,
    separator_line: String,
    rows: Vec<Vec<String>>,
    raw_rows: Vec<String>,
}

struct GapDoc {
    lines: Vec<String>,
    tables: Vec<GapTable>,
}

fn split_table_row(line: &str) -> Vec<String> {
    let inner = line.trim();
    if !inner.starts_with('|') {
        return vec![];
    }
    let inner = inner.trim_matches('|');
    inner.split('|').map(|c| c.trim().to_string()).collect()
}

fn parse_gap_analysis(content: &str) -> GapDoc {
    let lines: Vec<String> = content
        .split_inclusive('\n')
        .map(|s| s.to_string())
        .collect();
    let mut doc = GapDoc {
        lines: lines.clone(),
        tables: vec![],
    };
    let mut i = 0;
    while i < lines.len() {
        let line = &lines[i];
        if line.contains("| eden_id |") && line.contains("priority |") {
            let header_line = line.clone();
            let sep_line = lines.get(i + 1).cloned().unwrap_or_default();
            if !SEP_RE.is_match(sep_line.trim()) {
                i += 1;
                continue;
            }
            let mut rows = vec![];
            let mut raw_rows = vec![];
            let mut j = i + 2;
            while j < lines.len() {
                let row_line = &lines[j];
                if !row_line.trim().starts_with('|') {
                    break;
                }
                if SEP_RE.is_match(row_line.trim()) {
                    break;
                }
                let cells = split_table_row(row_line);
                if cells.len() >= 5 {
                    rows.push(cells);
                    raw_rows.push(row_line.clone());
                }
                j += 1;
            }
            doc.tables.push(GapTable {
                start_line: i,
                end_line: j - 1,
                header_line,
                separator_line: sep_line,
                rows,
                raw_rows,
            });
            i = j;
            continue;
        }
        i += 1;
    }
    doc
}

fn write_gap_tables(doc: &GapDoc, ticket_column: bool) -> String {
    let mut lines = doc.lines.clone();
    let mut offset: isize = 0;
    for table in &doc.tables {
        let start = (table.start_line as isize + offset) as usize;
        let end = (table.end_line as isize + offset) as usize;
        let mut new_block: Vec<String> = vec![];
        let mut header = table.header_line.clone();
        if ticket_column {
            header = header.replace("priority |", "ticket |");
        }
        new_block.push(header);
        new_block.push(table.separator_line.clone());
        for (row_cells, raw) in table.rows.iter().zip(table.raw_rows.iter()) {
            if ticket_column && row_cells.len() >= 5 {
                let mut new_line = format!("| {} |", row_cells.join(" | "));
                if raw.ends_with('\n') {
                    new_line.push('\n');
                }
                new_block.push(new_line);
            } else {
                new_block.push(raw.clone());
            }
        }
        let old_len = (end - start + 1) as isize;
        let new_len = new_block.len() as isize;
        lines.splice(start..=end, new_block);
        offset += new_len - old_len;
    }
    lines.join("")
}

/// Prove the gap-analysis document's tables re-render to the exact original text; `Ok` when the
/// document is absent.
///
/// # Errors
/// When the document cannot be read, or the re-rendered text differs (the message gives both
/// byte lengths).
pub fn test_gap_analysis_round_trip(root: &Path) -> Result<()> {
    let path = root.join(GAP_ANALYSIS);
    if !path.is_file() {
        return Ok(());
    }
    let original = fs::read_to_string(&path)?;
    let doc = parse_gap_analysis(&original);
    let round_trip = write_gap_tables(&doc, false);
    if round_trip != original {
        return Err(Error::msg(format!(
            "gap_analysis round-trip failed: {} vs {} bytes",
            original.len(),
            round_trip.len()
        )));
    }
    Ok(())
}

fn lookup_ticket_for_gap(
    pins: &CorpusPins,
    registry: &Value,
    eden_id: &str,
    tbd_id: &str,
    gap_notes: &str,
) -> String {
    if let Some(c) = CHECKMARK_TICKET.captures(gap_notes) {
        return c[1].to_string();
    }
    for row in tickets(registry) {
        if let Some(impls) = row.get("implements").and_then(|v| v.as_array()) {
            for item in impls {
                if let Some(s) = item.as_str()
                    && (s == eden_id || s == tbd_id)
                {
                    return str_field(row, "id");
                }
            }
        }
    }
    if let Some(ticket) = pins.gap_implementation(&EditorGapRowId::from(eden_id)) {
        return ticket.to_string();
    }
    if let Some(ticket) = pins.gap_implementation(&EditorGapRowId::from(tbd_id)) {
        return ticket.to_string();
    }
    "—".to_string()
}

/// Rewrite the ticket column of every gap-analysis row: the ticket a checkmark in the notes
/// names, else the registry row whose `implements` lists the row's Eden or TBD id, else the
/// pinned ticket for either id, else `—`.
///
/// # Errors
/// When the tables do not round-trip, the corpus pins cannot be loaded, or the document cannot
/// be read or written.
pub fn sync_gap_analysis_ticket_column(root: &Path, registry: &Value) -> Result<()> {
    test_gap_analysis_round_trip(root)?;
    let path = root.join(GAP_ANALYSIS);
    if !path.is_file() {
        return Ok(());
    }
    let pins = corpus_pins::load(root)?;
    let original = fs::read_to_string(&path)?;
    let mut doc = parse_gap_analysis(&original);
    for table in &mut doc.tables {
        for row in &mut table.rows {
            if row.len() >= 5 {
                row[3] = lookup_ticket_for_gap(&pins, registry, &row[0], &row[1], &row[4]);
            }
        }
    }
    let updated = write_gap_tables(&doc, true);
    fs::write(&path, updated)?;
    Ok(())
}
