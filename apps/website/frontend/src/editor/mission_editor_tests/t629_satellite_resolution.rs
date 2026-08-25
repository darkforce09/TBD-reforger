use super::tbd_sat_pure::{
    parse_tbd_sat_index_strict, pick_base_level, pick_base_level_for_limit, TbdSatIndex,
};

/// The live `packages/map-assets/everon/satellite/everon-sat.tbd-sat` index, read off the
/// bundle on 2026-08-01: 14 levels from 12800² down to 1×1, `sourceMeta` (which the loader
/// does not deserialize) dropped. Offsets and lengths are the real ones, so the strict
/// validator below runs the same level-numbering / halving / coverage / terminator rules it
/// runs in the browser.
const EVERON_INDEX_JSON: &str = concat!(
    r#"{"formatVersion":1,"terrainId":"everon","worldBounds":[0,0,12800,12800],"#,
    r#""baseWidthPx":12800,"baseHeightPx":12800,"mipCount":14,"mips":["#,
    r#"{"level":0,"width":12800,"height":12800,"tiles":["#,
    r#"{"x":0,"y":0,"width":6400,"height":6400,"offset":2644,"length":28326346},"#,
    r#"{"x":6400,"y":0,"width":6400,"height":6400,"offset":28328990,"length":21632714},"#,
    r#"{"x":0,"y":6400,"width":6400,"height":6400,"offset":49961704,"length":27555806},"#,
    r#"{"x":6400,"y":6400,"width":6400,"height":6400,"offset":77517510,"length":33042794}]},"#,
    r#"{"level":1,"width":6400,"height":6400,"tiles":[{"x":0,"y":0,"width":6400,"height":6400,"offset":110560304,"length":30866380}]},"#,
    r#"{"level":2,"width":3200,"height":3200,"tiles":[{"x":0,"y":0,"width":3200,"height":3200,"offset":141426684,"length":8271166}]},"#,
    r#"{"level":3,"width":1600,"height":1600,"tiles":[{"x":0,"y":0,"width":1600,"height":1600,"offset":149697850,"length":2218572}]},"#,
    r#"{"level":4,"width":800,"height":800,"tiles":[{"x":0,"y":0,"width":800,"height":800,"offset":151916422,"length":583330}]},"#,
    r#"{"level":5,"width":400,"height":400,"tiles":[{"x":0,"y":0,"width":400,"height":400,"offset":152499752,"length":153506}]},"#,
    r#"{"level":6,"width":200,"height":200,"tiles":[{"x":0,"y":0,"width":200,"height":200,"offset":152653258,"length":42470}]},"#,
    r#"{"level":7,"width":100,"height":100,"tiles":[{"x":0,"y":0,"width":100,"height":100,"offset":152695728,"length":12086}]},"#,
    r#"{"level":8,"width":50,"height":50,"tiles":[{"x":0,"y":0,"width":50,"height":50,"offset":152707814,"length":3584}]},"#,
    r#"{"level":9,"width":25,"height":25,"tiles":[{"x":0,"y":0,"width":25,"height":25,"offset":152711398,"length":1138}]},"#,
    r#"{"level":10,"width":12,"height":12,"tiles":[{"x":0,"y":0,"width":12,"height":12,"offset":152712536,"length":328}]},"#,
    r#"{"level":11,"width":6,"height":6,"tiles":[{"x":0,"y":0,"width":6,"height":6,"offset":152712864,"length":126}]},"#,
    r#"{"level":12,"width":3,"height":3,"tiles":[{"x":0,"y":0,"width":3,"height":3,"offset":152712990,"length":86}]},"#,
    r#"{"level":13,"width":1,"height":1,"tiles":[{"x":0,"y":0,"width":1,"height":1,"offset":152713076,"length":38}]}]}"#,
);
const FILE_BYTES: u64 = 152_713_114;

