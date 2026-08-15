//! T-920.1 — the one-shot `user_story` → `main_goal` on-disk migration, plus the
//! same-land live-ready body fills (t920 spec §Schema changes + the plan's
//! self-referential risk note).
//!
//! Two passes over the typed corpus, both writing ONLY through
//! [`tbd_tickets::Corpus::write_back`] (render → re-parse → temp+rename — the
//! typed path; no hand-crafted TOML bytes ever land):
//!
//! 1. **Body fills** for the live ready set. The ready-tier check rule
//!    (`check_ready_tier_body`) binds corpus-wide the moment T-920.1 lands, and the
//!    live tree carried six ready work tickets with the five body lists empty —
//!    T-920.1 itself, T-919, and the four T-090 children. Their content is DERIVED
//!    from each ticket's own spec/plan documents (`docs/plans/t-*_plan.md` four
//!    sections), never invented — the T-921 "thin evidence yields thin honest
//!    fields" rule, applied early. Fills are idempotent by emptiness: a ticket
//!    whose five lists are no longer all-empty is reported and left untouched, so
//!    a re-run can never clobber later hand edits.
//! 2. **The rename migration**: every file whose RAW bytes still carry a
//!    `user_story = ` line is loaded (the serde alias parses it into `main_goal`)
//!    and written back — render emits `main_goal` in the same canonical slot, so a
//!    load + write_back IS the migration. Counted from the raw bytes, so the
//!    printed number is exactly the `grep -c '^user_story = '` delta.
//!
//! No sync, no repack: `main_goal` feeds no generated view and no wave.lock input
//! (`wave.lock` must stay byte-identical through this land — acceptance).
//! Idempotent overall: a second run finds zero raw carriers and zero empty fill
//! targets and writes nothing.

use anyhow::{Result, bail};
use std::fs;
use std::path::Path;
use tbd_tickets::{Corpus, Ticket};

/// The five ready-tier line lists this verb fills (acceptance already nonempty on
/// every target — parse-enforced for ready-class).
struct Fill {
    id: &'static str,
    context: &'static [&'static str],
    requirement: &'static [&'static str],
    current_state: &'static [&'static str],
    approach: &'static [&'static str],
    verify: &'static [&'static str],
}

