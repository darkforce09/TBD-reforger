use anyhow::{Result, bail};
use regex::Regex;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

use crate::registry::{str_field, tickets};
use crate::root::gap_analysis_path;

static CHECKMARK_TICKET: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"✅\s*(T-\d{3})").unwrap());
static SEP_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\|\s*-+\s*\|").unwrap());

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

pub fn test_gap_analysis_round_trip(root: &Path) -> Result<()> {
    let path = gap_analysis_path(root);
    if !path.is_file() {
        return Ok(());
    }
    let original = fs::read_to_string(&path)?;
    let doc = parse_gap_analysis(&original);
    let round_trip = write_gap_tables(&doc, false);
    if round_trip != original {
        bail!(
            "gap_analysis round-trip failed: {} vs {} bytes",
            original.len(),
            round_trip.len()
        );
    }
    Ok(())
}

fn lookup_ticket_for_gap(registry: &Value, eden_id: &str, tbd_id: &str, gap_notes: &str) -> String {
    if let Some(c) = CHECKMARK_TICKET.captures(gap_notes) {
        return c[1].to_string();
    }
    for row in tickets(registry) {
        if let Some(impls) = row.get("implements").and_then(|v| v.as_array()) {
            for item in impls {
                if let Some(s) = item.as_str() {
                    if s == eden_id || s == tbd_id {
                        return str_field(row, "id");
                    }
                }
            }
        }
    }
    let mapping: &[(&str, &str)] = &[
        ("PLACE-004", "T-072"),
        ("XFORM-SHIFT-001", "T-073"),
        ("XFORM-ROT-001", "T-073"),
        ("RIGHT-SUBMODE-001", "T-074"),
        ("WIDGET-CYCLE-001", "T-075"),
        ("CREW-PANEL-001", "T-076"),
        ("CREW-BOARD-001", "T-076"),
        ("PLACE-CREW-001", "T-077"),
        ("COMP-SAVE-001", "T-078"),
        ("COMP-PLACE-001", "T-078"),
        ("RIGHT-MODE-001", "T-068"),
        ("RIGHT-CAT-001", "T-068"),
        ("RIGHT-MODE-006", "T-069"),
        ("RIGHT-STUB-002", "T-069"),
        ("CONN-GROUP-001", "T-071"),
        ("LEFT-ORBAT-001", "T-071"),
        ("RIGHT-SEARCH-001", "T-055"),
        ("SEL-MOD-001", "T-053"),
        ("SEL-ORBAT-DBL-001", "T-054"),
        ("ATTR-FIELD-OBJ-POSITION", "T-049"),
        ("ATTR-TAB-001", "T-049"),
        ("MAP-TERRAIN-001", "T-049"),
        ("DATA-HYD-TITLE-001", "T-049"),
        ("TOP-TITLE-001", "T-049"),
        ("TOP-UNDO-001", "T-052"),
        ("TOOLBAR-UNDO-001", "T-052"),
    ];
    for (k, v) in mapping {
        if *k == eden_id {
            return (*v).to_string();
        }
    }
    for (k, v) in mapping {
        if *k == tbd_id {
            return (*v).to_string();
        }
    }
    "—".to_string()
}

pub fn sync_gap_analysis_ticket_column(root: &Path, registry: &Value) -> Result<()> {
    test_gap_analysis_round_trip(root)?;
    let path = gap_analysis_path(root);
    if !path.is_file() {
        return Ok(());
    }
    let original = fs::read_to_string(&path)?;
    let mut doc = parse_gap_analysis(&original);
    for table in &mut doc.tables {
        for row in &mut table.rows {
            if row.len() >= 5 {
                row[3] = lookup_ticket_for_gap(registry, &row[0], &row[1], &row[4]);
            }
        }
    }
    let updated = write_gap_tables(&doc, true);
    fs::write(&path, updated)?;
    Ok(())
}
