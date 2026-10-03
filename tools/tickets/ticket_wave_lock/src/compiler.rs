//! The wave lock compiler: ticket files, the previous lock and git history in, a [`WaveLock`] out.
//!
//! **Role:** chooses the packing width, runs the greedy, derives the ledger base and floor from
//! the `wave N CLOSED` commits, carries the pending emptied waves and an optional reservation,
//! and numbers the waves ([`compile`], [`compile_with_cap`], [`compile_reserving`]).
//! **Position:** over `ticket_views`, `packing`, `parking` and `history`; called by the repack in
//! `persistence` and re-derived in part by `verification`.
//! **Signals & state:** none in memory; reads the ticket files, the `TBD_MAX_CONCURRENT`
//! variable and git history, and prints packing warnings to stderr.
//! **Invariants:** the same inputs compile to the same lock; wave 0 keeps label 0; open waves
//! number from one past the highest of the ledger base, the ledger floor and every pending
//! emptied label; an incidental repack keeps the previous lock's width unless
//! `TBD_MAX_CONCURRENT` asks for another.

use crate::error::{Error, Result};
use crate::packing::{greedy_waves, reserved_entry};
use crate::parking::{carry_emptied, snapshots, wave_zero};
use crate::{LOCK_VERSION, LockWave, TicketView, WaveLock, load_views, max_concurrent};
use std::collections::BTreeSet;
use std::path::Path;
use ticket_model::TicketId;

/// The ledger base the lock records: the newest standing `wave N CLOSED` number reachable from
/// `HEAD` in `root`'s git history, derived at pack time and never from a constant. 0 when no
/// marker is reachable (a scratch folder, a fixture tree, a tree before its first close), which
/// numbers open waves from 1. A number beyond `u32` counts as no marker. Fails on a shallow
/// clone, whose ledger is incomplete.
pub(super) fn ledger_base(root: &Path) -> Result<u32> {
    // An error is the shallow-clone refusal: an incomplete ledger never derives a base.
    Ok(crate::history::newest_close_base(root)
        .map_err(Error::msg)?
        .and_then(|n| u32::try_from(n).ok())
        .unwrap_or(0))
}

/// The ledger floor: the highest wave number any standing close marker claims, never lower than
/// `wave_base`. No label at or below it may be issued.
///
/// `wave_base` names the newest close commit; the floor names the numbers already spent by a
/// close that still stands, which differ when programs close out of numerical order (see
/// [`crate::history::max_close_claim`]). The platform wave-number check accepts exactly
/// `floor + 1` as the next close, so the lock numbers from the floor. On a ledger closed in
/// order the newest marker carries the highest claim, the floor equals `wave_base`, and nothing
/// renumbers.
pub(super) fn ledger_floor(root: &Path, wave_base: u32) -> Result<u32> {
    let claim = crate::history::max_close_claim(root)
        .map_err(Error::msg)?
        .and_then(|n| u32::try_from(n).ok())
        .unwrap_or(0);
    Ok(wave_base.max(claim))
}

/// Builds the lock from already packed open waves: wave 0 from `baseline` and the parked views,
/// open waves labelled past the base, the floor and every pending emptied label, and the
/// corpus-wide snapshots.
pub(super) fn assemble(
    views: &[TicketView],
    baseline: &BTreeSet<TicketId>,
    open_waves: Vec<Vec<TicketId>>,
    cap: usize,
    wave_base: u32,
    ledger_floor: u32,
    emptied: Vec<LockWave>,
) -> WaveLock {
    let (owns, depends_on, pack_last) = snapshots(views);
    let mut waves = Vec::new();
    let zero = wave_zero(views, baseline);
    if !zero.is_empty() {
        waves.push(LockWave {
            n: 0,
            tickets: zero,
        });
    }
    // Open waves continue the close-marker ledger and number past every pending emptied label:
    // those labels are reserved for their close markers, so an open wave never takes the label
    // of a wave waiting to close. Wave 0 is a ledger, not a schedule; its label stays 0.
    let floor = emptied
        .iter()
        .map(|e| e.n)
        .fold(wave_base.max(ledger_floor), u32::max);
    for (i, tickets) in open_waves.into_iter().enumerate() {
        waves.push(LockWave {
            n: floor + 1 + i as u32,
            tickets,
        });
    }
    WaveLock {
        version: LOCK_VERSION,
        max_concurrent: cap,
        wave_base,
        pack_last,
        waves,
        emptied,
        owns,
        depends_on,
    }
}

