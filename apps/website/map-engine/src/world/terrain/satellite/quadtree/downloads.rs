//! Role: downloads.
//! Position: `world/terrain/satellite/quadtree` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::Ordered;
use super::RANGE_ATTEMPTS;
use super::SAT_CHUNK_BYTES;
use super::SAT_FETCH_CONCURRENCY;
use super::TbdSatMip;
use super::TbdSatTile;
use super::fetch_range_resilient;
use super::split_range;

/// Fetch tiles.
pub(super) async fn fetch_tiles(
    url: &str,
    file_size: u64,
    tiles: &[TbdSatTile],
    mut on_bytes: impl FnMut(u64),
) -> Option<Vec<Vec<u8>>> {
    use futures::stream::StreamExt;

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
            crate::diagnostics::platform::console::error!(
                "satellite: Range bytes={start}-{end} (tile {ti}, part {pi}) failed after \
                 {RANGE_ATTEMPTS} attempts — abandoning the full-resolution basemap"
            );
        }
        let body = got?;
        let want = end - start + 1;

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
        let chunks = slot.finish()?;
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

/// Fetch mip blocks.
pub(super) async fn fetch_mip_blocks(
    url: &str,
    file_size: u64,
    mip: &TbdSatMip,
) -> Option<Vec<(TbdSatTile, Vec<u8>)>> {
    let bodies = fetch_tiles(url, file_size, &mip.tiles, |_| {}).await?;
    Some(mip.tiles.iter().cloned().zip(bodies).collect())
}
