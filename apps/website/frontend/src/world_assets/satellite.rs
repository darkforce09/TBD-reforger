//! T-166 — unified satellite host: TBDS Range preview + optional full mip upload.
//!
//! T-627: **no** path GETs the bundle body any more, not just `?sat=preview`. The full load reads
//! the index by Range and then fetches only the tiles it will upload, `SAT_FETCH_CONCURRENCY`
//! requests at a time — so the old "CI never downloads 146–206 MB" guarantee now holds for every
//! caller, and on a GPU whose `maxTextureDimension2D` is below the 12800 px base level it is the
//! difference between 152.7 MB and 42.2 MB of everon.

use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

use crate::mission_editor::boot_progress::{
    split_range, BootEvent, BootSeg, Ordered, SAT_CHUNK_BYTES, SAT_FETCH_CONCURRENCY,
};
use crate::select_tool::EngineHandle;

use super::bridge::{publish, BridgeHandle};
use super::fetch::{fetch_bytes, fetch_range_outcome, RangeBody, RangeOutcome};
use super::tbd_sat::{
    parse_tbd_sat_index_only, parse_tbd_sat_index_strict, pick_base_level_for_limit,
    pick_preview_level, TbdSatIndex, TbdSatMip, TbdSatTile,
};

const ROLE_BASEMAP: u32 = 0;
const MODE_UNIFIED: u32 = 0;
const MODE_SINGLE: u32 = 2;
const PREVIEW_MAX_EDGE: u32 = 1024;

/// T-629 — what the GPU will actually accept as one texture's long edge, read off the live engine.
///
/// Two numbers, because a half-resolution basemap has two different explanations and the operator
/// deserves to be told which one he is looking at:
///
/// * `device` is the limit the renderer may allocate against, and the only one
///   [`pick_base_level_for_limit`] may be given.
/// * `adapter` is the ceiling that limit was *requested* against
///   (`base_limits.using_resolution(adapter.limits())` in `RenderEngine::create`). When the two
///   agree, the hardware genuinely cannot hold the full-resolution level as a single texture; when
///   `device` is the smaller, the device request lost resolution the GPU was offering.
#[derive(Clone, Copy)]
struct TextureLimit {
    device: u32,
    adapter: u32,
}

/// T-629 — read the limit, or report that there is nothing to read it from.
///
/// This replaced `engine.borrow().as_ref().map(|e| e.max_texture_dimension_2d()).unwrap_or(8192)`,
/// which appeared at both call sites. There is no fallback any more, loud or otherwise: a guessed
/// limit picks a real mip and commits it, so the guess is indistinguishable downstream from a
/// measurement — which is precisely the failure this codebase keeps producing. `None` here makes
/// the absence unignorable at the call site instead.
fn texture_limit(engine: &EngineHandle) -> Option<TextureLimit> {
    let guard = engine.borrow();
    let e = guard.as_ref()?;
    Some(TextureLimit {
        device: e.max_texture_dimension_2d(),
        adapter: e.adapter_max_texture_dimension_2d(),
    })
}

/// T-629 — warn when this GPU's texture limit forces a downscaled basemap.
///
/// Level 0 is the source resolution and says nothing here; the commit at the end of
/// [`load_unified_full`] reports what actually landed, which is the only thing worth claiming
/// before the bytes are on the GPU. Anything above level 0 means the operator is looking at a
/// downscaled island, which was previously indistinguishable from a correctly loaded one — the bar
/// reached 100%, a texture appeared, and the only symptom was that everon looked soft.
fn report_chosen_level(index: &TbdSatIndex, base: usize, limit: TextureLimit) {
    let Some(mip) = index.mips.get(base) else {
        return;
    };
    if base == 0 {
        return;
    }
    let cause = if limit.device >= limit.adapter {
        "this GPU cannot hold the full-resolution basemap as a single texture"
    } else {
        "the device was granted less than the adapter offered — this is a renderer bug, not a GPU \
         limit"
    };
    leptos::logging::warn!(
        "satellite: DOWNSCALED basemap — showing level {} ({}x{}) instead of level 0 ({}x{}). GPU \
         maxTextureDimension2D = {} (adapter {}); {}.",
        base,
        mip.width,
        mip.height,
        index.base_width_px,
        index.base_height_px,
        limit.device,
        limit.adapter,
        cause
    );
}

