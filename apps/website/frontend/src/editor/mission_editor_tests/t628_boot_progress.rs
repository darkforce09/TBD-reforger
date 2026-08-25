use super::boot_progress::{
    fmt_bytes_pair, fmt_files_pair, percent, split_range, BootEvent, BootProgress, BootSeg,
    Ordered, PLANNED_SATELLITE_BYTES, PLANNED_TERRAIN_BYTES, PLANNED_WORLD_BYTES, SAT_CHUNK_BYTES,
    SAT_FETCH_CONCURRENCY, STREAM_REPORT_BYTES,
};
use super::BOOT_HANDOVER_MS;

/// everon `everon-sat.tbd-sat`, read off the live index at `/map-assets/everon/satellite/`
/// (2026-08-01): file 152,713,114 B; level 0 = 4 tiles of 28,326,346 / 21,632,714 / 27,555,806
/// / 33,042,794 starting at 2,644.
const L0_TILE0_OFFSET: u64 = 2_644;
const L0_TILE0_LENGTH: u64 = 28_326_346;
const FILE_BYTES: u64 = 152_713_114;

// ── split_range: the spans must rebuild the tile byte for byte ────────────────────────────

#[test]
fn split_range_covers_the_tile_exactly_contiguously_and_in_order() {
    let spans = split_range(L0_TILE0_OFFSET, L0_TILE0_LENGTH, SAT_CHUNK_BYTES);
    assert!(!spans.is_empty(), "a 28 MB tile must produce requests");
    assert_eq!(
        spans[0].0, L0_TILE0_OFFSET,
        "the run must start at the tile's own offset"
    );
    assert_eq!(
        spans[spans.len() - 1].1,
        L0_TILE0_OFFSET + L0_TILE0_LENGTH - 1,
        "the run must end on the tile's last byte (Range ends are inclusive)"
    );
    let mut covered = 0u64;
    for (i, &(start, end)) in spans.iter().enumerate() {
        assert!(end >= start, "span {i} is inverted");
        assert!(
            end - start < SAT_CHUNK_BYTES,
            "span {i} is larger than one request"
        );
        if i > 0 {
            assert_eq!(
                start,
                spans[i - 1].1 + 1,
                "span {i} must resume exactly where {} stopped — a gap loses bytes, an \
                 overlap duplicates them, and concatenation cannot tell either from a good run",
                i - 1
            );
        }
        covered += end - start + 1;
    }
    assert_eq!(
        covered, L0_TILE0_LENGTH,
        "the spans must cover the tile exactly"
    );
}

#[test]
fn split_range_degenerate_inputs_do_not_loop_or_overrun() {
    assert!(
        split_range(2_644, 0, SAT_CHUNK_BYTES).is_empty(),
        "a zero-length tile asks for nothing"
    );
    assert_eq!(
        split_range(100, 10, SAT_CHUNK_BYTES),
        vec![(100, 109)],
        "a tile below one chunk is one request"
    );
    assert_eq!(
        split_range(100, 3, 0),
        vec![(100, 100), (101, 101), (102, 102)],
        "a zero chunk must degrade to 1 B a request, not spin"
    );
}

// ── Ordered: the scrambled-texture guard ─────────────────────────────────────────────────

#[test]
fn completions_arriving_out_of_order_reassemble_in_request_order() {
    // The network hands back 3, 0, 2, 1 — the shape `buffer_unordered` actually produces.
    let mut slots: Ordered<&str> = Ordered::new(4);
    for (i, body) in [(3, "d"), (0, "a"), (2, "c"), (1, "b")] {
        assert!(slots.put(i, body), "slot {i} must accept its body");
    }
    assert_eq!(
        slots.finish(),
        Some(vec!["a", "b", "c", "d"]),
        "the assembled run must be in REQUEST order, not completion order — `commit_mip` \
         uploads element n at mip.tiles[n]'s (x, y), so completion order here is a scrambled \
         satellite texture that reads as a rendering bug"
    );
}

#[test]
fn a_dropped_completion_fails_instead_of_shifting_the_run() {
    let mut slots: Ordered<u8> = Ordered::new(3);
    assert!(slots.put(0, 1));
    assert!(slots.put(2, 3));
    assert_eq!(
        slots.finish(),
        None,
        "a missing chunk must fail the whole fetch; a 2-element Vec would silently shift \
         every tile after the gap"
    );
}

