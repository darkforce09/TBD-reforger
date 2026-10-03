//! The ticket file key contract: which top-level keys a ticket file may carry.
//!
//! **Role:** the three governed key sets and the key union of a registry value.
//! **Position:** re-exported by the storage module; only the key-governance tests read them.
//! **Signals & state:** none; constants and a pure function.
//! **Invariants:** [`ENCODING_C_KEYS`] and [`FROZEN_27`] never change; a new ticket key goes into
//! [`ALLOWED_NEW`], `TicketFile` and `.ai/tickets/schema.json` together.

use super::*;

/// The 27 top-level keys of an untyped registry row. Fixed.
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

/// The mapped key set: the base top-level keys `TicketFile` maps, in canonical spelling
/// (`slices` and `active_slice` are parse-time serde aliases and never appear on disk). Fixed: a
/// new ticket key goes in [`ALLOWED_NEW`], never here.
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

/// The only keys legal on disk beyond [`ENCODING_C_KEYS`]: the timestamps, `class`, `plan`,
/// `main_goal`, the five body lists, `citations`, the estimate pair `estimated` and
/// `estimate_note`, and `migration_legacy`. The `[scope]` table uses the mapped `scope` key.
///
/// Adding a ticket key means adding it here, to `TicketFile` and to `.ai/tickets/schema.json`
/// in one commit; `on_disk_keys_are_mapped_or_allowed_new` stays red until all three agree.
/// On-disk keys must be a subset of the union, so a mapped key no file carries (`user_story`,
/// which `main_goal` replaces and which past revisions still parse through a serde alias) is
/// legal.
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

/// Every top-level key any row of the registry value carries.
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