/// `?sat=preview` — Range-only path; never full-bundle GET (CI / gate harness).
pub fn sat_preview_only() -> bool {
    web_sys::window()
        .and_then(|w| w.location().search().ok())
        .map(|s| s.contains("sat=preview"))
        .unwrap_or(false)
}

enum Decoded {
    Bitmap(web_sys::ImageBitmap),
    Rgba { w: u32, h: u32, rgba: Vec<u8> },
}

async fn decode_webp(bytes: &[u8], webgl2: bool) -> Option<Decoded> {
    let win = web_sys::window()?;
    let u8 = js_sys::Uint8Array::new_with_length(bytes.len() as u32);
    u8.copy_from(bytes);
    let parts = js_sys::Array::new();
    parts.push(&u8);
    let props = web_sys::BlobPropertyBag::new();
    props.set_type("image/webp");
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &props).ok()?;
    let opts = web_sys::ImageBitmapOptions::new();
    opts.set_color_space_conversion(web_sys::ColorSpaceConversion::None);
    let p = win
        .create_image_bitmap_with_blob_and_image_bitmap_options(&blob, &opts)
        .ok()?;
    let bmp: web_sys::ImageBitmap = JsFuture::from(p).await.ok()?.dyn_into().ok()?;
    if !webgl2 {
        return Some(Decoded::Bitmap(bmp));
    }
    let w = bmp.width();
    let h = bmp.height();
    let canvas = web_sys::OffscreenCanvas::new(w, h).ok()?;
    let ctx = canvas
        .get_context("2d")
        .ok()
        .flatten()?
        .dyn_into::<web_sys::OffscreenCanvasRenderingContext2d>()
        .ok()?;
    ctx.draw_image_with_image_bitmap(&bmp, 0.0, 0.0).ok()?;
    bmp.close();
    let image_data = ctx
        .get_image_data(0.0, 0.0, f64::from(w), f64::from(h))
        .ok()?;
    let data = image_data.data().0;
    Some(Decoded::Rgba { w, h, rgba: data })
}

fn upload_decoded(
    engine: &mut map_engine_render::RenderEngine,
    role: u32,
    mip: u32,
    x: u32,
    y: u32,
    decoded: Decoded,
) -> bool {
    match decoded {
        Decoded::Bitmap(bmp) => {
            let w = bmp.width();
            let h = bmp.height();
            engine
                .tex_layer_write_bitmap(role, mip, x, y, w, h, bmp)
                .is_ok()
        }
        Decoded::Rgba { w, h, rgba } => engine
            .tex_layer_write_rgba(role, mip, x, y, w, h, &rgba)
            .is_ok(),
    }
}

/// `strict` runs the full mip-chain validation (`parse_tbd_sat_index_strict`) instead of the loose
/// block-range one — see that function. The preview path stays loose, as it always was; the full
/// load is strict, as it always was when it parsed the whole downloaded file.
async fn fetch_index_head(url: &str, strict: bool) -> Option<(TbdSatIndex, u64)> {
    // T-629 — the index is two Range requests and everything downstream depends on them, so they
    // get the same retry ladder as the tiles: losing the header to a 429 loses the whole basemap.
    let head = fetch_range_resilient(url, 0, 11).await?;
    if head.bytes.len() < 12 {
        return None;
    }
    let version = u32::from_le_bytes(head.bytes[4..8].try_into().ok()?);
    let json_len = u32::from_le_bytes(head.bytes[8..12].try_into().ok()?);
    if version != 1 || json_len == 0 || json_len > 16 * 1024 * 1024 {
        return None;
    }
    let full = fetch_range_resilient(url, 0, 11 + u64::from(json_len)).await?;
    let index = if strict {
        parse_tbd_sat_index_strict(&full.bytes, full.total).ok()?
    } else {
        parse_tbd_sat_index_only(&full.bytes, full.total).ok()?
    };
    Some((index, full.total))
}

