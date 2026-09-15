//! Role: substitutions.
//! Position: `mission/compiler/flatten` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::is_wire_unsafe;

/// One character the author placed that `kit-aliases.json` has no row for, and the faction default the compile used instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KitSubstitution {
    /// The full Enfusion ResourceName the author placed — verbatim, because it is exactly the string a new `kit-aliases.json` row has to carry to make this stop happening.
    pub asset_id: String,

    /// The slugged faction key whose default was used.
    pub faction: String,

    /// The `kit:` alias that reached `slots[].kit` and `orbat.*.groups[].roles[].kit` instead.
    pub kit: String,

    /// Derived id (`faction:callsign:role:occurrence`) of the FIRST slot that hit this pair — the same string the compiled document carries in `slots[].id`, so a reader holding the document can find the seat this is talking about.
    pub example_slot_id: String,

    /// That slot's editor id, as carried to `slots[].uid`. Kept alongside `example_slot_id` because the derived id shifts under role renames/reorders/deletes and this one does not.
    pub example_slot_uid: String,

    /// How many slots in this compile resolved through this same pair.
    pub occurrences: usize,
}

/// Canonical max reported substitutions value.
pub(super) const MAX_REPORTED_SUBSTITUTIONS: usize = 20;

/// Domain representation of kit substitution report.
#[derive(Debug, Clone, Default)]
pub struct KitSubstitutionReport {
    /// Rows.
    pub(super) rows: Vec<KitSubstitution>,
    /// Slots.
    pub(super) slots: usize,
}

impl KitSubstitutionReport {
    /// True when every placed character resolved to its own kit.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.slots == 0
    }

    /// Every slot that got a faction default in place of its own kit — uncapped and NOT deduped, so this is the number of seats that will spawn as somebody else.
    #[must_use]
    pub fn slots(&self) -> usize {
        self.slots
    }

    /// The named substitutions, at most [`MAX_REPORTED_SUBSTITUTIONS`] of them.
    #[must_use]
    pub fn rows(&self) -> &[KitSubstitution] {
        &self.rows
    }

    /// One readable line per substitution, for a log line or an editor dialog — the shape `wire_safety::scan_editor_payload` returns, so a caller can render both the same way.
    #[must_use]
    pub fn details(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .rows
            .iter()
            .map(|r| {
                let more = if r.occurrences > 1 {
                    format!(" (and {} more slot(s) on this side)", r.occurrences - 1)
                } else {
                    String::new()
                };
                format!(
                    "{}: \"{}\" has no kit-aliases.json row — compiled as the {} default \"{}\", \
                     so this seat spawns a different character than the one placed{}",
                    r.example_slot_id,
                    escape_resource_name(&r.asset_id),
                    r.faction,
                    r.kit,
                    more,
                )
            })
            .collect();

        let unnamed = self.slots - self.rows.iter().map(|r| r.occurrences).sum::<usize>();
        if unnamed > 0 {
            out.push(format!(
                "+ {unnamed} further slot(s) were substituted under assets not named above"
            ));
        }
        out
    }
}

/// Accumulator for [`KitSubstitutionReport`], filled during the one existing slot walk.
#[derive(Default)]
pub(super) struct SubstitutionAcc {
    /// Rows.
    pub(super) rows: Vec<KitSubstitution>,
    /// Slots.
    pub(super) slots: usize,
}

impl SubstitutionAcc {
    /// `slot_id` is a closure so the derived id is only formatted for the first slot of a new pair — the 300,000th slot carrying an unaliased asset costs one bounded scan and nothing else.
    pub(super) fn record(
        &mut self,
        asset_id: &str,
        faction: &str,
        kit: &str,
        slot_uid: &str,
        slot_id: impl FnOnce() -> String,
    ) {
        self.slots += 1;
        if let Some(row) = self
            .rows
            .iter_mut()
            .find(|r| r.asset_id == asset_id && r.faction == faction)
        {
            row.occurrences += 1;
            return;
        }
        if self.rows.len() >= MAX_REPORTED_SUBSTITUTIONS {
            return;
        }
        self.rows.push(KitSubstitution {
            asset_id: asset_id.to_string(),
            faction: faction.to_string(),
            kit: kit.to_string(),
            example_slot_id: slot_id(),
            example_slot_uid: slot_uid.to_string(),
            occurrences: 1,
        });
    }

    /// Finish using the supplied domain data.
    pub(super) fn finish(self) -> KitSubstitutionReport {
        KitSubstitutionReport {
            rows: self.rows,
            slots: self.slots,
        }
    }
}

/// Render a ResourceName into a line a human reads.
pub(super) fn escape_resource_name(s: &str) -> String {
    if !s.bytes().any(is_wire_unsafe) {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if (c as u32) < 0x80 && is_wire_unsafe(c as u8) {
            out.push_str(&format!("\\u{{{:02x}}}", c as u32));
        } else {
            out.push(c);
        }
    }
    out
}
