//! Grid reference.

use super::*;

pub use website_map_engine::camera::grid_reference::{grid_lines_in_range, grid_ref_3digit};

/// A map grid reference and its current screen position.
#[derive(Clone, Debug, PartialEq)]
pub struct EdgeLabel {
    pub pos_px: f64,
    pub text: String,
    pub key: String,
}

#[must_use]
fn edge_label_key(axis: char, pos_px: f64, text: &str) -> String {
    format!("{axis}{}:{text}", pos_px.round() as i64)
}

/// Projects visible easting grid lines onto the map pane top edge.
#[must_use]
pub fn edge_eastings(
    cam: &OrthoCamera,
    pane_left_px: f64,
    pane_right_px: f64,
    top_px: f64,
) -> Vec<EdgeLabel> {
    if pane_right_px <= pane_left_px {
        return Vec::new();
    }
    let wl = cam.unproject_xy(pane_left_px, top_px)[0].clamp(0.0, TERRAIN_SPAN_M);
    let wr = cam.unproject_xy(pane_right_px, top_px)[0].clamp(0.0, TERRAIN_SPAN_M);
    let mut out = Vec::new();
    for wx in grid_lines_in_range(wl, wr) {
        let sx = cam.project([wx, cam.target_y(), 0.0])[0];
        if sx >= pane_left_px - 0.5 && sx <= pane_right_px + 0.5 {
            let text = grid_ref_3digit(wx);
            out.push(EdgeLabel {
                pos_px: sx,
                key: edge_label_key('E', sx, &text),
                text,
            });
        }
    }
    out
}

/// Projects visible northing grid lines onto the map pane left edge.
#[must_use]
pub fn edge_northings(
    cam: &OrthoCamera,
    pane_left_px: f64,
    top_px: f64,
    bottom_px: f64,
) -> Vec<EdgeLabel> {
    if bottom_px <= top_px {
        return Vec::new();
    }
    let w_top = cam.unproject_xy(pane_left_px, top_px)[1].clamp(0.0, TERRAIN_SPAN_M);
    let w_bottom = cam.unproject_xy(pane_left_px, bottom_px)[1].clamp(0.0, TERRAIN_SPAN_M);
    let mut out = Vec::new();
    for wy in grid_lines_in_range(w_bottom, w_top) {
        let sy = cam.project([cam.target_x(), wy, 0.0])[1];
        if sy >= top_px - 0.5 && sy <= bottom_px + 0.5 {
            let text = grid_ref_3digit(wy);
            out.push(EdgeLabel {
                pos_px: sy,
                key: edge_label_key('N', sy, &text),
                text,
            });
        }
    }
    out
}