#[test]
fn an_out_of_range_slot_is_refused_rather_than_dropped() {
    let mut slots: Ordered<u8> = Ordered::new(2);
    assert!(
        !slots.put(2, 9),
        "an index past the plan must be reported so the caller aborts — silently ignoring \
         it loses a chunk the length check would then blame on the server"
    );
}

// ── percent / byte formatting ────────────────────────────────────────────────────────────

#[test]
fn percent_is_clamped_and_survives_a_zero_total() {
    assert!((percent(0, FILE_BYTES) - 0.0).abs() < 1e-9);
    assert!((percent(FILE_BYTES / 2, FILE_BYTES) - 50.0).abs() < 0.001);
    assert!((percent(FILE_BYTES, FILE_BYTES) - 100.0).abs() < 1e-9);
    assert!(
        (percent(FILE_BYTES + 4096, FILE_BYTES) - 100.0).abs() < 1e-9,
        "a body longer than the index promised must not push the fill past its track"
    );
    assert!(
        (percent(1, 0) - 0.0).abs() < 1e-9,
        "nothing measured is nothing done, not a division"
    );
}

#[test]
fn the_byte_pair_reads_in_one_unit_and_matches_the_manifest() {
    assert_eq!(
        fmt_bytes_pair(0, FILE_BYTES),
        "0.0 MB / 152.7 MB",
        "the total must read as the manifest's own `bytes` field does"
    );
    assert_eq!(fmt_bytes_pair(47_300_000, FILE_BYTES), "47.3 MB / 152.7 MB");
    assert_eq!(
        fmt_bytes_pair(4_194_304, 42_152_810),
        "4.2 MB / 42.2 MB",
        "the 8192-limit device fetches level 1 down — 42 MB, not 152"
    );
    assert_eq!(
        fmt_bytes_pair(500, 900),
        "500 B / 900 B",
        "a sub-KB total must not read as 0.0 MB / 0.0 MB"
    );
}

// ── the one bar: weighting, monotonicity, clamping, and reaching 100% ────────────────────

/// The world segment's real shape at boot, measured on the live stack: 7 `WorldHost::init`
/// files + 2 label files + 625 density bins are declared up front, and the chunk batch the
/// residency pins declares itself before it fetches.
const WORLD_STATIC_FILES: u64 = 7 + 2 + 625;
const WORLD_CHUNK_FILES: u64 = 200;

/// Drive the whole boot the way the loaders do, in the order they do it.
fn boot_to_completion() -> BootProgress {
    let mut p = BootProgress::new();
    p.apply(BootEvent::Files(BootSeg::World, WORLD_STATIC_FILES));
    p.apply(BootEvent::Budget(BootSeg::Mission, 2_032));
    p.apply(BootEvent::Done(BootSeg::Mission, 2_032));
    p.apply(BootEvent::Finish(BootSeg::Mission));
    p.apply(BootEvent::Budget(BootSeg::Terrain, PLANNED_TERRAIN_BYTES));
    p.apply(BootEvent::Done(BootSeg::Terrain, PLANNED_TERRAIN_BYTES));
    p.apply(BootEvent::Finish(BootSeg::Terrain));
    p.apply(BootEvent::Budget(
        BootSeg::Satellite,
        PLANNED_SATELLITE_BYTES,
    ));
    p.apply(BootEvent::Done(BootSeg::Satellite, PLANNED_SATELLITE_BYTES));
    p.apply(BootEvent::Finish(BootSeg::Satellite));
    p.apply(BootEvent::Files(BootSeg::World, WORLD_CHUNK_FILES));
    p.apply(BootEvent::Done(
        BootSeg::World,
        WORLD_STATIC_FILES + WORLD_CHUNK_FILES,
    ));
    p.apply(BootEvent::Finish(BootSeg::World));
    p
}

#[test]
fn nothing_is_claimed_before_anything_is_measured() {
    let mut p = BootProgress::new();
    assert!(
        (p.percent() - 0.0).abs() < 1e-9,
        "a boot that has measured nothing is at 0% — the old sweep's whole problem was that it \
         looked identical at 0 and at 99"
    );
    // Budgets alone move nothing: they are denominators, not work.
    p.apply(BootEvent::Budget(BootSeg::Terrain, PLANNED_TERRAIN_BYTES));
    p.apply(BootEvent::Budget(
        BootSeg::Satellite,
        PLANNED_SATELLITE_BYTES,
    ));
    p.apply(BootEvent::Files(BootSeg::World, WORLD_STATIC_FILES));
    assert!(
        (p.percent() - 0.0).abs() < 1e-9,
        "knowing how big the download is is not the same as having downloaded any of it"
    );
    p.apply(BootEvent::Done(BootSeg::Terrain, PLANNED_TERRAIN_BYTES / 2));
    assert!(p.percent() > 0.0, "real bytes must move the bar");
}