/// Derived, not invented: each block condenses the ticket's own plan document
/// (docs/plans/<id>_plan.md §Context/§Approach/§Risks/§Verification) and spec.
/// Every line stays under the 30-word body cap (check-enforced).
const FILLS: &[Fill] = &[
    Fill {
        id: "T-920.1",
        context: &[
            "user_story sits on 50/1199 tickets and its content is goal-shaped, not persona prose; operator decisions 2026-08-15 (t920 spec Decisions log).",
            "99 tickets carry their id as their title and 341 titles exceed 10 words; body fields are renderable but nothing forces them filled.",
        ],
        requirement: &[
            "Rename user_story to main_goal everywhere with a serde alias; migrate every on-disk carrier through write_back.",
            "Tiered body gates: queued adds main_goal, ready-class adds the six body fields, refused at mark-ready and ship.",
            "Title gate on changed tickets; two shrink-only debt pins measured at land, printed with their instruments.",
        ],
        current_state: &[
            "TicketFile, the typed model, Status::live_ready, ops, check and the board all speak user_story; no tier gate exists beyond the ready-class parse.",
            "Measured 2026-08-15: 440 title-debt tickets (99 id-as-title, 341 over-cap) and 53 live work tickets without main_goal.",
        ],
        approach: &[
            "Serde-alias rename through TicketFile, the typed model, ops, check, sync views and the on-disk carriers via a one-shot write_back verb.",
            "Tier rules in check: idea and ready tiers corpus-wide on work tickets; queued main_goal metered by MAIN_GOAL_DEBT_PIN.",
            "Pre-write refusals in ops::mark_ready and ops::ship naming each empty field; title and main_goal gates in the ops post-image.",
            "Two shrink-only ratchet pins in tbd-tickets, the T-917.3 pattern; the live ready set filled honestly in the same land.",
        ],
        verify: &[
            "cargo test -p tbd-tickets and cargo test -p xtask green; corpus roundtrip prints N/N byte-identical.",
            "grep -c '^user_story = ' over .ai/tickets/T-*.toml prints 0 and '^main_goal = ' prints the carrier count.",
            "cargo xtask ticket check --strict prints check OK; git diff --stat -- .ai/tickets/wave.lock prints empty.",
        ],
    },
    Fill {
        id: "T-919",
        context: &[
            "T-917.3 quarantined 694 over-cap summaries verbatim into migration_legacy behind a shrink-only pin; the prose is preserved but uncategorised.",
            "The ten typed body fields sit empty on those tickets; pass 2 is semantic work, deliberately kept out of the mechanical migration.",
        ],
        requirement: &[
            "Decompose every quarantined wall into the typed body fields in operator-reviewed batches until the pin hits zero.",
            "Delete migration_legacy in the same edit; shrink the ratchet pin by exactly the batch size in the same commit.",
            "Per T-920: each batch also fills the full tier set and repairs titles on the tickets it drains, shrinking those pins too.",
        ],
        current_state: &[
            "669 carriers remain after batch 1 drained 25; walls carry recognisable FIX:/ACCEPTANCE:/Repro: idioms.",
        ],
        approach: &[
            "AI batches of 20-30 tickets per reviewed commit; content is reorganised, never invented — lines fitting no field land in notes.",
            "The anti-blend definitions are the sorting authority; every non-whitespace token of the wall survives into some field or notes.",
        ],
        verify: &[
            "Per batch: cargo test -p tbd-tickets green — the ratchet pins enforce both directions; remaining files pass the reversibility join-proof.",
            "cargo xtask ticket check --strict prints check OK; wave.lock byte-identical per batch.",
        ],
    },
    Fill {
        id: "T-090.4",
        context: &[
            "Enfusion map objects export with pivots at ground, roof or model center, so the basemap can render props at wrong heights across 1M+ Eden objects.",
            "Phase A is the cheap full-catalog screen: one DEM sample per object, detect only, no auto-fix (geometry-aware Phase B is T-090.6).",
        ],
        requirement: &[
            "Compare each exported object's pivot Z against the T-091 DEM at (x, y) with per-kind warn/fail thresholds.",
            "Missing z is a warn, never a fabricated value; no auto-fix writes anywhere.",
        ],
        current_state: &[
            "Blocked on the T-090.3 export and the T-091 DEM being present; the object catalog lives under packages/map-assets/everon/objects.",
            "16-bit DEM quantization on slopes yields false positives; tilted and large props yield false negatives by design — deferred to T-090.6.",
        ],
        approach: &[
            "Offline tool in tools/tbd-tools/src/world: sample the DEM at each instance (x, y) and classify demZ versus pivot z per kind.",
            "Emit a machine- and human-readable report keyed by object id and kind so T-090.6 can consume the deltas.",
        ],
        verify: &[
            "Run the audit over the full exported catalog; every instance reports demZ vs z with its classification.",
            "Missing z counts surface as warns; spot-check known bridges and trees against the report; no auto-fix writes anywhere.",
        ],
    },
    Fill {
        id: "T-090.6",
        context: &[
            "The T-090.4 point audit (pivot Z vs DEM) misses tilted, large and spanning props by design; Phase B classifies with simplified 3D bounds.",
        ],
        requirement: &[
            "For every exported map object use center, rotation and simplified 3D bounds — never full meshes — to classify above, buried or inside.",
            "Overlap detection against neighboring objects; fully automated at the 1M-object scale, no manual eyeballing.",
        ],
        current_state: &[
            "spatial.halfExtentsM and rotationDeg ship in the catalog as the normative geometry; the T-090.3.0 spike proved the localUp to world Z remap.",
        ],
        approach: &[
            "Extend the audit tool in tools/tbd-tools/src/world: build an OBB per instance, sample the DEM at corners and edges, classify per kind.",
            "Neighbor-overlap detection via a spatial grid; concave props carry a confidence note rather than pretending mesh precision.",
        ],
        verify: &[
            "Full-catalog run completes without manual eyeballing; OBB classes reproduce the T-090.4 findings on the point-audit subset.",
            "Known tilted props get flagged; spot-check against Workbench ground truth where available.",
        ],
    },
    Fill {
        id: "T-090.7",
        context: &[
            "Mission Creator will expose AI inside the Eden-style editor; the AI must read the 1M+ object world base layer with Workbench-selection certainty.",
        ],
        requirement: &[
            "ResolvedWorldObject is the exact AI tool shape, exactly as packages/tbd-schema/schema/map-object-resolved.schema.json defines it.",
            "No parallel field names invented in frontend AI code; the schema is the single contract.",
        ],
        current_state: &[
            "The schema slice pinned the ResolvedWorldObject shape; the frontend resolver in world_assets does not yet produce it for the AI surface.",
        ],
        approach: &[
            "Wire the resolver in apps/website/frontend/src/world_assets: prefab plus instance join into the required typed fields, exposed as the AI tool shape.",
            "Audit-trust fields depend on T-090.4/.6 outputs; absent audits render as explicitly unknown, never as trusted placement.",
        ],
        verify: &[
            "ResolvedWorldObject instances validate against the committed schema; frontend AI reads compile against the pinned shape only.",
            "A sample object round-trips prefab plus instance to resolved with every required field populated.",
        ],
    },
    Fill {
        id: "T-090.9",
        context: &[
            "Static world objects render as pixels today; mission makers cannot interrogate them.",
            "Edits to terrain props stay Workbench-only (N7 locked) and Deck GPU picking stays disabled.",
        ],
        requirement: &[
            "Hover tooltip, click-to-inspect panel with Ask AI about this object, taxonomy filter and search, a legend and a Z-trust badge — read-only.",
            "Move, delete and edit of world objects remain Workbench-only; no mutation affordance anywhere.",
        ],
        current_state: &[
            "Depends on the T-090.5 render and the T-090.7 resolver being live; the worker's spatial index is the picking authority.",
        ],
        approach: &[
            "CPU-side picking over the worker's spatial index; tooltip and read-only inspect panel fed by the T-090.7 resolver.",
            "Filter and legend driven by the taxonomy; the Z-trust badge reads the T-090.4/.6 audit flags; no per-frame full-catalog scans.",
        ],
        verify: &[
            "Hover shows the tooltip; click opens read-only inspect with resolved fields; filter, search and legend act on taxonomy classes.",
            "No mutation affordance exists on world objects; editor FPS stays within the HUD budget.",
        ],
    },
];

