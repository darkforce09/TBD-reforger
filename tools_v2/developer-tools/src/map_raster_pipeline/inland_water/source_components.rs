//! Classifies inland-water pixels and accepts connected source components.

use super::{
    DENSITY_MIN, FLAT_FRAC_MAX, GREY_RIVER_LOWLAND_SLOPE_DEG, GREY_RIVER_VALLEY_MIN,
    LIN_FLAT_FRAC_MAX, LIN_MIN_AREA_M2, LIN_SLOPE_MEAN_MAX_DEG, LUM_MAX, LUM_MIN, MEAN_SAT_MAX,
    MIN_AREA_M2, OPEN_R, RIBBON_W_MAX_PX, ROAD_OVERLAP_MAX, SAT_MAX, SLOPE_MEAN_MAX_DEG,
    SLOPE_PX_MAX_DEG, WET_LUM_MAX, WET_LUM_MIN, WET_MEAN_LUM_MAX, WET_MEAN_SAT_MAX,
    WET_MIN_AREA_M2, WET_SAT_MAX, WET_SLOPE_PX_MAX_DEG, WET_VALLEY_FRAC_MIN, dilate,
};

pub(super) struct WaterSourcePlanes<'a> {
    pub(super) dimension: usize,
    pub(super) saturation: &'a [f32],
    pub(super) luminance: &'a [f32],
    pub(super) slope: &'a [f32],
    pub(super) elevation_m: &'a [f32],
    pub(super) sea: &'a [u8],
    pub(super) sea_wide: &'a [u8],
    pub(super) flat_wide: &'a [u8],
    pub(super) valley: &'a [u8],
    pub(super) road_corridor: &'a [u8],
}

pub(super) struct WaterSourceComponent {
    pub(super) px: Vec<usize>,
    pub(super) accepted: bool,
    pub(super) klass: &'static str,
    pub(super) road_frac: f64,
    pub(super) area_m2: u64,
    pub(super) mean_sat: f64,
    pub(super) mean_lum: f64,
    pub(super) mean_slope: f64,
    pub(super) mean_elev: f64,
    pub(super) flat_frac: f64,
    pub(super) valley_frac: f64,
    pub(super) ribbon_w: f64,
    pub(super) bbox: [usize; 4],
    pub(super) centre: [f64; 2],
}

pub(super) struct WaterSourceClassification {
    pub(super) components: Vec<WaterSourceComponent>,
    pub(super) grey_ocean_recall: f64,
}