/// Rebuild the TBDS container header (`"TBDS"`, formatVersion 1, jsonLen) in front of the
/// index so the real parser runs, not `serde_json` on its own.
fn everon_index() -> TbdSatIndex {
    let json = EVERON_INDEX_JSON.as_bytes();
    assert!(
        12 + json.len() as u64 <= 2_644,
        "the header + this index must still end at or before the real bundle's first tile \
         offset (2,644), or the strict validator will reject real offsets as out of range"
    );
    let mut buf = Vec::with_capacity(12 + json.len());
    buf.extend_from_slice(&0x5344_4254_u32.to_le_bytes()); // "TBDS"
    buf.extend_from_slice(&1_u32.to_le_bytes());
    buf.extend_from_slice(&(json.len() as u32).to_le_bytes());
    buf.extend_from_slice(json);
    parse_tbd_sat_index_strict(&buf, FILE_BYTES)
        .unwrap_or_else(|e| panic!("the committed everon index must parse strictly: {e}"))
}

// ── the level the GPU limit buys ──────────────────────────────────────────────────────────

#[test]
fn the_everon_ladder_is_the_real_one() {
    let idx = everon_index();
    assert_eq!(idx.mip_count, 14);
    assert_eq!((idx.base_width_px, idx.base_height_px), (12_800, 12_800));
    assert_eq!((idx.mips[0].width, idx.mips[0].height), (12_800, 12_800));
    assert_eq!((idx.mips[1].width, idx.mips[1].height), (6_400, 6_400));
    assert_eq!(idx.mips[0].tiles.len(), 4, "level 0 is four 6400² tiles");
}

#[test]
fn eight_k_costs_exactly_half_the_resolution_and_sixteen_k_does_not() {
    let idx = everon_index();
    assert_eq!(
        pick_base_level(&idx, 8_192),
        1,
        "12800 does not fit 8192, so the base becomes level 1 — 6400², literally half the \
         island's resolution. This is the operator-visible cost of the limit, and it is the \
         number the removed `unwrap_or(8192)` used to produce without measuring anything"
    );
    assert_eq!(
        pick_base_level(&idx, 16_384),
        0,
        "a 16384 GPU must get the 12800² source level"
    );
    assert_eq!(
        pick_base_level(&idx, 12_800),
        0,
        "the comparison is `<=`: a limit exactly equal to the base edge still fits"
    );
    assert_eq!(
        pick_base_level(&idx, 12_799),
        1,
        "one pixel short must fall to level 1, not silently clamp"
    );
    assert_eq!(
        pick_base_level(&idx, 4_096),
        2,
        "a 4096 GPU walks two levels down, not one"
    );
}

#[test]
fn an_unknown_limit_yields_no_level_at_all() {
    let idx = everon_index();
    assert_eq!(
        pick_base_level_for_limit(&idx, None),
        None,
        "this is the whole point of T-629: when the GPU limit could not be read there is no \
         level to pick. The previous code answered this case with 8192 — a real, plausible, \
         wrong number that committed half resolution and told nobody"
    );
    for limit in [4_096_u32, 8_192, 12_800, 16_384, 32_768] {
        assert_eq!(
            pick_base_level_for_limit(&idx, Some(limit)),
            Some(pick_base_level(&idx, limit)),
            "a KNOWN limit must decide exactly as it always did"
        );
    }
}

// ── the wasm side must actually route through the code proved above ───────────────────────