fn to_vec(lines: &[&str]) -> Vec<String> {
    lines.iter().map(|s| (*s).to_string()).collect()
}

pub fn cmd_migrate_main_goal(root: &Path) -> Result<()> {
    let mut corpus = Corpus::load(root).map_err(anyhow::Error::msg)?;
    let mut to_write: Vec<String> = Vec::new();

    // Pass 1 — live-ready body fills, idempotent by emptiness.
    let mut filled = 0usize;
    for fill in FILLS {
        let Some(ticket) = corpus.tickets.get_mut(fill.id) else {
            bail!(
                "fill target {} is not in the corpus — the fill table is stale",
                fill.id
            );
        };
        let Ticket::Work(w) = ticket else {
            bail!("fill target {} is not a work ticket", fill.id);
        };
        let all_empty = w.context.is_empty()
            && w.requirement.is_empty()
            && w.current_state.is_empty()
            && w.approach.is_empty()
            && w.verify.is_empty();
        if !all_empty {
            println!("{}: body fields already present — untouched", fill.id);
            continue;
        }
        w.context = to_vec(fill.context);
        w.requirement = to_vec(fill.requirement);
        w.current_state = to_vec(fill.current_state);
        w.approach = to_vec(fill.approach);
        w.verify = to_vec(fill.verify);
        filled += 1;
        to_write.push(fill.id.to_string());
    }

    // Pass 2 — the rename migration: raw-byte carriers of the dead spelling.
    // The typed parse already landed them in `main_goal` (serde alias); a
    // write_back re-renders the canonical form, which spells `main_goal`.
    let dir = corpus.tickets_dir();
    let mut migrated = 0usize;
    let mut names: Vec<String> = fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.starts_with("T-") && n.ends_with(".toml"))
        .collect();
    names.sort();
    for name in names {
        let text = fs::read_to_string(dir.join(&name))?;
        if text.lines().any(|l| l.starts_with("user_story = ")) {
            migrated += 1;
            let id = name.trim_end_matches(".toml").to_string();
            if !to_write.contains(&id) {
                to_write.push(id);
            }
        }
    }

    if to_write.is_empty() {
        println!(
            "nothing to write — migration already ran (0 raw user_story carriers, 0 empty fill targets)"
        );
        return Ok(());
    }
    corpus.write_back(&to_write).map_err(anyhow::Error::msg)?;
    println!(
        "{migrated} file(s) migrated user_story -> main_goal; {filled} live-ready body fill(s); {} file(s) written via Corpus::write_back",
        to_write.len()
    );
    Ok(())
}