#[test]
fn one_bar_spans_the_whole_boot_and_never_resets_between_segments() {
    let mut p = BootProgress::new();
    p.apply(BootEvent::Files(BootSeg::World, WORLD_STATIC_FILES));
    let mut seen: Vec<f64> = vec![p.percent()];
    // Mission, then terrain, then satellite, then world — the four stages in boot order.
    p.apply(BootEvent::Budget(BootSeg::Mission, 2_032));
    p.apply(BootEvent::Done(BootSeg::Mission, 2_032));
    p.apply(BootEvent::Finish(BootSeg::Mission));
    seen.push(p.percent());
    p.apply(BootEvent::Budget(BootSeg::Terrain, PLANNED_TERRAIN_BYTES));
    for _ in 0..4 {
        p.apply(BootEvent::Done(BootSeg::Terrain, PLANNED_TERRAIN_BYTES / 4));
        seen.push(p.percent());
    }
    p.apply(BootEvent::Finish(BootSeg::Terrain));
    seen.push(p.percent());
    p.apply(BootEvent::Budget(
        BootSeg::Satellite,
        PLANNED_SATELLITE_BYTES,
    ));
    for _ in 0..4 {
        p.apply(BootEvent::Done(
            BootSeg::Satellite,
            PLANNED_SATELLITE_BYTES / 4,
        ));
        seen.push(p.percent());
    }
    p.apply(BootEvent::Finish(BootSeg::Satellite));
    seen.push(p.percent());
    p.apply(BootEvent::Files(BootSeg::World, WORLD_CHUNK_FILES));
    p.apply(BootEvent::Done(
        BootSeg::World,
        WORLD_STATIC_FILES + WORLD_CHUNK_FILES,
    ));
    p.apply(BootEvent::Finish(BootSeg::World));
    seen.push(p.percent());

    for w in seen.windows(2) {
        assert!(
            w[1] >= w[0],
            "the bar must never step back — it went {:.3} → {:.3}. Restarting per stage is \
             exactly what T-627 did and what the operator rejected",
            w[0],
            w[1]
        );
    }
    // Crossing a segment boundary must not drop the bar to zero.
    assert!(
        seen.iter().skip(2).all(|v| *v > 0.0),
        "no reading after the first stage may be 0%: that is a reset, not one bar"
    );
    assert!((seen[seen.len() - 1] - 100.0).abs() < 1e-9);
}

#[test]
fn a_budget_that_grows_holds_the_bar_instead_of_rewinding_it() {
    let mut p = BootProgress::new();
    // The world's static plan lands, and the init + label files complete against it…
    p.apply(BootEvent::Files(BootSeg::World, WORLD_STATIC_FILES));
    p.apply(BootEvent::Done(BootSeg::World, 9));
    let before = p.percent();
    // …then the residency pins the boot camera and 200 chunk files join the same segment.
    p.apply(BootEvent::Files(BootSeg::World, WORLD_CHUNK_FILES));
    assert!(
        p.raw() < before,
        "the arithmetic really does dip here — 9/634 is a bigger fraction than 9/834. If this \
         assert fails the test is no longer exercising the case it exists for"
    );
    assert!(
        (p.percent() - before).abs() < 1e-9,
        "the bar must ABSORB the larger budget by holding, not by rewinding: it read {before:.4} \
         and then {:.4}",
        p.percent()
    );
    // And it resumes as soon as real work passes the mark it held.
    p.apply(BootEvent::Done(BootSeg::World, 400));
    assert!(p.percent() > before, "real work past the hold must move it");
}

#[test]
fn a_weight_that_grows_holds_the_bar_instead_of_rewinding_it() {
    let mut p = BootProgress::new();
    p.apply(BootEvent::Budget(BootSeg::Terrain, PLANNED_TERRAIN_BYTES));
    p.apply(BootEvent::Done(BootSeg::Terrain, PLANNED_TERRAIN_BYTES / 2));
    let before = p.percent();
    // A 16384-limit GPU takes level 0 too: the satellite's real budget is 152.7 MB, not the
    // 42.2 MB planned — so the denominator jumps and every completed byte is worth less.
    p.apply(BootEvent::Budget(BootSeg::Satellite, 152_710_470));
    assert!(
        p.raw() < before,
        "a satellite 3.6× the planned size really does shrink everything else's share"
    );
    assert!(
        (p.percent() - before).abs() < 1e-9,
        "learning the device's real satellite size must not rewind the bar"
    );
}

