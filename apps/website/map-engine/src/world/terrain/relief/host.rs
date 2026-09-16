//! Role: host.
//! Position: `world/terrain/relief` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::core::culling::lod::class_visible;
use crate::core::culling::lod::contour_interval_for_zoom;
use crate::renderers::primitives::compose::compose_two_tone_contours;
use crate::world::terrain::dem::grid::DEM_VECTOR_GRID_FACTOR;
use crate::world::terrain::dem::grid::DemVectorGrid;
use crate::world::terrain::dem::grid::downsample_dem_grid;
use crate::world::terrain::dem::grid::reduce_grid_2x;
use crate::world::terrain::relief::contours::contour_grid_reductions;
use crate::world::terrain::relief::contours::contour_levels;
use crate::world::terrain::relief::contours::contour_rings;
use crate::world::terrain::relief::contours::summit_ring_indices;
use crate::world::terrain::relief::sea_band::build_sea_band_geometry;
use crate::world::terrain::relief::sea_band::sea_fill_alpha;
use crate::world::terrain::water::mesh::compose_sea_mesh;

use std::rc::Rc;

use crate::core::context::handles::EngineHandle;

const CONTOUR_RGBA: [u8; 4] = [188, 150, 100, 235];

const CONTOUR_SUMMIT_RGBA: [u8; 4] = [174, 145, 123, 235];
const TERRAIN_M: f64 = 12_800.0;

/// Dem vectors.
pub struct DemVectors {
    grid: Option<Rc<DemVectorGrid>>,
    last_interval: f64,
    sea_built_alpha: f64,
}

impl DemVectors {
    /// New.
    pub fn new() -> Self {
        Self {
            grid: None,
            last_interval: 0.0,
            sea_built_alpha: -1.0,
        }
    }

    /// Ensure grid.
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

    /// Grid.
    pub fn grid(&self) -> Option<Rc<DemVectorGrid>> {
        self.grid.clone()
    }

    /// Sync.
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
                e.clear_vector_lane(crate::overlay::lanes::role_id::SEA);
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
                crate::overlay::lanes::role_id::SEA,
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
                e.clear_vector_lane(crate::overlay::lanes::role_id::CONTOURS);
            }
            self.last_interval = 0.0;
            return;
        }

        let m_per_px = 2.0_f64.powf(-zoom);
        let interval = contour_interval_for_zoom(m_per_px);
        if (interval - self.last_interval).abs() < f64::EPSILON {
            return;
        }
        let mut g = grid.clone();
        for _ in 0..contour_grid_reductions(interval) {
            g = reduce_grid_2x(&g);
        }
        let levels = contour_levels(interval, g.max_elev_m);

        let rings = contour_rings(&g, &levels);
        let summit = summit_ring_indices(&rings);
        let hair = compose_two_tone_contours(&rings, &summit, CONTOUR_RGBA, CONTOUR_SUMMIT_RGBA);
        if let Some(e) = engine.borrow_mut().as_mut() {
            e.upload_hairline_segments(
                crate::overlay::lanes::role_id::CONTOURS,
                &hair.verts,
                hair.segment_count,
                true,
            );
        }
        self.last_interval = interval;
    }
}

impl Default for DemVectors {
    fn default() -> Self {
        Self::new()
    }
}
