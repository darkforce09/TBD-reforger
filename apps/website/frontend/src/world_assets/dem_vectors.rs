//! T-166 W4 — DEM sea band + contours (React `useWgpuDemVectors` port via core DemVectorGrid).

use map_engine_core::dem::downsample::{
    downsample_dem_grid, reduce_grid_2x, DEM_VECTOR_GRID_FACTOR,
};
use map_engine_core::dem::DemVectorGrid;
use map_engine_core::geometry::contours::{
    contour_grid_reductions, contour_levels, contour_segments,
};
use map_engine_core::geometry::sea_band::{build_sea_band_geometry, sea_fill_alpha};
use map_engine_core::geometry::vector_compose::{compose_contour_hairlines, compose_sea_mesh};
use map_engine_core::world::{class_visible, contour_interval_for_zoom};

use std::rc::Rc;

use crate::select_tool::EngineHandle;

const ROLE_SEA: u32 = 0;
const ROLE_CONTOURS: u32 = 2;
// T-175 A3 — contours were near-invisible: the old dark brown (luma ~72, α180) vanished over both
// the dark satellite photo and the tan Map basemap. Raised to a lighter warm tan-brown at higher
// alpha (luma ~155, α235) so the 1 px hairline reads on both basemaps. (wgpu draws contours as a
// native 1 px LineList — width is not a lever; only colour/alpha is. Operator-tunable.)
const CONTOUR_RGBA: [u8; 4] = [188, 150, 100, 235];
const TERRAIN_M: f64 = 12_800.0;

pub struct DemVectors {
    // Rc: `sync` runs on every camera settle — a plain clone deep-copied the ~10 MB grid each
    // time (T-172 H3). The Rc is also shared out for cursor-Z sampling (T-172 B2).
    grid: Option<Rc<DemVectorGrid>>,
    last_interval: f64,
    sea_built_alpha: f64,
}

impl DemVectors {
    pub fn new() -> Self {
        Self {
            grid: None,
            last_interval: 0.0,
            sea_built_alpha: -1.0,
        }
    }

    pub fn ensure_grid(&mut self, meters: &[f32], width: u32, height: u32) {
        if self.grid.is_some() {
            return;
        }
        self.grid = Some(Rc::new(downsample_dem_grid(
            meters,
            width as usize,
            height as usize,
            DEM_VECTOR_GRID_FACTOR,
            TERRAIN_M,
            TERRAIN_M,
        )));
        self.last_interval = 0.0;
        self.sea_built_alpha = -1.0;
    }

    /// Shared handle to the retained grid (cursor-Z sampling, T-172 B2).
    pub fn grid(&self) -> Option<Rc<DemVectorGrid>> {
        self.grid.clone()
    }

    pub fn sync(&mut self, engine: &EngineHandle, zoom: f64) {
        let Some(grid) = self.grid.clone() else {
            return;
        };
        self.push_sea(engine, zoom, &grid);
        self.push_contours(engine, zoom, &grid);
    }

    fn push_sea(&mut self, engine: &EngineHandle, zoom: f64, grid: &DemVectorGrid) {
        let alpha = sea_fill_alpha(zoom);
        if !class_visible("sea", zoom) || alpha <= 0.0 {
            if let Some(e) = engine.borrow_mut().as_mut() {
                e.clear_vector_lane(ROLE_SEA);
            }
            self.sea_built_alpha = -1.0;
            return;
        }
        if (self.sea_built_alpha - alpha).abs() < f64::EPSILON {
            return;
        }
        let geo = build_sea_band_geometry(grid);
        let mesh = compose_sea_mesh(&geo, alpha);
        if let Some(e) = engine.borrow_mut().as_mut() {
            e.upload_polygon_mesh(
                ROLE_SEA,
                &mesh.positions,
                &mesh.colors,
                &mesh.indices,
                mesh.polygon_count,
                true,
            );
        }
        self.sea_built_alpha = alpha;
    }

    fn push_contours(&mut self, engine: &EngineHandle, zoom: f64, grid: &DemVectorGrid) {
        if !class_visible("contour", zoom) {
            if let Some(e) = engine.borrow_mut().as_mut() {
                e.clear_vector_lane(ROLE_CONTOURS);
            }
            self.last_interval = 0.0;
            return;
        }
        let interval = contour_interval_for_zoom(zoom);
        if (interval - self.last_interval).abs() < f64::EPSILON {
            return;
        }
        let mut g = grid.clone();
        for _ in 0..contour_grid_reductions(interval) {
            g = reduce_grid_2x(&g);
        }
        let levels = contour_levels(interval, g.max_elev_m);
        let segs = contour_segments(&g, &levels);
        let hair = compose_contour_hairlines(&segs, CONTOUR_RGBA);
        if let Some(e) = engine.borrow_mut().as_mut() {
            e.upload_hairline_segments(ROLE_CONTOURS, &hair.verts, hair.segment_count, true);
        }
        self.last_interval = interval;
    }
}

impl Default for DemVectors {
    fn default() -> Self {
        Self::new()
    }
}