#[test]
fn a_segment_that_overruns_its_promised_budget_is_clamped_to_its_own_share() {
    let mut p = BootProgress::new();
    p.apply(BootEvent::Budget(BootSeg::Terrain, PLANNED_TERRAIN_BYTES));
    p.apply(BootEvent::Done(BootSeg::Terrain, PLANNED_TERRAIN_BYTES));
    let honest = p.percent();
    // A `content-length` that undercounts the body (a proxy re-encoding it, say) must not let
    // the terrain segment spend the satellite's and the world's share of the track.
    p.apply(BootEvent::Done(BootSeg::Terrain, PLANNED_TERRAIN_BYTES * 4));
    assert!(
        (p.percent() - honest).abs() < 1e-9,
        "a segment that overruns is clamped at its own weight — it read {honest:.4} then {:.4}",
        p.percent()
    );
    let expected = 100.0 * PLANNED_TERRAIN_BYTES as f64
        / (PLANNED_TERRAIN_BYTES + PLANNED_SATELLITE_BYTES + PLANNED_WORLD_BYTES) as f64;
    assert!(
        (honest - expected).abs() < 0.001,
        "a finished terrain is worth exactly its weight's share: {honest:.3} vs {expected:.3}"
    );
}

#[test]
fn the_bar_can_never_exceed_one_hundred() {
    let mut p = BootProgress::new();
    for seg in BootSeg::ALL {
        p.apply(BootEvent::Budget(seg, 1_000));
        p.apply(BootEvent::Files(seg, 10));
        p.apply(BootEvent::Done(seg, u64::MAX));
        assert!(
            p.percent() <= 100.0,
            "{seg:?} pushed the bar to {:.4} — past the end of its own track",
            p.percent()
        );
    }
    p.apply(BootEvent::Done(BootSeg::World, u64::MAX));
    assert!(
        (p.percent() - 100.0).abs() < 1e-9,
        "saturating every segment reads 100%, not 400%"
    );
}

#[test]
fn every_segment_finishing_reads_exactly_one_hundred_even_when_one_failed() {
    // The failure shape the overlay has to survive: the DEM never arrived, so its segment has
    // no budget and no bytes at all — but the boot still ends and the overlay still has to come
    // down on a full bar rather than park at 49% forever.
    let mut p = BootProgress::new();
    p.apply(BootEvent::Files(BootSeg::World, WORLD_STATIC_FILES));
    p.apply(BootEvent::Budget(
        BootSeg::Satellite,
        PLANNED_SATELLITE_BYTES,
    ));
    p.apply(BootEvent::Done(BootSeg::Satellite, PLANNED_SATELLITE_BYTES));
    p.apply(BootEvent::Done(BootSeg::World, WORLD_STATIC_FILES));
    assert!(!p.is_complete());
    assert!(p.percent() < 100.0, "an unfinished boot is not a full bar");
    for seg in BootSeg::ALL {
        p.apply(BootEvent::Finish(seg));
    }
    assert!(p.is_complete());
    assert!(
        (p.percent() - 100.0).abs() < 1e-9,
        "every loader has reported in, so the bar reads 100% — it read {:.4}. A hand-over on a \
         bar that stopped short is the failure this slice exists to remove",
        p.percent()
    );
    assert!((boot_to_completion().percent() - 100.0).abs() < 1e-9);
}