/// T-627 — fetch every tile in `tiles` over **concurrent** HTTP Range requests, returning the
/// bodies in `tiles` order.
///
/// wasm is single-threaded, so "concurrent" here is `buffer_unordered` over a stream of futures:
/// `SAT_FETCH_CONCURRENCY` requests are in flight at once and the browser pipelines them. What that
/// buys, and what it costs:
///
/// * **Order.** Completions arrive in network order, and callers consume the result positionally
///   (`commit_mip` pairs element *n* with `mip.tiles[n]`, and a tile's chunks concatenate at their
///   own offsets). Every request therefore carries `(tile, part)` indices and is written into
///   [`Ordered`] at exactly those, never pushed. Getting this wrong is a scrambled satellite
///   texture that reads as a rendering bug, so the reassembly is pinned by a host test.
/// * **Fail-fast.** The first failed request, short body, or `content-range` total that disagrees
///   with the index's file size returns `None` immediately; dropping the stream cancels whatever is
///   still in flight. A partial texture never reaches `commit_mip`.
/// * **Progress.** `on_bytes` is called with the size of each completed request — completed work
///   only, never a prediction.
/// T-629 — attempts allowed per Range span before the whole load is abandoned.
///
/// **This is the T-627 regression.** [`fetch_tiles`] is deliberately fail-fast: the first failed
/// request cancels the run so a partial texture can never reach `commit_mip`. That is right. What
/// was not right is what counted as a failure. Before T-627 the satellite was **one** GET of the
/// whole bundle; T-627 turned it into per-tile Range requests — **49** of them for everon's base
/// level 0 — and T-625/T-626, landing immediately before, keyed the API's rate limiter per client.
/// That limiter is `20/s` **burst 40** and `/map-assets` is not exempt from it, so a base-level-0
/// boot asks for 49 spans out of a 40-token bucket that the DEM, the world chunks and the SPA's own
/// assets have already been drawing on. The overflow comes back `429`, `fetch_range` mapped every
/// non-206 to `None`, one `None` killed all 152,710,470 B, and `load_satellite` discarded the bool —
/// so the ≤1024 px preview stayed on screen as "the satellite map".
///
/// Measured on the T-629 rig (headless Chrome against this repo's `trunk serve`, level 0 chosen
/// from a real 16384 limit): three consecutive boots failed on three *different* spans, and
/// replaying the exact 49-span plan with `curl` at concurrency 4 returned **48× `429`, 1× `206`**.
/// The base-level-1 plan is 21 spans and survived — which is precisely what a 40-token bucket
/// predicts.
///
/// Five attempts with [`RANGE_BACKOFF_MS`], not "retry until it works": a span that fails five
/// times is a real failure and must still fail the load loudly rather than hang the boot behind an
/// unbounded loop.
const RANGE_ATTEMPTS: usize = 5;

/// T-629 — backoff before each retry, in ms. The limiter refills at 20 tokens/s (one per 50 ms) and
/// answers `Retry-After: 1`, so the ladder starts just above one token and ends above a full second;
/// a span that is still throttled after 2.15 s of waiting is not a burst, it is an outage.
const RANGE_BACKOFF_MS: [i32; 4] = [100, 250, 600, 1_200];

/// Await `ms` of wall clock. Resolves immediately if there is no window to schedule on, so this can
/// never wedge the boot.
async fn sleep_ms(ms: i32) {
    let p = js_sys::Promise::new(&mut |resolve, _reject| {
        let scheduled = web_sys::window().and_then(|w| {
            w.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms)
                .ok()
        });
        if scheduled.is_none() {
            let _ = resolve.call0(&wasm_bindgen::JsValue::NULL);
        }
    });
    let _ = JsFuture::from(p).await;
}