/// Compiles the lock from the ticket files under `root`: wave 0 on `baseline`, open waves from
/// the greedy, labelled from the close-marker ledger past every pending emptied label, and the
/// emptied section carried from `prev`, the previous lock. `prev = None` carries nothing (a
/// first compile, a fixture tree). Fails when the ticket files do not load, on a dependency
/// cycle among dispatchable tickets, and on a shallow clone.
pub fn compile(
    root: &Path,
    baseline: &BTreeSet<TicketId>,
    prev: Option<&WaveLock>,
) -> Result<WaveLock> {
    compile_with_cap(root, baseline, prev, None)
}

/// [`compile`] with the width supplied as `cap_override` instead of read from the environment
/// (`None` falls back to the environment, then the previous lock, then 8).
///
/// Tests pack a narrow plan through this rather than setting `TBD_MAX_CONCURRENT`: the variable
/// is process state shared by every thread of a multi-threaded `cargo test`, so setting it would
/// re-pack whichever test calls the packer at that instant.
pub fn compile_with_cap(
    root: &Path,
    baseline: &BTreeSet<TicketId>,
    prev: Option<&WaveLock>,
    cap_override: Option<usize>,
) -> Result<WaveLock> {
    compile_inner(root, baseline, prev, cap_override, &[])
}

/// [`compile`] with an explicit pending-close reservation: `reserve` becomes one more
/// `[[emptied]]` entry after the carried ones. An empty `reserve` compiles exactly as
/// [`compile`]. This is the only way a wave membership the emptied carry cannot see enters the
/// lock, and it still enters through the repack. Fails as [`compile`] does, and when a reserved
/// id is unknown, repeated, not shipped or cancelled, or already pending close.
pub fn compile_reserving(
    root: &Path,
    baseline: &BTreeSet<TicketId>,
    prev: Option<&WaveLock>,
    reserve: &[TicketId],
) -> Result<WaveLock> {
    compile_inner(root, baseline, prev, None, reserve)
}

/// The compile behind the three public entry points: width choice, greedy, ledger, carry,
/// reservation, assembly.
pub(super) fn compile_inner(
    root: &Path,
    baseline: &BTreeSet<TicketId>,
    prev: Option<&WaveLock>,
    cap_override: Option<usize>,
    reserve: &[TicketId],
) -> Result<WaveLock> {
    let views = load_views(root)?;
    let mut warnings = Vec::new();
    // A repack never reshapes a plan nobody asked it to reshape. `ticket ship` repacks with no
    // environment, and the default width of 8 would regroup a wave packed narrower while it
    // runs, leaving `wave --close` nothing to close. So the width is: an explicit override or a
    // nonempty `TBD_MAX_CONCURRENT` (a caller asking on purpose), else the width the previous
    // lock records, else the default.
    let cap = match (
        cap_override.or_else(|| {
            std::env::var("TBD_MAX_CONCURRENT")
                .ok()
                .filter(|v| !v.is_empty())
                .and_then(|v| v.parse().ok())
        }),
        prev,
    ) {
        (Some(n), _) if n > 0 => n,
        (None, Some(p)) if p.max_concurrent > 0 => p.max_concurrent,
        _ => max_concurrent(),
    };
    let open = greedy_waves(&views, cap, &mut warnings)?;
    for w in &warnings {
        eprintln!("wave repack: warning: {w}");
    }
    let wave_base = ledger_base(root)?;
    let floor = ledger_floor(root, wave_base)?;
    let mut emptied = carry_emptied(prev, &views, wave_base, floor);
    if let Some(entry) = reserved_entry(&views, reserve, &emptied, floor)? {
        emptied.push(entry);
    }
    Ok(assemble(
        &views, baseline, open, cap, wave_base, floor, emptied,
    ))
}