#[test]
fn a_weightless_segment_redistributes_its_share_to_the_others() {
    // The mission document starts weightless (its size is unknowable until its headers land),
    // so before it reports the other three divide the whole bar between them…
    let mut without = BootProgress::new();
    without.apply(BootEvent::Budget(BootSeg::Terrain, PLANNED_TERRAIN_BYTES));
    without.apply(BootEvent::Done(BootSeg::Terrain, PLANNED_TERRAIN_BYTES));
    let share_without = without.percent();

    // …and the moment it weighs 142 MB, the terrain is worth materially less of the track.
    let mut with = BootProgress::new();
    with.apply(BootEvent::Budget(BootSeg::Mission, 142_000_000));
    with.apply(BootEvent::Budget(BootSeg::Terrain, PLANNED_TERRAIN_BYTES));
    with.apply(BootEvent::Done(BootSeg::Terrain, PLANNED_TERRAIN_BYTES));
    let share_with = with.percent();

    let denom_without = PLANNED_TERRAIN_BYTES + PLANNED_SATELLITE_BYTES + PLANNED_WORLD_BYTES;
    assert!(
        (share_without - 100.0 * PLANNED_TERRAIN_BYTES as f64 / denom_without as f64).abs() < 0.001,
        "with no mission weight the terrain is its share of the other three"
    );
    assert!(
        (share_with - 100.0 * PLANNED_TERRAIN_BYTES as f64 / (denom_without + 142_000_000) as f64)
            .abs()
            < 0.001,
        "a 142 MB mission document takes its own share of the bar — the T-060 scale case is \
         exactly why the document cannot be treated as a rounding error"
    );
    assert!(
        share_with < share_without / 2.0,
        "a mission bigger than the whole map must take more than half the track: {share_with:.2} \
         vs {share_without:.2}"
    );
}

#[test]
fn the_weights_are_the_live_measurements_and_the_map_dominates_them() {
    assert_eq!(
        PLANNED_TERRAIN_BYTES, 71_911_548,
        "the terrain weight is the `content-length` of \
         /map-assets/everon/dem/everon-dem-16bit.png, measured 2026-08-01"
    );
    assert_eq!(
        PLANNED_SATELLITE_BYTES, 42_152_810,
        "the satellite weight is the tbd-sat index's own tile lengths from level 1 down — what \
         an 8192-limit maxTextureDimension2D actually uploads"
    );
    // The whole reason weights exist: a naive equal-quarters bar would stall in two places.
    let total = PLANNED_TERRAIN_BYTES + PLANNED_SATELLITE_BYTES + PLANNED_WORLD_BYTES;
    let dem_and_sat = PLANNED_TERRAIN_BYTES + PLANNED_SATELLITE_BYTES;
    assert!(
        dem_and_sat * 100 / total >= 80,
        "the DEM and satellite are ~81% of the map's bytes — an equal-quarters bar would give \
         them half the track and crawl through both, then race through the rest"
    );
    assert!(
        STREAM_REPORT_BYTES > 0 && PLANNED_TERRAIN_BYTES / STREAM_REPORT_BYTES >= 100,
        "the stream must report at least ~100 times across the DEM, or the terrain segment is \
         a per-file bar again: 0% for the whole download, then a snap"
    );
}

#[test]
fn the_stage_name_follows_the_first_unfinished_segment() {
    let mut p = BootProgress::new();
    assert_eq!(p.stage(), BootSeg::Mission);
    assert_eq!(p.stage().title(), "Loading mission…");
    p.apply(BootEvent::Finish(BootSeg::Mission));
    assert_eq!(p.stage(), BootSeg::Terrain);
    assert_eq!(p.stage().title(), "Loading terrain…");
    p.apply(BootEvent::Finish(BootSeg::Terrain));
    assert_eq!(p.stage(), BootSeg::Satellite);
    assert_eq!(p.stage().title(), "Loading satellite…");
    p.apply(BootEvent::Finish(BootSeg::Satellite));
    assert_eq!(p.stage(), BootSeg::World);
    assert_eq!(p.stage().title(), "Loading world objects…");
}

#[test]
fn the_caption_reports_bytes_for_bytes_and_files_for_files() {
    let mut p = BootProgress::new();
    assert_eq!(
        p.caption(),
        "0%",
        "a stage that has not read its own budget shows the percentage alone — not a \
         denominator nobody measured"
    );
    p.apply(BootEvent::Budget(BootSeg::Mission, 2_032));
    p.apply(BootEvent::Done(BootSeg::Mission, 2_032));
    assert_eq!(p.caption(), "0% · 3 KB / 3 KB");
    p.apply(BootEvent::Finish(BootSeg::Mission));
    p.apply(BootEvent::Budget(BootSeg::Terrain, PLANNED_TERRAIN_BYTES));
    p.apply(BootEvent::Done(BootSeg::Terrain, 26_700_000));
    assert_eq!(p.caption(), "18% · 26.7 MB / 71.9 MB");
    p.apply(BootEvent::Finish(BootSeg::Terrain));
    p.apply(BootEvent::Finish(BootSeg::Satellite));
    p.apply(BootEvent::Files(BootSeg::World, 834));
    p.apply(BootEvent::Done(BootSeg::World, 214));
    assert!(
        p.caption().ends_with("214 / 834 files"),
        "the world counts completed fetches, so it says files — implying a byte budget nothing \
         published is the same defect one size down. Got {}",
        p.caption()
    );
    assert_eq!(fmt_files_pair(214, 834), "214 / 834 files");
}

