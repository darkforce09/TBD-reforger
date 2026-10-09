# Ticket registry readers

The read side of the [ticket](/documentation/glossary/n_to_z.md#ticket) files: the registry as one
JSON value that the query, sync and validation commands consume, the field helpers they read it
with, and three narrower readers for the platform wave driver (shipping status, ticket statuses at
a past revision, and titles). Mutations never pass through here; they go through `crate::ops`.

## Contents

```text
tools/tickets/ticket_registry/src/registry/
├── mod.rs                    `load_registry`, `save_registry`, `write_json_ascii` and the row helpers
├── shipping_status.rs        `ShippingStatus`: which tickets are shipped or cancelled
├── ticket_file_storage/      the untyped ticket file encoding, path helpers and key sets
├── ticket_status_history.rs  `status_map_at_rev`: ticket statuses at a past git revision
├── ticket_titles.rs          `read_ticket_title`: one ticket's title, or empty
└── typed_projection.rs       the parents-only JSON projection of the typed ticket files
```

## How it works

`load_registry(root)` answers `{ "next_id", "tickets" }`. When the ticket files are typed (they
carry `kind =`), `typed_projection::load_phase2_tree` reads every parent file, orders the rows by
`order` (absent sorts as 99999) then id, and gives each program a synthesised `slice_plan` built
from its children's files, with each child's `targets` derived from its scope domain (`website`,
`mod`, `schema` as `shared`, `engine` and `repo` as `root`). An untyped folder loads through
`ticket_file_storage`; a folder with neither refuses rather than answering an empty registry.

`save_registry` refuses a typed tree, since the typed operations are the one writer, and exists so
the tests can prove that no command reaches a whole-tree writer. `write_json_ascii` writes
`queue.json`: two-space indentation, every character outside printable ASCII escaped. The helpers
(`tickets`, `ticket_by_id`, `str_field`, `opt_str`, `ticket_sort_key`, `slice_spec`,
`slice_executor`, `slice_targets`, `slice_handoff_path` and the rest) read one row, falling back
from the active slice's entry to the ticket's own field.

The platform readers answer from other sources: `ShippingStatus::load_repo` from the wave lock's
ticket views, `status_map_at_rev` from `git show` of the ticket files at a revision,
and `read_ticket_title` from one file. At revisions older than one file per ticket,
`status_map_at_rev` reads the single `registry.json` the ticket folder held then.

## Public surface

- `load_registry` and the `Registry` value, with the row helpers: `cargo xtask ticket` and
  `registry-get` (`tools/xtask/src/commands/ticket/`), `platform slice-run`
  (`tools/commands/platform_execution/src/slice_execution.rs`) and the mod wave driver
  (`tools/commands/mod_operations/src/wave_execution/execution.rs`).
- `shipping_status::ShippingStatus`: the platform wave ledger, which re-exports it as `Registry`
  (`tools/commands/platform_execution/src/wave_execution/ledger.rs`).
- `ticket_status_history::status_map_at_rev`: the wave-close number check
  (`tools/commands/platform_execution/src/wave_execution/base/wave_close_number.rs`).
- `ticket_titles::read_ticket_title`: the mod wave driver's display names.
- `ticket_file_storage::save_toml_tree`: fixture trees in
  `tools/commands/mod_operations/src/tests/wave_execution.rs`.

## Boundaries

- Depends on: `ticket_model` (the model, `parse_ticket_toml`, `store::ticket_id_order_key` and
  `repository::handoff_doc`); `ticket_wave_lock::load_views` for `ShippingStatus::load_repo`;
  `repository_layout` (`TICKETS_DIR`), `repository_root` (`ROOT_MARKER`); `process_runner` to run `git` for the
  past-revision reads.
- Used by: in this crate, `verbs`, `sync` and `validation`, which read the projection; outside
  it, the xtask callers listed under Public surface.
- Rules:
  - the projection holds parents only, and a child's fields reach it through its parent's
    `slice_plan`, with `targets` derived from the scope (`targets_from_scope_v2_outputs`);
  - `save_registry` refuses a typed tree, and no module of `verbs/` names it
    (`mutators_never_reach_the_value_writer_pin`);
  - one row without an id poisons `ShippingStatus`, which then answers "not shipped" for every
    ticket, and a cancelled ticket counts as shipped (`registry_poisons_on_a_ticket_without_an_id`,
    `cancelled_counts_as_shipped`);
  - `status_map_at_rev` reads both the single-file and the one-file-per-ticket forms
    (`dual_read_json_then_toml`), and a revision with neither answers `None`, never an empty map.
