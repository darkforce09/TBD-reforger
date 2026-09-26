# Ticket detail panel

The right-hand column of the [ticketboard](/documentation_v2/glossary/n_to_z.md#ticketboard) that shows
one [ticket](/documentation_v2/glossary/n_to_z.md#ticket) in full: its header, the ticket actions strip,
an optional ownership comparison with a second ticket, a metadata table, the typed body sections,
and its links to other tickets and files.

## Contents

```text
apps/ticketboard/src/ticket_browser/ui/detail_panel/
├── body_sections.rs    section headers, the "—" marker, one body section, the quarantine
├── cells.rs            `Cell`, one metadata value: text, link, scope, stamp or token estimate
├── comparison.rs       the ownership comparison: both `owns` lists, colliding pairs, verdict
├── metadata.rs         `detail_ui`, the whole panel from header to reference lists
├── mod.rs              the module tree
└── reference_lists.rs  id links, the `depends_on`, `unblocks` and `children` lists, `owns`
```

## How it works

`detail_ui` paints, top to bottom:

1. The id, kind and class chip with a close button, the title, then the header lines from
   `models::detail_sections` (`main_goal` in its own tint, then `summary`), each with its
   definition as tooltip.
2. The action strip, a callback the application supplies from `crate::ticket_actions`; its events
   return wrapped in `BrowserEvent::TicketAction`.
3. When a second ticket is shift-clicked, `compare_ui`: "never the same wave" with every colliding
   `owns` pair, or "no collision", by `collides` and `colliding_pairs` of
   `crate::wave_plan::services::lock_file`; otherwise the hint "shift-click another ticket to
   compare owns".
4. A table of `status`, `executor` (marked "(default)" when unset), `priority`, `spec`, `plan`,
   `parent`, `active`, the `shipped_at`, `created_at` and `completed_at` stamps, a "tokens
   (estimated)" row only when the ticket marks its tokens as estimated, `pack_last`, `scope` and
   the ticket `file`.
5. The body sections in `body_region_order`, with a count on list fields and "—" for an absent
   field, then the `migration_legacy` quarantine only when it has lines, in an amber frame with
   "Copy for triage" and an expand control past eight lines.
6. The `depends_on`, `unblocks` and `children` id lists and the `owns` paths.

A path cell whose target is a Markdown file (a spec, a plan, a citation) emits `OpenDoc` for the
document viewer; any other path, such as the ticket's TOML file, emits `OpenPath` for the
operating system's handler. An id links only when it exists in the corpus. A stamp shows a
measured value bare, and an estimated one with the `~` glyph and the ticket's estimate note as
tooltip; the panel never shows a measured token count, which the Metrics tab owns.

## Boundaries

- Depends on: `crate::ticket_browser::models` (`BrowserView`, `detail_sections`) and the browser's
  `appearance` colours; `crate::ticket_registry::models::projection` (the ticket view, labels,
  breadcrumbs, `Class`); `crate::execution_metrics::estimated` (`stamp_cell`, `tokens_cell`, the
  estimate glyph and markers); `crate::document_viewer::services::document_loading::wants_viewer`;
  `crate::wave_plan::services::lock_file` (`collides`, `colliding_pairs`); `crate::core::ui`;
  `eframe::egui` and `egui_extras::TableBuilder`.
- Used by: `crate::application::feature_views`, which calls `metadata::detail_ui` with the
  borrowed view and the action-strip callback.
- Rules: the panel only emits `BrowserEvent`s and changes nothing itself; the comparison takes its
  verdict and its pairs from `crate::wave_plan::services::lock_file`, the viewer's one copy of the
  collision rule, and keeps none of its own;
  `apps/ticketboard/src/application/tests/rendering.rs` paints the details headlessly.