// ── the wasm side must actually route through the code proved above ──────────────────────

/// Source pin on `world_assets/satellite.rs`. It is `#[cfg(target_arch = "wasm32")]` (via
/// `mod world_assets` in `main.rs`), so nothing in it can be called from here — but it can be
/// held to *shape*. `live_code` blanks comments and string literals first, so a needle can only
/// be satisfied by code that ships.
#[test]
fn the_satellite_fetch_is_bounded_concurrent_ordered_and_fails_fast() {
    use crate::editor::arsenal::class_r_scrub::{live_code, only_body};
    let src = live_code(include_str!("../world_assets/satellite.rs"));
    let body = only_body(&src, "async fn fetch_tiles(");

    assert!(
        body.contains("buffer_unordered(SAT_FETCH_CONCURRENCY)"),
        "the fetch must be concurrent and BOUNDED by the named constant — an unbounded \
         `FuturesUnordered` over 37 requests starves the world-chunk loader on the same origin"
    );
    assert!(
        body.contains("split_range(t.offset, t.length, SAT_CHUNK_BYTES)"),
        "requests must come from the span planner proved above, not an ad-hoc loop"
    );
    assert!(
        body.contains("Ordered::new(p.len())") && body.contains(".put(pi, body.bytes)"),
        "completions must be written to their own index; a `push` here is the scrambled \
         texture this module exists to prevent"
    );
    assert!(
        body.contains("slot.finish()?"),
        "reassembly must refuse a partially filled run"
    );
    assert!(
        body.contains("let body = got?;")
            && body.contains("body.bytes.len() as u64 != want")
            && body.contains("body.total != file_size"),
        "fail-fast and the length check must both survive: the pre-T-627 loop returned None \
         on the first failure and on a short body, and a partial texture must still never \
         reach commit_mip"
    );
    assert!(
        !body.contains("out.push(body.bytes)"),
        "bodies must never be pushed in completion order"
    );

    // And the full load must not be back to swallowing the whole bundle to read its index.
    let full = only_body(&src, "async fn load_unified_full(");
    assert!(
        full.contains("fetch_index_head(url, true)") && full.contains("fetch_tiles(url,"),
        "the full mip chain must come from the index + per-tile Range fetches"
    );
    assert!(
        !full.contains("fetch_bytes(url)"),
        "a whole-file GET has no byte progress to report and drags down 110.6 MB of level 0 \
         that an 8192-limit GPU cannot use"
    );
}

/// Source pin on the overlay itself. Raw `include_str!` (not `live_code`) because this file's
/// first `#[cfg(test)]` is a `clear_for_test` helper near the top, which would cut the view
/// out; the needles are therefore assembled at runtime so this test's own text cannot satisfy
/// them.
#[test]
fn the_overlay_draws_one_measured_bar_and_no_sweep_anywhere() {
    let src = include_str!("../mission_editor.rs");
    let from_progress = format!("{}{}", "p.", "percent()");
    assert!(
        src.contains(&from_progress),
        "the overlay's width must come from the accumulator, not from a per-stage step"
    );
    let inline_width = format!("{}{}", "width:{", "pct:.1}%");
    assert!(
        src.contains(&inline_width),
        "the fill's width must be the real percentage"
    );
    let sweep = format!("{}{}", "animate-mc-", "load-bar");
    assert!(
        !src.contains(sweep.as_str()),
        "the Mission Creator boot overlay must contain NO indeterminate sweep. A sweep looks \
         identical at 1%, at 99% and while stalled — 'you might as well have a black screen'. \
         (The class itself still ships for other surfaces; this file may not use it.)"
    );
    assert!(
        SAT_FETCH_CONCURRENCY >= 4 && SAT_FETCH_CONCURRENCY <= 6,
        "browsers cap ~6 connections per origin and the chunk loader shares them — outside \
         4..=6 this is either not parallel or actively starving the rest of the boot"
    );
    assert!(
        SAT_CHUNK_BYTES > 0 && FILE_BYTES / SAT_CHUNK_BYTES >= 20,
        "the chunk size must give the bar at least ~20 steps across the bundle, or it is a \
         per-tile bar again: four ~25 MB tiles fetched four-up would sit at 0% then snap"
    );
}