/// T-629 — one Range span, retried up to [`RANGE_ATTEMPTS`] times with backoff.
///
/// Every retry is announced. A throttled origin has to show up as noise in the console rather than
/// as a mysteriously soft island — that silence is the whole bug.
async fn fetch_range_resilient(url: &str, start: u64, end: u64) -> Option<RangeBody> {
    for attempt in 1..=RANGE_ATTEMPTS {
        let throttled = match fetch_range_outcome(url, start, end).await {
            RangeOutcome::Body(body) => {
                if attempt > 1 {
                    leptos::logging::warn!(
                        "satellite: Range bytes={start}-{end} succeeded on attempt {attempt}"
                    );
                }
                return Some(body);
            }
            RangeOutcome::RateLimited { retry_after_s } => {
                leptos::logging::warn!(
                    "satellite: Range bytes={start}-{end} throttled (429, Retry-After {:?}) \
                     — attempt {attempt}/{RANGE_ATTEMPTS}",
                    retry_after_s
                );
                true
            }
            RangeOutcome::Failed { status } => {
                leptos::logging::warn!(
                    "satellite: Range bytes={start}-{end} failed (status {status}) — attempt \
                     {attempt}/{RANGE_ATTEMPTS}"
                );
                false
            }
        };
        // The last attempt has nothing to wait for.
        let Some(&base_ms) = RANGE_BACKOFF_MS.get(attempt - 1) else {
            break;
        };
        // A 429 is the server telling us when to come back; anything else is just a retry.
        let wait = if throttled { base_ms } else { base_ms / 2 };
        sleep_ms(wait).await;
    }
    None
}

async fn fetch_tiles(
    url: &str,
    file_size: u64,
    tiles: &[TbdSatTile],
    mut on_bytes: impl FnMut(u64),
) -> Option<Vec<Vec<u8>>> {
    use futures::stream::StreamExt;

    // One `fetch_range` per ≤SAT_CHUNK_BYTES slice of a tile — sub-tile granularity is what makes
    // the bar move during level 0's four ~25 MB tiles instead of sitting at 0% until they all land.
    let plans: Vec<Vec<(u64, u64)>> = tiles
        .iter()
        .map(|t| split_range(t.offset, t.length, SAT_CHUNK_BYTES))
        .collect();
    let reqs: Vec<(usize, usize, u64, u64)> = plans
        .iter()
        .enumerate()
        .flat_map(|(ti, plan)| {
            plan.iter()
                .enumerate()
                .map(move |(pi, &(start, end))| (ti, pi, start, end))
        })
        .collect();

    let mut parts: Vec<Ordered<Vec<u8>>> = plans.iter().map(|p| Ordered::new(p.len())).collect();
    let mut inflight =
        futures::stream::iter(reqs.into_iter().map(|(ti, pi, start, end)| async move {
            (
                ti,
                pi,
                start,
                end,
                fetch_range_resilient(url, start, end).await,
            )
        }))
        .buffer_unordered(SAT_FETCH_CONCURRENCY);

    while let Some((ti, pi, start, end, got)) = inflight.next().await {
        if got.is_none() {
            leptos::logging::error!(
                "satellite: Range bytes={start}-{end} (tile {ti}, part {pi}) failed after \
                 {RANGE_ATTEMPTS} attempts — abandoning the full-resolution basemap"
            );
        }
        let body = got?;
        let want = end - start + 1;
        // Both halves of the original loop's validation, per request: the body is exactly the span
        // that was asked for, and it came from the file the index describes (a bundle rebuilt
        // mid-boot changes `content-range`'s total, and stitching those bytes in would be silent
        // corruption).
        if body.bytes.len() as u64 != want || body.total != file_size {
            return None;
        }
        on_bytes(want);
        if !parts.get_mut(ti)?.put(pi, body.bytes) {
            return None;
        }
    }
    drop(inflight);

    let mut out = Vec::with_capacity(tiles.len());
    for (tile, slot) in tiles.iter().zip(parts) {
        let chunks = slot.finish()?; // a dropped completion fails here rather than shifting the run
        let mut bytes = Vec::with_capacity(tile.length as usize);
        for c in chunks {
            bytes.extend_from_slice(&c);
        }
        if bytes.len() as u64 != tile.length {
            return None;
        }
        out.push(bytes);
    }
    Some(out)
}

