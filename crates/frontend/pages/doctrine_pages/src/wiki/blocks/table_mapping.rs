//! A manual's tables as render nodes, each column keeping the alignment its author set.
//!
//! **Role:** maps a table block — its column alignments, header cells and body rows — to a
//! scrollable wrapper around a `<table>`.
//! **Position:** called by the block mapper for every table block; cells are inlines, mapped by
//! the inline mapper.
//! **Signals & state:** none; pure functions.
//! **Invariants:** a cell takes the alignment of its column index; a column past the end of the
//! alignment list, like a column whose alignment is `none`, starts at the reading edge. Each cell
//! carries exactly one alignment class.

use super::inline_mapping::inline_nodes;
use super::render_tree::{ElementTag, RenderElement, RenderNode};
use frontend_api_dtos::wiki::{WikiInline, WikiTableAlignment};

/// The scroll box around a table.
const TABLE_WRAPPER_CLASS: &str = "my-6 overflow-x-auto rounded-xl border border-white/10";
/// The table.
const TABLE_CLASS: &str = "w-full border-collapse text-sm";
/// The header row group.
const TABLE_HEAD_CLASS: &str = "bg-white/5";
/// A body row.
const BODY_ROW_CLASS: &str = "border-t border-white/5";
/// A header cell, before its alignment class.
const HEADER_CELL_CLASS: &str = "border-b border-white/10 px-3 py-2 font-mono text-xs font-bold tracking-widest text-on-surface uppercase";
/// A body cell, before its alignment class.
const DATA_CELL_CLASS: &str = "px-3 py-2 align-top text-on-surface-variant";

/// The class that aligns a cell of a column with `alignment`.
pub(super) fn alignment_class(alignment: WikiTableAlignment) -> &'static str {
    match alignment {
        WikiTableAlignment::None => "text-start",
        WikiTableAlignment::Left => "text-left",
        WikiTableAlignment::Center => "text-center",
        WikiTableAlignment::Right => "text-right",
    }
}

/// The table of `alignments`, `header` cells and body `rows`.
pub(super) fn table_node(
    alignments: &[WikiTableAlignment],
    header: &[Vec<WikiInline>],
    rows: &[Vec<Vec<WikiInline>>],
) -> RenderNode {
    let head_row = table_row(
        ElementTag::HeaderCell,
        HEADER_CELL_CLASS,
        "",
        alignments,
        header,
    );
    let head = RenderElement::new(ElementTag::TableHead, TABLE_HEAD_CLASS)
        .with_children(vec![head_row])
        .into_node();
    let body = RenderElement::new(ElementTag::TableBody, "")
        .with_children(
            rows.iter()
                .map(|row| {
                    table_row(
                        ElementTag::DataCell,
                        DATA_CELL_CLASS,
                        BODY_ROW_CLASS,
                        alignments,
                        row,
                    )
                })
                .collect(),
        )
        .into_node();
    let table = RenderElement::new(ElementTag::Table, TABLE_CLASS)
        .with_children(vec![head, body])
        .into_node();
    RenderElement::new(ElementTag::Division, TABLE_WRAPPER_CLASS)
        .with_children(vec![table])
        .into_node()
}

/// One row of `cells`, each a `cell_tag` element whose class is `cell_class` plus its column's
/// alignment class, a header cell also naming its column as its scope; the row itself carries
/// `row_class`.
fn table_row(
    cell_tag: ElementTag,
    cell_class: &str,
    row_class: &str,
    alignments: &[WikiTableAlignment],
    cells: &[Vec<WikiInline>],
) -> RenderNode {
    let cells = cells
        .iter()
        .enumerate()
        .map(|(column, inlines)| {
            let alignment = alignments
                .get(column)
                .copied()
                .unwrap_or(WikiTableAlignment::None);
            let class = format!("{cell_class} {}", alignment_class(alignment));
            let mut cell = RenderElement::new(cell_tag, &class);
            if cell_tag == ElementTag::HeaderCell {
                cell = cell.with_attribute("scope", "col");
            }
            cell.with_children(inline_nodes(inlines)).into_node()
        })
        .collect();
    RenderElement::new(ElementTag::TableRow, row_class)
        .with_children(cells)
        .into_node()
}