/// Source pin on the terrain segment. The DEM is the single biggest thing the boot fetches and
/// the pre-T-628 path pulled it with a plain `fetch_bytes`, which yields one 71.9 MB step at the
/// very end — indistinguishable from a stall for the whole download.
#[test]
fn the_terrain_dem_is_streamed_against_its_content_length() {
    use crate::editor::arsenal::class_r_scrub::{live_code, only_body};
    let src = live_code(include_str!("../world_assets/mod.rs"));
    let body = only_body(&src, "async fn load_dem_and_hillshade(");
    assert!(
        body.contains("fetch_bytes_streamed(") && body.contains("BootSeg::Terrain"),
        "the DEM must be fetched through the measured, streamed helper — a whole-body GET has \
         nothing to report until it is already finished"
    );
    assert!(
        !body.contains("fetch_bytes(&format!"),
        "the unmeasured whole-body GET must not come back"
    );

    let fetch = live_code(include_str!("../world_assets/fetch.rs"));
    let streamed = only_body(&fetch, "pub async fn fetch_bytes_streamed(");
    // `live_code` blanks string literals, so the header NAME cannot be the needle — the shape
    // that survives is "a header off this response, parsed as a number, becomes the budget",
    // which is the property that matters anyway.
    assert!(
        streamed.contains(".headers()")
            && streamed.contains("parse::<u64>()")
            && streamed.contains("BootEvent::Budget(seg, budget)"),
        "the budget must be a header read off this response, not a constant and not a guess"
    );
    assert!(
        streamed.contains("reader.read()") && streamed.contains("BootEvent::Done"),
        "progress must be the bytes that came out of the body reader — nothing else in this \
         function is allowed to be the numerator"
    );
    let elapsed = ["Date::now", "set_timeout", "performance"];
    for needle in elapsed {
        assert!(
            !streamed.contains(needle),
            "`{needle}` in the streaming fetch would be a bar moving on a clock: the one \
             defect this whole slice is aimed at"
        );
    }
}

/// Source pin on the world segment's two dynamic budgets. Both must be declared **before** the
/// fetches they cover: a batch that announces itself on completion is a bar that reaches 100%
/// and then finds more work, which reads to the operator as a lie either way round.
#[test]
fn every_world_batch_declares_its_files_before_it_fetches_them() {
    use crate::editor::arsenal::class_r_scrub::{live_code, only_body};
    let world = live_code(include_str!("../world_assets/world_host.rs"));
    let queue = only_body(&world, "async fn fetch_and_queue(");
    let declare = queue
        .find("BootEvent::Files")
        .expect("the chunk batch must declare its own size");
    let fetch = queue
        .find("fetch_bytes(&url)")
        .expect("the chunk batch must still fetch");
    assert!(
        declare < fetch,
        "the chunk count must be declared before the first request goes out, not after"
    );
    assert!(
        queue.contains("ids.len() as u64"),
        "the declared count must be the residency's own missing set — the exact list it is \
         about to request, not an estimate of it"
    );

    let boot = live_code(include_str!("../world_assets/mod.rs"));
    let bootstrap = only_body(&boot, "pub async fn bootstrap(");
    let plan = bootstrap
        .find("planned_density_bins()")
        .expect("the 625 density bins must be declared up front");
    let init = bootstrap
        .find("world.init(")
        .expect("bootstrap must still init the world host");
    assert!(
        plan < init,
        "the density bins are a known constant (25×25) and must join the budget before the \
         world starts filling it — declaring them after the chunks land would park the bar at \
         100% and then discover 625 more files"
    );

    // The forest host may only count a bin it actually landed; counting attempts would let a
    // retried bin advance a unit that was already declared and spent.
    let forest = live_code(include_str!("../world_assets/forest_mass.rs"));
    let upload = only_body(&forest, "async fn boot_upload(");
    let done_at = upload
        .find("BootEvent::Done")
        .expect("a landed bin must be counted");
    let ok_at = upload
        .rfind("if ok {")
        .expect("the bin must only be counted when it decoded");
    assert!(
        ok_at < done_at,
        "a density bin counts on success only — a retry loop that counts attempts finishes 625 \
         declared bins at 640 done, i.e. a full segment over a holed canopy"
    );
}