async fn fetch_mip_blocks(
    url: &str,
    file_size: u64,
    mip: &TbdSatMip,
) -> Option<Vec<(TbdSatTile, Vec<u8>)>> {
    let bodies = fetch_tiles(url, file_size, &mip.tiles, |_| {}).await?;
    Some(mip.tiles.iter().cloned().zip(bodies).collect())
}

async fn commit_mip(
    engine: &EngineHandle,
    terrain_w: f64,
    terrain_h: f64,
    mip: &TbdSatMip,
    blocks: Vec<(super::tbd_sat::TbdSatTile, Vec<u8>)>,
    mode: u32,
    mip_count: u32,
    opacity: f64,
) -> bool {
    let webgl2 = {
        let g = engine.borrow();
        g.as_ref().map(|e| e.backend() == "webgl2").unwrap_or(true)
    };
    let mut decoded = Vec::with_capacity(blocks.len());
    for (tile, bytes) in &blocks {
        let Some(d) = decode_webp(bytes, webgl2).await else {
            return false;
        };
        decoded.push((tile.clone(), d));
    }
    let mut guard = engine.borrow_mut();
    let Some(e) = guard.as_mut() else {
        return false;
    };
    if e.tex_layer_begin(
        ROLE_BASEMAP,
        0.0,
        0.0,
        terrain_w,
        terrain_h,
        mip.width,
        mip.height,
        mip_count,
        mode,
    )
    .is_err()
    {
        return false;
    }
    for (tile, d) in decoded {
        if !upload_decoded(e, ROLE_BASEMAP, 0, tile.x, tile.y, d) {
            return false;
        }
    }
    e.tex_layer_commit(ROLE_BASEMAP, opacity as f32, true)
        .is_ok()
}

/// Range-preview one coarse mip (≤1024 px), mode=single. Best-effort.
async fn try_preview(
    engine: &EngineHandle,
    url: &str,
    terrain_w: f64,
    terrain_h: f64,
    bridge: &BridgeHandle,
) -> bool {
    let Some((index, total)) = fetch_index_head(url, false).await else {
        return false;
    };
    let mip = pick_preview_level(&index, PREVIEW_MAX_EDGE).clone();
    let Some(blocks) = fetch_mip_blocks(url, total, &mip).await else {
        return false;
    };
    if !commit_mip(
        engine,
        terrain_w,
        terrain_h,
        &mip,
        blocks,
        MODE_SINGLE,
        1,
        1.0,
    )
    .await
    {
        return false;
    }
    {
        let mut b = bridge.borrow_mut();
        b.sat_w = mip.width;
        b.sat_h = mip.height;
        b.sat_mode = "single".into();
        b.sat_mips = 1;
    }
    publish(bridge);
    true
}

