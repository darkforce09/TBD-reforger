//! Compiler.

use super::*;

/// The ledger base the lock records: derived from `root`'s git history at pack time,
/// never from a constant. 0 when no marker is reachable (scratch dirs, stub roots, unit tests,
/// a tree before its first close), which keeps open waves at 1..N. The u32
/// conversion is total in practice — the subject authority admits digits only, and no ledger
/// approaches the boundary; a number that somehow overflows is treated as no marker.
pub(super) fn ledger_base(root: &Path) -> Result<u32> {
    // Err is the shallow-clone refusal — an unreadable ledger never derives a base.
    Ok(crate::wave_lock::history::newest_close_base(root)
        .map_err(|e| anyhow::anyhow!(e))?
        .and_then(|n| u32::try_from(n).ok())
        .unwrap_or(0))
}

/// The number below which no label may be issued: the highest wave any reachable close marker
/// CLAIMS, never lower than the base.
///
/// `wave_base` answers "which commit is the wave boundary" and is derived newest-first;
/// this answers "which numbers are already spent by a close that still stands", and the oracle
/// (the platform wave-number check) will accept exactly `floor + 1`. Numbering
/// the lock from anything else is how the two drifted 13 labels apart — see
/// [`crate::wave_lock::history::max_close_claim`] for the measured ledger that did it.
///
/// On a healthy ledger the newest marker also carries the highest claim, so this equals
/// `wave_base` and nothing renumbers.
pub(super) fn ledger_floor(root: &Path, wave_base: u32) -> Result<u32> {
    let claim = crate::wave_lock::history::max_close_claim(root)
        .map_err(|e| anyhow::anyhow!(e))?
        .and_then(|n| u32::try_from(n).ok())
        .unwrap_or(0);
    Ok(wave_base.max(claim))
}

pub(super) fn assemble(
    views: &[TicketView],
    baseline: &BTreeSet<String>,
    open_waves: Vec<Vec<String>>,
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
    // Open waves continue the close-marker ledger (module header §NUMBERING), numbering past
    // every PENDING emptied label too: those labels are reserved for their close
    // markers, so the first open wave is max(wave_base, highest pending) + 1 and a relabel
    // can never collide with a wave that is waiting to close. Wave 0 is a LEDGER, not a
    // schedule — its label never moves off 0.
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

/// Compile the lock from the ticket tree: wave 0 on `baseline`, open waves from the greedy,
/// labeled from the close-marker ledger past every pending emptied label, and the emptied
/// section carried from `prev` — the previous committed lock. `None` compiles a tree with no
/// lock history and carries nothing (first-ever compile, migration, fixture trees).
pub fn compile(
    root: &Path,
    baseline: &BTreeSet<String>,
    prev: Option<&WaveLock>,
) -> Result<WaveLock> {
    compile_with_cap(root, baseline, prev, None)
}

/// [`compile`] with the width supplied instead of read from the environment.
///
/// Exists so tests can pack a deliberately narrow plan without `set_var`: `TBD_MAX_CONCURRENT` is
/// process state, `cargo test` is multi-threaded, and a test that mutates it re-packs whatever
/// sibling happens to call the packer at that instant — measured 2026-09-06, one such test made
/// `candidates_sort_by_order_then_id_never_glob_order` fail about one run in three. Same lesson as
/// the cwd race this file's neighbours already carry, arriving through a different global.
pub fn compile_with_cap(
    root: &Path,
    baseline: &BTreeSet<String>,
    prev: Option<&WaveLock>,
    cap_override: Option<usize>,
) -> Result<WaveLock> {
    compile_inner(root, baseline, prev, cap_override, &[])
}

/// [`compile`] with an explicit pending-close reservation — see `reserved_entry`. An empty
/// `reserve` compiles byte-identically to [`compile`]; this is the only way a membership the
/// derived carry cannot see enters the lock, and it still enters it through the repack.
pub fn compile_reserving(
    root: &Path,
    baseline: &BTreeSet<String>,
    prev: Option<&WaveLock>,
    reserve: &[String],
) -> Result<WaveLock> {
    compile_inner(root, baseline, prev, None, reserve)
}

pub(super) fn compile_inner(
    root: &Path,
    baseline: &BTreeSet<String>,
    prev: Option<&WaveLock>,
    cap_override: Option<usize>,
    reserve: &[String],
) -> Result<WaveLock> {
    let views = load_views(root)?;
    let mut warnings = Vec::new();
    // A REPACK MUST NOT RESHAPE A PLAN NOBODY ASKED IT TO RESHAPE.
    //
    // `max_concurrent()` reads `TBD_MAX_CONCURRENT` and defaults to 8, and `ticket ship`'s
    // lifecycle hook repacks with no environment at all. Measured 2026-09-06: wave 236 was packed
    // at the run's 3-wide cap, then a later ship hook re-packed the whole lock at 8 and
    // wave 236 came back holding eight different tickets — the wave that had just been gated no
    // longer existed in the plan, and `wave --close` had nothing to close. The lock RECORDS its
    // own `max_concurrent`; an incidental repack must honour it. An explicit
    // `TBD_MAX_CONCURRENT` still wins, because that is a caller asking on purpose.
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
