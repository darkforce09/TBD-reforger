use super::boot_progress::{
    BootProgress, BootSegView, PLANNED_SATELLITE_BYTES, PLANNED_TERRAIN_BYTES, PLANNED_WORLD_BYTES,
    fmt_bytes_pair, fmt_files_pair, percent,
};
use map_streaming_model::boot_progress::{
    BootEvent, BootSeg, Ordered, SAT_CHUNK_BYTES, STREAM_REPORT_BYTES, split_range,
};

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
    const {
        assert!(
            STREAM_REPORT_BYTES > 0 && PLANNED_TERRAIN_BYTES / STREAM_REPORT_BYTES >= 100,
            "the stream must report at least ~100 times across the DEM, or the terrain segment is \
         a per-file bar again: 0% for the whole download, then a snap"
        )
    };
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