async fn load_unified_full(
    engine: &EngineHandle,
    url: &str,
    terrain_w: f64,
    terrain_h: f64,
    bridge: &BridgeHandle,
    report: &dyn Fn(BootEvent),
) -> bool {
    // T-627 — the index first (12 B header, then ~2.6 KB of JSON), NOT the whole bundle. The old
    // path GET'd all 152,713,114 B of `everon-sat.tbd-sat` and then used only the mips at or below
    // the GPU's `maxTextureDimension2D`; on an 8192-limit device that is level 1 down, so 110.6 MB
    // of level 0 was downloaded and thrown away. Fetching per tile skips it outright — and gives
    // the loading bar a byte budget it can actually count against.
    let Some((index, file_size)) = fetch_index_head(url, true).await else {
        leptos::logging::error!("satellite: could not read the .tbd-sat index from {url}");
        return false;
    };
    // T-629 — the level is chosen from the limit the engine REPORTS, or not at all. The engine is
    // mounted before `world_assets::bootstrap` is even constructed (`mission_editor.rs`: the
    // bootstrap future is built inside the `RenderEngine::create` success arm, after
    // `*engine.borrow_mut() = Some(eng)`), so `None` here is not a boot-ordering race that waiting
    // would fix — it is the engine having failed to come up at all, and the honest response is to
    // leave the preview placeholder in place and say so.
    let limit = texture_limit(engine);
    let Some(base) = pick_base_level_for_limit(&index, limit.map(|l| l.device)) else {
        leptos::logging::error!(
            "satellite: no render engine when the basemap level had to be chosen — refusing to \
             guess a GPU texture limit. The <={PREVIEW_MAX_EDGE} px preview is what is on screen."
        );
        return false;
    };
    let base = base as usize;
    let Some(base_mip) = index.mips.get(base).cloned() else {
        leptos::logging::error!("satellite: index has no mip at the chosen base level {base}");
        return false;
    };
    if let Some(limit) = limit {
        report_chosen_level(&index, base, limit);
    }
    let mip_count = (index.mip_count as usize).saturating_sub(base) as u32;
    let webgl2 = {
        let g = engine.borrow();
        g.as_ref().map(|e| e.backend() == "webgl2").unwrap_or(true)
    };

    // Every tile that will actually be uploaded, flattened in upload order, with its relative mip
    // level alongside. One flat list (rather than a fetch per level) so the bounded concurrency
    // spans levels: the 13 small tails keep the pipe full behind the big levels.
    let plan: Vec<(u32, TbdSatTile)> = index
        .mips
        .iter()
        .enumerate()
        .skip(base)
        .flat_map(|(li, mip)| {
            let rel = (li - base) as u32;
            mip.tiles.iter().cloned().map(move |t| (rel, t))
        })
        .collect();
    let tiles: Vec<TbdSatTile> = plan.iter().map(|(_, t)| t.clone()).collect();
    // The budget is exact and known before the first byte moves: the index lists every tile's
    // `length`. Nothing here is estimated — and because it is the real figure for THIS device
    // (42,152,810 B from level 1 down on an 8192-limit GPU, 152,710,470 B including level 0 on a
    // 16384-limit one), it also re-weights the satellite's share of the bar to what is actually
    // going to be transferred.
    let total: u64 = tiles.iter().map(|t| t.length).sum();
    // T-629 — the request count is the number that broke this: 49 spans at base level 0 against a
    // 40-token bucket. Worth stating next to the byte budget rather than leaving it to be inferred.
    let req_count: usize = tiles
        .iter()
        .map(|t| split_range(t.offset, t.length, SAT_CHUNK_BYTES).len())
        .sum();
    report(BootEvent::Budget(BootSeg::Satellite, total));
    let Some(bodies) = fetch_tiles(url, file_size, &tiles, |n| {
        report(BootEvent::Done(BootSeg::Satellite, n));
    })
    .await
    else {
        leptos::logging::error!(
            "satellite: the Range fetch of {} tiles ({total} B) from level {base} down did not \
             complete",
            tiles.len()
        );
        return false;
    };

    // Decode all mips ≥ base before taking the engine borrow for begin/write/commit.
    // T-629 — every arm below used to be a bare `return false`, so a basemap that failed at decode
    // and one that failed at commit were the same non-event: `load_satellite` discarded the bool
    // and the preview stayed up. Each one now names itself, because "the map is blurry" is not a
    // report anyone can act on and it was the only signal this path produced.
    let mut levels: Vec<(u32, TbdSatTile, Decoded)> = Vec::with_capacity(plan.len());
    for ((rel, tile), bytes) in plan.into_iter().zip(bodies) {
        let Some(d) = decode_webp(&bytes, webgl2).await else {
            leptos::logging::error!(
                "satellite: WebP decode failed for the {}x{} tile at ({},{}) of relative level \
                 {rel} ({} B, webgl2={webgl2})",
                tile.width,
                tile.height,
                tile.x,
                tile.y,
                bytes.len()
            );
            return false;
        };
        levels.push((rel, tile, d));
    }

    {
        let mut guard = engine.borrow_mut();
        let Some(e) = guard.as_mut() else {
            leptos::logging::error!("satellite: the render engine went away before upload");
            return false;
        };
        if let Err(err) = e.tex_layer_begin(
            ROLE_BASEMAP,
            0.0,
            0.0,
            terrain_w,
            terrain_h,
            base_mip.width,
            base_mip.height,
            mip_count,
            MODE_UNIFIED,
        ) {
            leptos::logging::error!(
                "satellite: could not allocate the {}x{} basemap texture with {mip_count} mips: \
                 {err:?}",
                base_mip.width,
                base_mip.height
            );
            return false;
        }
        for (rel, tile, d) in levels {
            if !upload_decoded(e, ROLE_BASEMAP, rel, tile.x, tile.y, d) {
                leptos::logging::error!(
                    "satellite: GPU upload rejected the {}x{} tile at ({},{}) of relative level \
                     {rel}",
                    tile.width,
                    tile.height,
                    tile.x,
                    tile.y
                );
                return false;
            }
        }
        if let Err(err) = e.tex_layer_commit(ROLE_BASEMAP, 1.0_f32, true) {
            leptos::logging::error!("satellite: basemap commit failed: {err:?}");
            return false;
        }
    }
    // T-629 — the one line the operator can check against what he is looking at, printed only once
    // the texture is actually on the GPU. Everything before this point is an intention.
    leptos::logging::log!(
        "satellite: basemap up — level {base}, {}x{} with {mip_count} mips ({total} B over {} \
         Range requests); GPU maxTextureDimension2D = {}",
        base_mip.width,
        base_mip.height,
        req_count,
        limit.map_or(0, |l| l.device)
    );
    {
        let mut b = bridge.borrow_mut();
        b.sat_w = base_mip.width;
        b.sat_h = base_mip.height;
        b.sat_mode = "unified".into();
        b.sat_mips = mip_count;
    }
    publish(bridge);
    true
}

