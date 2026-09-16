//! Role: ledger.
//! Position: `streaming/memory/budget` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::*;

impl Ledger {
    /// A ledger with `budget` bytes and nothing held.
    #[must_use]
    pub fn with_budget(budget: u64) -> Self {
        Self {
            budget,
            entries: [Entry::default(); 7],
            total_peak: 0,
            sat_floor: None,
            sat_raised: 0,
        }
    }
}

impl Ledger {
    /// The ceiling, in bytes.
    #[must_use]
    pub fn budget(&self) -> u64 {
        self.budget
    }
}

impl Ledger {
    /// Total declared bytes currently held across every asset.
    #[must_use]
    pub fn held_total(&self) -> u64 {
        self.entries.iter().map(|e| e.held).sum()
    }
}

impl Ledger {
    /// High-water mark of [`Ledger::held_total`].
    #[must_use]
    pub fn total_peak(&self) -> u64 {
        self.total_peak
    }
}

impl Ledger {
    /// One asset's row.
    #[must_use]
    pub fn entry(&self, a: Asset) -> Entry {
        self.entries[a.index()]
    }
}

impl Ledger {
    /// Would `bytes` be servable right now? Pure — asks without recording.
    #[must_use]
    pub fn decide(&self, bytes: u64) -> Decision {
        if self.held_total().saturating_add(bytes) <= self.budget {
            Decision::Ok
        } else if bytes <= self.budget {
            Decision::Degrade
        } else {
            Decision::Refuse
        }
    }
}

impl Ledger {
    /// Ask for `bytes` on `a`'s behalf; on [`Decision::Ok`] the bytes are recorded as held.
    pub fn reserve(&mut self, a: Asset, bytes: u64) -> Decision {
        let d = self.decide(bytes);
        if d == Decision::Ok {
            let e = &mut self.entries[a.index()];
            e.held = e.held.saturating_add(bytes);
            e.peak = e.peak.max(e.held);
            self.total_peak = self.total_peak.max(self.held_total());
        }
        d
    }
}

impl Ledger {
    /// Record `bytes` on `a` **unconditionally** — no decision, no refusal.
    pub fn hold(&mut self, a: Asset, bytes: u64) {
        let e = &mut self.entries[a.index()];
        e.held = e.held.saturating_add(bytes);
        e.peak = e.peak.max(e.held);
        self.total_peak = self.total_peak.max(self.held_total());
    }
}

impl Ledger {
    /// Replace `a`'s held bytes outright — a forecast superseded by what was actually allocated. The peak still ratchets, so a forecast larger than the truth stays in the record.
    pub fn set_held(&mut self, a: Asset, bytes: u64) {
        let e = &mut self.entries[a.index()];
        e.held = bytes;
        e.peak = e.peak.max(e.held);
        self.total_peak = self.total_peak.max(self.held_total());
    }
}

impl Ledger {
    /// Give `bytes` back. Saturating: releasing more than is held is a bookkeeping bug, and wrapping to `u64::MAX` would turn it into a permanently exhausted budget.
    pub fn release(&mut self, a: Asset, bytes: u64) {
        let e = &mut self.entries[a.index()];
        e.held = e.held.saturating_sub(bytes);
    }
}

impl Ledger {
    /// Record measured linear-memory growth against `a` (see [`Entry::growth`]).
    pub fn add_growth(&mut self, a: Asset, bytes: u64) {
        let e = &mut self.entries[a.index()];
        e.growth = e.growth.saturating_add(bytes);
    }
}

impl Ledger {
    /// Record the satellite mip level the budget settled on, and how many levels it cost.
    pub fn set_satellite_floor(&mut self, base: usize, raised: u32) {
        self.sat_floor = Some(base);
        self.sat_raised = raised;
    }
}

impl Ledger {
    /// The live satellite floor, once one has been chosen.
    #[must_use]
    pub fn satellite_floor(&self) -> Option<usize> {
        self.sat_floor
    }
}

impl Ledger {
    /// How many levels the budget raised that floor by.
    #[must_use]
    pub fn satellite_raised(&self) -> u32 {
        self.sat_raised
    }
}
