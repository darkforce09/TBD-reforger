//! Role: queries.
//! Position: `streaming/host` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::*;

/// Canonical terrain m value.
pub(super) const TERRAIN_M: f64 = 12_800.0;

/// Camera snapshot.
#[must_use]
pub fn camera_snapshot() -> Option<(f64, f64, f64)> {
    RENDER_CTX.with(|c| {
        c.borrow().as_ref().and_then(|(engine, _)| {
            engine
                .borrow()
                .as_ref()
                .map(|e| (e.target_x(), e.target_y(), e.zoom()))
        })
    })
}

/// Fly to.
pub fn fly_to(x: f64, y: f64, zoom: f64) {
    RENDER_CTX.with(|c| {
        let Some((engine, host)) = c.borrow().as_ref().map(|(e, h)| (e.clone(), h.clone())) else {
            return;
        };
        if let Some(e) = engine.borrow_mut().as_mut() {
            e.set_view(x, y, zoom);
            e.on_camera_changed();
        }
        flush_viewport(host, engine);
    });
}

/// With occluder.
pub fn with_occluder<R>(
    f: impl FnOnce(&crate::spatial::los::world::state::WorldOccluder) -> R,
) -> Option<R> {
    RENDER_CTX.with(|c| {
        let ctx = c.borrow();
        let (_, host) = ctx.as_ref()?;
        let guard = host.try_borrow().ok()?;
        let mh = guard.as_ref()?;
        Some(f(mh.world.occluder()))
    })
}

/// Peer of [`with_occluder`] for the host's fetch bookkeeping (the smoke bridge reads the failed set); the same `None` semantics.
pub fn with_occluder_host<R>(f: impl FnOnce(&OccluderHost) -> R) -> Option<R> {
    RENDER_CTX.with(|c| {
        let ctx = c.borrow();
        let (_, host) = ctx.as_ref()?;
        let guard = host.try_borrow().ok()?;
        let mh = guard.as_ref()?;
        Some(f(mh.world.occluder_host()))
    })
}

/// With water mask.
#[allow(dead_code)]
pub fn with_water_mask<R>(
    f: impl FnOnce(&crate::world::terrain::water::vectors::WaterMask) -> R,
) -> Option<R> {
    RENDER_CTX.with(|c| {
        let ctx = c.borrow();
        let (_, host) = ctx.as_ref()?;
        let guard = host.try_borrow().ok()?;
        let mh = guard.as_ref()?;
        Some(f(mh.water.mask()?))
    })
}

/// `false` on dry ground, and `false` for every kind of *unknown*: off the map, no mask loaded, the host momentarily taken. It answers about water and cannot answer about ground it has no reading for, which is why a **placement guard must ask [`is_known_dry_land`] instead** — this one is the honest mask query, not the permission to build. See [`with_water_mask`] for why this is `dead_code`-allowed.
#[allow(dead_code)]
#[must_use]
pub fn is_water(x: f64, z: f64) -> bool {
    with_water_mask(|m| m.is_water(x, z)).unwrap_or(false)
}

/// `false` in water, `false` off the map, `false` when the terrain ships no bathymetry at all, and `false` while the host is borrowed. Every unknown answers "not here", so a guard that gates on this can be wrong by refusing a legal spot and never by drowning a squad. See [`with_water_mask`] for why this is `dead_code`-allowed.
#[allow(dead_code)]
#[must_use]
pub fn is_known_dry_land(x: f64, z: f64) -> bool {
    with_water_mask(|m| m.is_known_dry_land(x, z)).unwrap_or(false)
}

/// Named locations.
#[must_use]
pub fn named_locations() -> Vec<crate::overlay::symbology::labels::importance::LocationLabel> {
    RENDER_CTX.with(|c| {
        c.borrow()
            .as_ref()
            .and_then(|(_, host)| host.borrow().as_ref().map(|mh| mh.labels.towns().to_vec()))
            .unwrap_or_default()
    })
}