/// T-173 P6/H8 — restore the unified satellite lane as the visible basemap (opacity 1, no
/// texture rebuild). Used when the operator switches the Mission Settings basemap radio back from
/// Map to Satellite.
pub fn show_satellite_basemap(engine: &EngineHandle) {
    if let Some(e) = engine.borrow_mut().as_mut() {
        e.set_lane_opacity(ROLE_BASEMAP, 1.0, true);
    }
}

/// T-173 P6/H8 — load the stylized **Map** cartographic pyramid (`tiles/map/{z}/{x}/{y}.webp`)
/// into the basemap lane as one stitched level. Picks the largest XYZ zoom whose stitched edge
/// (`2^z · 256`) fits the GPU's `maxTextureDimension2D`, decodes every tile, and uploads via the
/// same `tex_layer_*` path the satellite loader uses (single level, MODE_SINGLE). Returns false if
/// the pyramid is absent (tiles not built locally) so the caller can fall back to satellite.
pub async fn load_map_basemap(
    engine: &EngineHandle,
    terrain: &str,
    terrain_w: f64,
    terrain_h: f64,
) -> bool {
    // T-629 — same rule as the satellite base level: the zoom that gets stitched is a rendering
    // resolution, so it comes from a measured limit or the load declines. `unwrap_or(8192)` here
    // was harmless only by luck (z4 = 4096 px fits 8192 anyway) — it is removed because the next
    // person to raise the z cap would have inherited a silent half-resolution map for free.
    let Some(limit) = texture_limit(engine) else {
        leptos::logging::error!(
            "map basemap: no render engine when the pyramid zoom had to be chosen — refusing to \
             guess a GPU texture limit."
        );
        return false;
    };
    let max_dim = limit.device;
    // Largest z in [0, 6] with 2^z·256 ≤ max_dim (cap z4 = 4096² — the cartographic source res).
    let mut z: u32 = 0;
    for cand in 0..=4u32 {
        if (1u32 << cand) * 256 <= max_dim {
            z = cand;
        }
    }
    let tiles_per_side = 1u32 << z;
    let stitched = tiles_per_side * 256;
    let webgl2 = {
        let g = engine.borrow();
        g.as_ref().map(|e| e.backend() == "webgl2").unwrap_or(true)
    };

    // Fetch + decode every tile of the chosen level before taking the engine borrow.
    let mut decoded: Vec<(u32, u32, Decoded)> = Vec::new();
    for ty in 0..tiles_per_side {
        for tx in 0..tiles_per_side {
            let url = format!("/map-assets/{terrain}/tiles/map/{z}/{tx}/{ty}.webp");
            let Some(bytes) = fetch_bytes(&url).await else {
                return false; // pyramid not built — caller falls back to satellite
            };
            let Some(d) = decode_webp(&bytes, webgl2).await else {
                return false;
            };
            decoded.push((tx * 256, ty * 256, d));
        }
    }

    let mut guard = engine.borrow_mut();
    let Some(e) = guard.as_mut() else {
        return false;
    };
    if e.tex_layer_begin(
        ROLE_BASEMAP,
        0.0,
        0.0,
        terrain_w,
        terrain_h,
        stitched,
        stitched,
        1,
        MODE_SINGLE,
    )
    .is_err()
    {
        return false;
    }
    for (x, y, d) in decoded {
        if !upload_decoded(e, ROLE_BASEMAP, 0, x, y, d) {
            return false;
        }
    }
    e.tex_layer_commit(ROLE_BASEMAP, 1.0, true).is_ok()
}