pub(super) fn classify_source_components(
    planes: WaterSourcePlanes<'_>,
    log: &dyn Fn(&str),
) -> WaterSourceClassification {
    let WaterSourcePlanes {
        dimension: d,
        saturation: sat,
        luminance: lum,
        slope,
        elevation_m: elev_m,
        sea,
        sea_wide,
        flat_wide,
        valley,
        road_corridor,
    } = planes;
    // ── Pixel classes ──
    let mut grey = vec![0u8; d * d];
    let mut wet = vec![0u8; d * d];
    let mut grey_px = 0u64;
    let mut wet_px = 0u64;
    let mut grey_on_sea = 0u64;
    let mut sea_px = 0u64;
    for i in 0..d * d {
        let is_grey = sat[i] < SAT_MAX
            && lum[i] > LUM_MIN
            && lum[i] < LUM_MAX
            && slope[i] <= SLOPE_PX_MAX_DEG;
        if sea[i] == 1 {
            sea_px += 1;
            if is_grey {
                grey_on_sea += 1;
            }
        }
        if sea_wide[i] == 1 {
            continue;
        }
        if is_grey {
            grey[i] = 1;
            grey_px += 1;
        }
        if valley[i] == 1
            && lum[i] > WET_LUM_MIN
            && lum[i] < WET_LUM_MAX
            && sat[i] < WET_SAT_MAX
            && slope[i] <= WET_SLOPE_PX_MAX_DEG
        {
            wet[i] = 1;
            wet_px += 1;
        }
    }
    let grey_ocean_recall = grey_on_sea as f64 / sea_px as f64;
    log(&format!(
        "grey px inland (pre-open): {grey_px}; wet-valley px: {wet_px}; ocean grey recall {:.1} %",
        grey_ocean_recall * 100.0
    ));

    // Speckle-tolerant density opening.
    let density_open = |src: &[u8], r: usize, min_frac: f64| -> Vec<u8> {
        let side = 2 * r + 1;
        let need = ((side * side) as f64 * min_frac).ceil() as i64;
        let mut ii = vec![0i64; (d + 1) * (d + 1)];
        for y in 0..d {
            let mut row = 0i64;
            for x in 0..d {
                row += i64::from(src[y * d + x]);
                ii[(y + 1) * (d + 1) + (x + 1)] = ii[y * (d + 1) + (x + 1)] + row;
            }
        }
        let box_sum = |x0: usize, y0: usize, x1: usize, y1: usize| -> i64 {
            ii[(y1 + 1) * (d + 1) + (x1 + 1)]
                - ii[y0 * (d + 1) + (x1 + 1)]
                - ii[(y1 + 1) * (d + 1) + x0]
                + ii[y0 * (d + 1) + x0]
        };
        let mut core = vec![0u8; d * d];
        for y in 0..d {
            for x in 0..d {
                if src[y * d + x] == 0 {
                    continue;
                }
                let s = box_sum(
                    x.saturating_sub(r),
                    y.saturating_sub(r),
                    (x + r).min(d - 1),
                    (y + r).min(d - 1),
                );
                if s >= need {
                    core[y * d + x] = 1;
                }
            }
        }
        let core_wide = dilate(&core, d, r + 1);
        (0..d * d)
            .map(|i| u8::from(src[i] == 1 && core_wide[i] == 1))
            .collect()
    };
    grey = density_open(&grey, OPEN_R, DENSITY_MIN);
    wet = density_open(&wet, 1, 0.55);
    for i in 0..d * d {
        if wet[i] == 1 {
            grey[i] = 1;
        }
    }

    // Connected components + per-component acceptance.
    let mut labels = vec![-1i32; d * d];
    let mut comps: Vec<WaterSourceComponent> = Vec::new();
    for i in 0..d * d {
        if grey[i] == 0 || labels[i] != -1 {
            continue;
        }
        let id = comps.len() as i32;
        let mut st = vec![i];
        labels[i] = id;
        let mut px = Vec::new();
        while let Some(k) = st.pop() {
            px.push(k);
            let x = k % d;
            let y = k / d;
            for (dx, dy) in [(1i64, 0i64), (-1, 0), (0, 1), (0, -1)] {
                let nx = x as i64 + dx;
                let ny = y as i64 + dy;
                if nx < 0 || nx >= d as i64 || ny < 0 || ny >= d as i64 {
                    continue;
                }
                let j = (ny * d as i64 + nx) as usize;
                if grey[j] == 1 && labels[j] == -1 {
                    labels[j] = id;
                    st.push(j);
                }
            }
        }
        let (mut s_sat, mut s_slope, mut s_elev, mut s_lum) = (0f64, 0f64, 0f64, 0f64);
        let (mut n_flat, mut n_valley, mut n_road, mut perim) = (0u64, 0u64, 0u64, 0u64);
        let (mut min_x, mut max_x, mut min_y, mut max_y) = (d, 0usize, d, 0usize);
        for &k in &px {
            s_sat += f64::from(sat[k]);
            s_slope += f64::from(slope[k]);
            s_elev += f64::from(elev_m[k]);
            s_lum += f64::from(lum[k]);
            if flat_wide[k] == 1 {
                n_flat += 1;
            }
            if valley[k] == 1 {
                n_valley += 1;
            }
            if road_corridor[k] == 1 {
                n_road += 1;
            }
            let x = k % d;
            let y = k / d;
            if x == 0
                || x == d - 1
                || y == 0
                || y == d - 1
                || grey[k - 1] == 0
                || grey[k + 1] == 0
                || grey[k - d] == 0
                || grey[k + d] == 0
            {
                perim += 1;
            }
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
        let np = px.len() as f64;
        let mean_sat = s_sat / np;
        let mean_slope = s_slope / np;
        let flat_frac = n_flat as f64 / np;
        let valley_frac = n_valley as f64 / np;
        let area_m2 = px.len() as u64 * 16;
        let ribbon_w = 2.0 * np / (perim.max(1)) as f64;
        let is_linear = ribbon_w <= RIBBON_W_MAX_PX;
        let road_frac = n_road as f64 / np;
        let is_grey_river = mean_sat <= MEAN_SAT_MAX;
        let (klass, mut accepted) = if !is_linear {
            (
                "compact",
                area_m2 >= MIN_AREA_M2
                    && mean_sat <= MEAN_SAT_MAX
                    && flat_frac <= FLAT_FRAC_MAX
                    && mean_slope <= SLOPE_MEAN_MAX_DEG,
            )
        } else if is_grey_river {
            (
                "grey-river",
                area_m2 >= LIN_MIN_AREA_M2
                    && mean_slope <= LIN_SLOPE_MEAN_MAX_DEG
                    && flat_frac <= LIN_FLAT_FRAC_MAX
                    && (valley_frac >= GREY_RIVER_VALLEY_MIN
                        || mean_slope <= GREY_RIVER_LOWLAND_SLOPE_DEG),
            )
        } else {
            (
                "wet-channel",
                area_m2 >= WET_MIN_AREA_M2
                    && mean_slope <= LIN_SLOPE_MEAN_MAX_DEG
                    && flat_frac <= LIN_FLAT_FRAC_MAX
                    && valley_frac >= WET_VALLEY_FRAC_MIN
                    && mean_sat <= WET_MEAN_SAT_MAX
                    && s_lum / np <= WET_MEAN_LUM_MAX,
            )
        };
        if accepted && road_frac > ROAD_OVERLAP_MAX {
            accepted = false;
        }
        comps.push(WaterSourceComponent {
            accepted,
            klass,
            road_frac: (road_frac * 1000.0).round() / 1000.0,
            area_m2,
            mean_sat: (mean_sat * 10000.0).round() / 10000.0,
            mean_lum: ((s_lum / np) * 1000.0).round() / 1000.0,
            mean_slope: (mean_slope * 100.0).round() / 100.0,
            mean_elev: ((s_elev / np) * 10.0).round() / 10.0,
            flat_frac: (flat_frac * 1000.0).round() / 1000.0,
            valley_frac: (valley_frac * 1000.0).round() / 1000.0,
            ribbon_w: (ribbon_w * 100.0).round() / 100.0,
            bbox: [min_x * 4, min_y * 4, (max_x + 1) * 4, (max_y + 1) * 4],
            centre: [
                ((min_x + max_x) as f64 / 2.0) * 4.0,
                12800.0 - ((min_y + max_y) as f64 / 2.0) * 4.0,
            ],
            px,
        });
    }
    WaterSourceClassification {
        components: comps,
        grey_ocean_recall,
    }
}
