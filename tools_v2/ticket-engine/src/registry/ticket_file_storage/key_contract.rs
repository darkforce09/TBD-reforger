//! Key contract.

use super::*;

#[allow(dead_code)]
pub const FROZEN_27: &[&str] = &[
    "id",
    "title",
    "summary",
    "program",
    "surfaces",
    "impact",
    "status",
    "executor",
    "targets",
    "stream",
    "order",
    "priority",
    "notes",
    "route",
    "spec",
    "depends_on",
    "shipped_at",
    "branch",
    "parallel_ok",
    "slices",
    "unblocks",
    "slice_plan",
    "implements",
    "active_slice",
    "user_story",
    "milestone",
    "acceptance",
];

/// T-911.2 mapped encoding-C key set — every top-level key `TicketFile` carried at the
/// typed cutover, in canonical spelling (`slices` / `active_slice` are parse-time serde
/// aliases and never appear on disk). FROZEN: a new ticket key goes in [`ALLOWED_NEW`],
/// never here.
#[allow(dead_code)]
// governance consts; consumed by the key-governance tests below
pub const ENCODING_C_KEYS: &[&str] = &[
    "id",
    "kind",
    "title",
    "summary",
    "status",
    "order",
    "spec",
    "executor",
    "notes",
    "priority",
    "depends_on",
    "unblocks",
    "parent",
    "children",
    "active",
    "user_story",
    "acceptance",
    "shipped_at",
    "owns",
    "pack_last",
    "scope",
];

/// T-913.1 deliberate widen: the ONLY keys legal on disk beyond [`ENCODING_C_KEYS`].
/// Inventing a ticket key means adding it here AND to `TicketFile` AND to
/// `.ai/tickets/schema.json` in one commit that says so —
/// `on_disk_keys_are_mapped_or_allowed_new` stays red until you do.
///
/// T-917.2 widen (schema v2, one governance commit with `TicketFile` + schema.json):
/// `class`, `plan`, the five body lists, `citations`, the provenance pair
/// `estimated`/`estimate_note`, and the wall-quarantine target `migration_legacy`.
/// The flat `[scope]` shape rides the frozen `scope` key.
///
/// T-920.1 widen: `main_goal` — the rename of `user_story` (t920 spec Decisions log
/// #1). `user_story` STAYS in the frozen [`ENCODING_C_KEYS`] as history: on-disk
/// keys must be a SUBSET of the union, and a vanished key is legal — the 50-carrier
/// migration emptied the spelling from the live tree, while old git revisions still
/// parse through the `TicketFile` serde alias.
#[allow(dead_code)]
// governance consts; consumed by the key-governance tests below
pub const ALLOWED_NEW: &[&str] = &[
    "created_at",
    "completed_at",
    "class",
    "plan",
    "main_goal",
    "context",
    "requirement",
    "current_state",
    "approach",
    "verify",
    "citations",
    "estimated",
    "estimate_note",
    "migration_legacy",
];

#[allow(dead_code)]
pub fn union_ticket_keys(registry: &Value) -> std::collections::BTreeSet<String> {
    let mut keys = std::collections::BTreeSet::new();
    if let Some(arr) = registry.get("tickets").and_then(Value::as_array) {
        for t in arr {
            if let Some(obj) = t.as_object() {
                for k in obj.keys() {
                    keys.insert(k.clone());
                }
            }
        }
    }
    keys
}