/// Load satellite for `terrain`. Preview via Range first, then the full unified mip chain
/// (preview -> full progressive) on every host, including `localhost` day-to-day (`cargo xtask mk leptos`),
/// so the editor is sharp instead of stuck on the <=1024 px preview: the coarse preview shows
/// first, then `load_unified_full` replaces it in the background.
///
/// T-174: `?sat=preview` (hostname-independent, via `sat_preview_only`) keeps the Range-only path
/// for CI/gate harnesses + fast local iteration -- it never GETs the full bundle body. (`?sat=full`
/// is now a redundant no-op: full is the default on all hosts; the old localhost preview-only
/// default was removed.)
pub async fn load_satellite(
    engine: EngineHandle,
    base: &str,
    unified_url: &str,
    terrain_w: f64,
    terrain_h: f64,
    bridge: BridgeHandle,
    report: &dyn Fn(BootEvent),
) {
    let url = if unified_url.starts_with('/') {
        unified_url.to_string()
    } else {
        format!("{base}/{unified_url}")
    };
    // The preview is one ≤1024 px mip (everon: a single 583 KB Range) fetched before the full
    // index is read, so it is outside the segment's budget and reports nothing. Counting its bytes
    // against a total that does not include them is exactly the overshoot the fraction clamp exists
    // to absorb, and there is no reason to create the case.
    let preview_ok = try_preview(&engine, &url, terrain_w, terrain_h, &bridge).await;
    if sat_preview_only() {
        return;
    }
    // T-629 — the full load's failure used to be discarded into `let _`. That is the operator's
    // report verbatim: "we are using the low resolution satellite map... that should only be a
    // placeholder if the main one doesn't load." When the full load returns false the placeholder
    // is exactly what stays on screen, and nothing said so — the loading bar had already reached
    // 100% on the DEM and world segments, so a silent basemap failure reads as a finished boot.
    if !load_unified_full(&engine, &url, terrain_w, terrain_h, &bridge, report).await {
        if preview_ok {
            leptos::logging::warn!(
                "satellite: the full-resolution basemap did NOT load — the <={PREVIEW_MAX_EDGE} px \
                 preview placeholder is what is on screen."
            );
        } else {
            leptos::logging::error!(
                "satellite: no basemap loaded at all — neither the preview nor the full mip chain."
            );
        }
    }
}
