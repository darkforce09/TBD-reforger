# Untyped ticket file storage

The key-order-preserving TOML encoding of an untyped [ticket](/documentation_v2/glossary.md#ticket)
file, one `T-<id>.toml` per parent, read into and written from the registry's JSON value; with it,
the path helpers every registry reader shares and the key sets that govern what a ticket file may
carry.

## Contents

```text
tools_v2/ticket-engine/src/registry/ticket_file_storage/
├── encoding.rs      one ticket as JSON value to TOML text and back; ticket paths and parent ids
├── key_contract.rs  `FROZEN_27`, `ENCODING_C_KEYS` and `ALLOWED_NEW`: the governed ticket key sets
├── mod.rs           the module tree; re-exports the encoding, storage and key-set items
├── storage.rs       `load_toml_tree` and `save_toml_tree` over a whole folder, and the id listings
└── tests/           unit tests for the key sets, the byte-identical round trip and the id listings
```

## How it works

`ticket_to_toml_string` writes a ticket's JSON object as a TOML table that records its own key
order in a `__keys` array and its position in the registry in `__ord`; a JSON `null` becomes the
string `__tbd_null__`. `ticket_from_toml_str` reverses it and returns the position with the value.
`load_toml_tree` reads every parent file (`T-` and digits only) into `{ "next_id", "tickets" }`,
ordered by `__ord`, and `save_toml_tree` writes one back. The live tree is typed, so the registry
loads it through `typed_projection` instead; this encoding serves untyped trees and the files of
past revisions that `ticket_status_history` reads.

`tickets_dir`, `root_marker_path`, `parent_toml_path`, `is_parent_id`, `parent_numeric_id` and
`derive_next_id` (the highest parent number plus one) are the shared helpers; the typed projection
and the title lookup use them too.

The key sets are the ticket file's key contract, which the tests hold against every file in
`.ai/tickets/`: `FROZEN_27` is the key set of an untyped registry, `ENCODING_C_KEYS` the keys
`TicketFile` maps, and `ALLOWED_NEW` the further keys a ticket file may carry. A new ticket key goes
into `ALLOWED_NEW`, `TicketFile` and `.ai/tickets/schema.json` in one commit.

## Boundaries

- Depends on: `crate::repository` (`TICKETS_DIR`, `ROOT_MARKER`); `crate::store`'s id order; the
  `toml` and `serde_json` crates.
- Used by: `crate::registry` (`load_registry` and `save_registry` for untyped trees,
  `typed_projection`, `ticket_titles`, `ticket_status_history`); and the tests of
  `tools_v2/xtask/src/commands/mod_ops/tests/wave_execution.rs`, which build fixture trees with
  `save_toml_tree`. `on_disk_ids`, `corpus_ids` and the key sets have no caller outside the
  tests.
- Rules:
  - a ticket written and read back is byte-identical to the registry document it came from
    (`toml_roundtrip_is_byte_identical_to_the_registry_document`);
  - every key on disk is a mapped key or an `ALLOWED_NEW` key (`on_disk_keys_are_mapped_or_allowed_new`);
  - no ticket file is lost across a load and a save (`no_ticket_lost_set_equality`).