#[test]
fn no_call_site_may_guess_a_texture_limit() {
    use crate::editor::arsenal::class_r_scrub::{live_code, only_body};
    let src = live_code(include_str!("../world_assets/satellite.rs"));

    assert!(
        !src.contains("unwrap_or(8192)"),
        "both copies of the guess must be gone. A default texture limit is not conservative, \
         it is unfalsifiable: it picks a real mip, uploads it, and leaves the operator looking \
         at half an island with a 100% loading bar above it"
    );
    let limit_fn = only_body(&src, "fn texture_limit(");
    assert!(
        limit_fn.contains("e.max_texture_dimension_2d()")
            && limit_fn.contains("e.adapter_max_texture_dimension_2d()"),
        "the limit reader must report the device limit AND the adapter ceiling it was \
         requested against"
    );
    assert_eq!(
        src.matches("max_texture_dimension_2d()").count(),
        2,
        "the GPU limit must be read in exactly ONE place (both reads inside `texture_limit`). \
         A second reader is a second opportunity to re-invent the default"
    );

    let full = only_body(&src, "async fn load_unified_full(");
    assert!(
        full.contains("pick_base_level_for_limit(&index, limit.map(|l| l.device))"),
        "the base level must be chosen from the Option-typed limit, so a missing engine \
         cannot be spelled the same way as a measured one"
    );
    assert!(
        full.contains("logging::error!") && full.contains("return false;"),
        "a missing engine must abort the load loudly, not substitute a number"
    );
    assert!(
        full.contains("report_chosen_level(&index, base, limit)"),
        "a downscaled basemap must announce itself — one that says nothing is \
         indistinguishable from a correct one"
    );
    let commit_at = full
        .find("tex_layer_commit")
        .expect("the full load must commit the basemap");
    assert!(
        full[commit_at..].contains("logging::log!"),
        "the load must report what LANDED, after the commit. A line printed before the upload \
         is a claim about the future, and this whole ticket exists because the map on screen \
         disagreed with what the boot implied had happened"
    );

    let map = only_body(&src, "pub async fn load_map_basemap(");
    assert!(
        map.contains("texture_limit(engine)") && !map.contains("unwrap_or(8192)"),
        "the cartographic pyramid picks a stitched zoom from the same limit and must obey the \
         same rule"
    );
}

#[test]
fn a_downscaled_basemap_warns_and_a_stuck_placeholder_warns() {
    use crate::editor::arsenal::class_r_scrub::{live_code, only_body};
    let src = live_code(include_str!("../world_assets/satellite.rs"));

    let report = only_body(&src, "fn report_chosen_level(");
    assert!(
        report.contains("logging::warn!"),
        "level > 0 means the operator is looking at a downscaled island; that must reach the \
         console at warn, not be inferred from how soft the map looks"
    );
    assert!(
        report.contains("limit.device") && report.contains("limit.adapter"),
        "the warning must name BOTH numbers — 'the GPU cannot do better' and 'the device \
         request lost resolution the GPU offered' are different bugs with the same symptom"
    );

    let fetch = only_body(&src, "async fn fetch_tiles(");
    assert!(
        fetch.contains("fetch_range_resilient(url, start, end)"),
        "T-629 root cause: at base level 0 everon's plan is 49 Range requests, `fetch_tiles` \
         is fail-fast, and a single dropped request therefore discarded all 152,710,470 B and \
         left the <=1024 px preview up. Each span must get bounded retries"
    );
    let retry = only_body(&src, "async fn fetch_range_resilient(");
    assert!(
        retry.contains("for attempt in 1..=RANGE_ATTEMPTS") && retry.contains("return Some("),
        "the retry must be BOUNDED — an unbounded loop turns a dead origin into a boot that \
         never finishes, which is worse than the blurry map it replaces"
    );
    assert!(
        retry.contains("RangeOutcome::RateLimited") && retry.contains("sleep_ms(wait).await"),
        "a 429 must be recognised AND waited out. The API's global limiter is 20/s burst 40 \
         and the base-level-0 plan is 49 spans, so retrying a throttled span immediately just \
         spends the remaining attempts inside the same exhausted bucket"
    );
    assert!(
        retry.contains("logging::warn!"),
        "a retried span must say so; silent recovery hides a degrading origin until it fails \
         outright"
    );
    assert!(
        src.contains("const RANGE_ATTEMPTS: usize = 5;")
            && src.contains("const RANGE_BACKOFF_MS: [i32; 4]"),
        "one fewer wait than attempt — the last attempt has nothing to wait for. (Pinned as \
         text because the loader is wasm-only and these consts do not exist on this target.)"
    );

    let load = only_body(&src, "pub async fn load_satellite(");
    assert!(
        !load.contains("let _ = load_unified_full("),
        "the full load's failure must not be discarded. That discard IS the reported symptom: \
         when it returns false the <=1024 px preview stays on screen as if it were the map"
    );
    assert!(
        load.contains("if !load_unified_full(") && load.contains("logging::warn!"),
        "a failed full load must say that the placeholder is what is being displayed"
    );
}