/// Source pin on the hand-over. Every segment must be closed by the code that owns it, or a
/// dead network leaves the bar short of 100% with the overlay still up — and the overlay may
/// not come down until it is full.
#[test]
fn every_segment_is_closed_and_the_overlay_waits_for_a_full_bar() {
    use crate::editor::arsenal::class_r_scrub::{live_code, only_body};
    let boot = live_code(include_str!("../world_assets/mod.rs"));
    let bootstrap = only_body(&boot, "pub async fn bootstrap(");
    for seg in ["BootSeg::Terrain", "BootSeg::Satellite", "BootSeg::World"] {
        assert!(
            bootstrap.contains(&format!("BootEvent::Finish({seg})")),
            "`bootstrap` owns {seg} and must close it on every path, including failure"
        );
    }
    // "On every path" has teeth: both map futures reach their loader through a `?` on the
    // manifest, and a `?` returns from whichever `async` block it sits in. The `Finish` must
    // therefore live in an OUTER block, or a failed manifest fetch skips it — and the bar comes
    // up short precisely on the boot that went wrong. Two `async {` before the close is that
    // nesting.
    for (open, seg) in [
        ("let dem_fut = async {", "BootSeg::Terrain"),
        ("let sat_fut = async {", "BootSeg::Satellite"),
    ] {
        let at = bootstrap
            .find(open)
            .unwrap_or_else(|| panic!("`{open}` must still exist"));
        let close = bootstrap[at..]
            .find(&format!("BootEvent::Finish({seg})"))
            .unwrap_or_else(|| panic!("{seg} must be closed inside its own future"));
        let region = &bootstrap[at..at + close];
        assert!(
            region.matches("async {").count() >= 2,
            "{seg}'s `Finish` must sit outside the block holding the `?` — one `async {{` \
             between them means a failed manifest fetch returns past it"
        );
    }
    let src = include_str!("../mission_editor.rs");
    let mission_finish = format!(
        "{}{}",
        "BootEvent::Finish(\n", "                        boot_progress::BootSeg::Mission,"
    );
    assert!(
        src.contains(&mission_finish)
            || src.contains("BootEvent::Finish(boot_progress::BootSeg::Mission)"),
        "the hydrate task owns the mission segment and must close it once the hydrate returns"
    );
    let handover = format!("{}{}", "hand_", "over(boot)");
    assert!(
        src.contains(&handover),
        "both rendezvous points must go through the hand-over, so the overlay is never removed \
         in the same render as the last measurement"
    );
    assert!(
        BOOT_HANDOVER_MS >= 200,
        "the hold must be at least the 200 ms `.mc-load-fill` ease, or the fill is still \
         travelling when the overlay is pulled and 100% is never actually drawn"
    );
}

/// Source pin on the mission document. It is the one measured fetch that is **not** on the
/// map-asset host, and the one that must not grow a second copy of the auth contract.
#[test]
fn the_mission_document_is_measured_and_still_defers_to_the_single_flight_client() {
    use crate::editor::arsenal::class_r_scrub::{live_code, only_body};
    let src = live_code(include_str!("../state/hydrate.rs"));
    let body = only_body(&src, "async fn get_mission_measured(");
    // `live_code` blanks string literals — see the terrain pin for why the shape, not the
    // header name, is the needle.
    assert!(
        body.contains(".headers()")
            && body.contains("parse::<u64>()")
            && body.contains("BootEvent::Budget(BootSeg::Mission, budget)"),
        "the mission segment's budget must be a header read off this response"
    );
    assert!(
        body.contains("reader.read()") && body.contains("BootEvent::Done"),
        "its progress must be the bytes off the body reader"
    );
    assert!(
        body.contains("crate::core::client::api_get::<MissionDetail>(auth, path)"),
        "anything that is not a 2xx — the 401 above all — must fall through to `api_get`, \
         which owns the single-flight refresh. A second refresh path would double-spend the \
         rotating token, and that is a data-safety bug, not a loading-bar bug"
    );
    assert!(
        !body.contains("auth/refresh"),
        "this function must never mint or spend a refresh token itself"
    );
    let hydrate = only_body(&src, "pub async fn hydrate_from_server(");
    assert!(
        hydrate.contains("get_mission_measured(auth, &path")
            && !hydrate.contains("client::api_get::<MissionDetail>"),
        "the hydrate's own GET must route through the measured wrapper, not around it"
    );
}
