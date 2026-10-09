//! The seam metrics of the stitched orthophoto and the `verify-sap-seams` gate.
//!
//! **Role:** measures every cell seam and the control columns between seams (`analyze_seams`),
//! summarises the measurements (`summarize`) and judges them against the lane's thresholds
//! (`verify_supertexture_seams`); also locates the orthophoto scratch folder (`supertexture_scratch_dir`).
//! **Position:** child of `aerial_orthophoto`, whose constants and metric types it fills; the
//! seam analysis and the stitcher's seam bridge read its metrics.
//! **Signals & state:** none; pure functions over a decoded image, plus one gate that reads it.
//! **Invariants:** a seam is judged only where it is evaluable (`evaluated`); the thresholds are the
//! parent's named constants.

use super::*;

use ::repository_layout::map_scratch_dir;

pub(super) fn supertexture_scratch_dir() -> Result<PathBuf> {
    Ok(map_scratch_dir(&find_repository_root()?, "everon").join("sap")) // E2c-allow (SAP lane is Eden-only)
}

pub(super) fn round_to_two_decimals(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

pub(super) fn luminance_sum(buf: &[u8], o: usize) -> f64 {
    f64::from(buf[o]) + f64::from(buf[o + 1]) + f64::from(buf[o + 2])
}

pub(super) fn column_gradient(img: &Rgb8, x: usize) -> f64 {
    let stride = img.w * 3;
    let mut s = 0.0;
    for y in 0..img.h {
        let o = y * stride + x * 3;
        s += (luminance_sum(&img.data, o) - luminance_sum(&img.data, o + 3)).abs();
    }
    s / (3.0 * img.h as f64)
}

pub(super) fn row_gradient(img: &Rgb8, y: usize) -> f64 {
    let stride = img.w * 3;
    let base = y * stride;
    let mut s = 0.0;
    for x in 0..img.w {
        let o = base + x * 3;
        s += (luminance_sum(&img.data, o) - luminance_sum(&img.data, o + stride)).abs();
    }
    s / (3.0 * img.w as f64)
}

pub(super) fn column_strip_mean(img: &Rgb8, x0: usize, x1: usize) -> [f64; 3] {
    let stride = img.w * 3;
    let mut acc = [0f64; 3];
    let mut n = 0f64;
    for y in 0..img.h {
        let base = y * stride;
        for x in x0..x1 {
            let o = base + x * 3;
            acc[0] += f64::from(img.data[o]);
            acc[1] += f64::from(img.data[o + 1]);
            acc[2] += f64::from(img.data[o + 2]);
            n += 1.0;
        }
    }
    [acc[0] / n, acc[1] / n, acc[2] / n]
}

pub(super) fn row_strip_mean(img: &Rgb8, y0: usize, y1: usize) -> [f64; 3] {
    let stride = img.w * 3;
    let mut acc = [0f64; 3];
    let mut n = 0f64;
    for y in y0..y1 {
        let base = y * stride;
        for x in 0..img.w {
            let o = base + x * 3;
            acc[0] += f64::from(img.data[o]);
            acc[1] += f64::from(img.data[o + 1]);
            acc[2] += f64::from(img.data[o + 2]);
            n += 1.0;
        }
    }
    [acc[0] / n, acc[1] / n, acc[2] / n]
}

pub(super) fn seam_metric(img: &Rgb8, c: usize, axis: char) -> SeamMetric {
    let grad = |i: usize| -> f64 {
        if axis == 'v' {
            column_gradient(img, i)
        } else {
            row_gradient(img, i)
        }
    };
    let mut cache: std::collections::HashMap<usize, f64> = std::collections::HashMap::new();
    let mut g_at = |i: usize| -> f64 { *cache.entry(i).or_insert_with(|| grad(i)) };

    let mut band_min = f64::INFINITY;
    for i in c - SEAM_HALF_WIDTH_PIXELS..=c + SEAM_HALF_WIDTH_PIXELS - 1 {
        band_min = band_min.min(g_at(i));
    }
    let mut refs = Vec::new();
    for i in c - 20..=c - 13 {
        refs.push(g_at(i));
    }
    for i in c + 12..=c + 19 {
        refs.push(g_at(i));
    }
    let interior: f64 = refs.iter().sum::<f64>() / refs.len() as f64;
    let max_scan = 8usize;
    let mut apron_left = 0usize;
    for i in (c - max_scan..=c - 1).rev() {
        if g_at(i) < FLAT_GRADIENT_EPSILON {
            apron_left += 1;
        } else {
            break;
        }
    }
    let mut apron_right = 0usize;
    for i in c..=c + max_scan - 1 {
        if g_at(i) < FLAT_GRADIENT_EPSILON {
            apron_right += 1;
        } else {
            break;
        }
    }
    let anchor_safe = !(interior > MINIMUM_INTERIOR_DETAIL
        && (apron_left >= SEAM_ANCHOR_OFFSET_PIXELS || apron_right >= SEAM_ANCHOR_OFFSET_PIXELS));
    let (left, right) = if axis == 'v' {
        (
            column_strip_mean(img, c - 12, c - 4),
            column_strip_mean(img, c + 4, c + 12),
        )
    } else {
        (
            row_strip_mean(img, c - 12, c - 4),
            row_strip_mean(img, c + 4, c + 12),
        )
    };
    let step =
        ((left[0] - right[0]).abs() + (left[1] - right[1]).abs() + (left[2] - right[2]).abs())
            / 3.0;
    SeamMetric {
        axis,
        k: c / CELL_PIXELS,
        c,
        band_min_grad: round_to_two_decimals(band_min),
        interior_grad: round_to_two_decimals(interior),
        apron_left,
        apron_right,
        anchor_safe,
        step_delta_rgb: round_to_two_decimals(step),
        evaluated: interior > MINIMUM_INTERIOR_DETAIL,
    }
}

pub(super) fn control_metric(img: &Rgb8, c: usize, axis: char) -> ControlMetric {
    let grad = |i: usize| -> f64 {
        if axis == 'v' {
            column_gradient(img, i)
        } else {
            row_gradient(img, i)
        }
    };
    let mut band_min = f64::INFINITY;
    for i in c - SEAM_HALF_WIDTH_PIXELS..=c + SEAM_HALF_WIDTH_PIXELS - 1 {
        band_min = band_min.min(grad(i));
    }
    ControlMetric {
        axis,
        at: c,
        band_min_grad: round_to_two_decimals(band_min),
    }
}

pub(crate) fn analyze_seams(img: &Rgb8) -> SeamAnalysis {
    let mut vertical = Vec::new();
    let mut horizontal = Vec::new();
    for k in 1..GRID_CELLS_PER_SIDE {
        vertical.push(seam_metric(img, k * CELL_PIXELS, 'v'));
        horizontal.push(seam_metric(img, k * CELL_PIXELS, 'h'));
    }
    let controls = vec![
        control_metric(img, 25 * CELL_PIXELS + 128, 'v'),
        control_metric(img, 30 * CELL_PIXELS + 128, 'v'),
        control_metric(img, 25 * CELL_PIXELS + 128, 'h'),
    ];
    SeamAnalysis {
        vertical,
        horizontal,
        controls,
    }
}

pub(crate) fn summarize(res: &SeamAnalysis) -> SeamSummary<'_> {
    let all: Vec<&SeamMetric> = res.vertical.iter().chain(res.horizontal.iter()).collect();
    let evaluated: Vec<&SeamMetric> = all.iter().copied().filter(|s| s.evaluated).collect();
    let worst = evaluated.iter().copied().min_by(|a, b| {
        a.band_min_grad
            .partial_cmp(&b.band_min_grad)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let fill_failures: Vec<&SeamMetric> = evaluated
        .iter()
        .copied()
        .filter(|s| {
            s.apron_left > 1
                || s.apron_right > 1
                || s.band_min_grad < RELATIVE_GRADIENT_FLOOR * s.interior_grad
        })
        .collect();
    let step_failures: Vec<&SeamMetric> = all
        .iter()
        .copied()
        .filter(|s| s.step_delta_rgb > STEP_DELTA_CAP)
        .collect();
    let anchor_unsafe: Vec<&SeamMetric> = all.iter().copied().filter(|s| !s.anchor_safe).collect();
    let mean_band = if evaluated.is_empty() {
        None
    } else {
        Some(round_to_two_decimals(
            evaluated.iter().map(|s| s.band_min_grad).sum::<f64>() / evaluated.len() as f64,
        ))
    };
    let max_step = round_to_two_decimals(
        all.iter()
            .map(|s| s.step_delta_rgb)
            .fold(f64::MIN, f64::max),
    );
    let absolute_floor_met = evaluated
        .iter()
        .filter(|s| s.band_min_grad >= FILL_GRADIENT_FLOOR)
        .count();
    let worst_apron = evaluated
        .iter()
        .map(|s| s.apron_left.max(s.apron_right))
        .max()
        .unwrap_or(0);
    let worst_ratio = if evaluated.is_empty() {
        None
    } else {
        Some(round_to_two_decimals(
            evaluated
                .iter()
                .map(|s| {
                    if s.interior_grad > 0.0 {
                        s.band_min_grad / s.interior_grad
                    } else {
                        1.0
                    }
                })
                .fold(f64::INFINITY, f64::min),
        ))
    };
    SeamSummary {
        seam_count: all.len(),
        evaluated_count: evaluated.len(),
        worst_evaluated: worst,
        mean_band_min_grad_eval: mean_band,
        max_step_delta: max_step,
        absolute_floor_met,
        worst_apron,
        worst_ratio,
        fill_failures,
        step_failures,
        anchor_unsafe,
    }
}

pub(super) fn format_two_decimals(v: f64) -> String {
    // JS prints round2 numbers via shortest repr (0.5 not 0.50).
    let n = world_export_pipeline::json_number_formatting::js_num(v);
    n.to_string()
}

pub(crate) fn verify_supertexture_seams(terrain: &str) -> Result<u8> {
    if terrain != "everon" {
        eprintln!("only everon supported this slice (got {terrain})");
        return Ok(1);
    }
    let ortho_path = supertexture_scratch_dir()?.join("everon-sap-ortho.png");
    if !ortho_path.exists() {
        eprintln!(
            "verify-sap-seams FAIL: missing {} — run the stitch first",
            ortho_path.display()
        );
        return Ok(1);
    }
    let mut errors: Vec<String> = Vec::new();
    let ok = |m: &str| println!("  ok: {m}");

    let ortho = image_operations::load_png_rgb(&ortho_path)?;
    if ortho.w != ORTHOPHOTO_PIXELS || ortho.h != ORTHOPHOTO_PIXELS {
        errors.push(format!("ortho {}x{} != 12800²", ortho.w, ortho.h));
    } else {
        ok(&format!("ortho {}x{}", ortho.w, ortho.h));
    }
    let stddev = image_operations::normalized_standard_deviation_magick(&ortho_path)?;

    let res = analyze_seams(&ortho);
    let sum = summarize(&res);

    if sum.evaluated_count == 0 {
        errors.push(format!(
            "no textured seams evaluated (interiorGrad > {MINIMUM_INTERIOR_DETAIL}) — unexpected for everon"
        ));
    } else if !sum.fill_failures.is_empty() {
        let sample = sum
            .fill_failures
            .iter()
            .take(6)
            .map(|s| {
                format!(
                    "{}k={}(apron {}/{}, band {})",
                    s.axis,
                    s.k,
                    s.apron_left,
                    s.apron_right,
                    format_two_decimals(s.band_min_grad)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        errors.push(format!(
            "FILL: {}/{} textured seams still flat (apron > 1 or recovery < {RELATIVE_GRADIENT_FLOOR}× interior): {sample}",
            sum.fill_failures.len(),
            sum.evaluated_count
        ));
    } else {
        ok(&format!(
            "FILL: flat band removed on all {} textured seams — worst apron {} (≤1), worst recovery {} (≥ {RELATIVE_GRADIENT_FLOOR}); abs bandMinGrad ≥ {FILL_GRADIENT_FLOOR} on {}/{}",
            sum.evaluated_count,
            sum.worst_apron,
            sum.worst_ratio.map(format_two_decimals).unwrap_or_default(),
            sum.absolute_floor_met,
            sum.evaluated_count
        ));
    }

    if !sum.step_failures.is_empty() {
        let sample = sum
            .step_failures
            .iter()
            .take(6)
            .map(|s| {
                format!(
                    "{}k={}({})",
                    s.axis,
                    s.k,
                    format_two_decimals(s.step_delta_rgb)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        errors.push(format!(
            "STEP: {} seams exceed STEP_CAP {STEP_DELTA_CAP}: {sample}",
            sum.step_failures.len()
        ));
    } else {
        ok(&format!(
            "STEP guard: max cross-seam ΔRGB {} ≤ STEP_CAP {STEP_DELTA_CAP}",
            format_two_decimals(sum.max_step_delta)
        ));
    }

    if !sum.anchor_unsafe.is_empty() {
        let sample = sum
            .anchor_unsafe
            .iter()
            .take(6)
            .map(|s| {
                format!(
                    "{}k={}(apron {}/{})",
                    s.axis, s.k, s.apron_left, s.apron_right
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        errors.push(format!(
            "ANCHOR: {} seams anchor-unsafe (apron reached anchors): {sample}",
            sum.anchor_unsafe.len()
        ));
    } else {
        ok("ANCHOR safety: all seams clear (apron never reached bridge anchors)");
    }

    let flat_controls: Vec<&ControlMetric> = res
        .controls
        .iter()
        .filter(|c| c.band_min_grad < FILL_GRADIENT_FLOOR)
        .collect();
    if !flat_controls.is_empty() {
        errors.push(format!(
            "CONTROL: interior line(s) unexpectedly flat (< FILL_FLOOR {FILL_GRADIENT_FLOOR}): {}",
            flat_controls
                .iter()
                .map(|c| format!(
                    "{}@{}({})",
                    c.axis,
                    c.at,
                    format_two_decimals(c.band_min_grad)
                ))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    } else {
        ok(&format!(
            "control interior lines textured ({})",
            res.controls
                .iter()
                .map(|c| format_two_decimals(c.band_min_grad))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    if stddev <= MIN_STDDEV {
        errors.push(format!(
            "ortho stddev {stddev} <= {MIN_STDDEV} (whole-map blur/flatten?)"
        ));
    } else {
        ok(&format!("global stddev {stddev:.4} (> {MIN_STDDEV})"));
    }

    if !errors.is_empty() {
        eprintln!("\nverify-sap-seams FAIL ({}):", errors.len());
        for e in &errors {
            eprintln!("  - {e}");
        }
        return Ok(1);
    }
    println!("\nverify-sap-seams OK");
    Ok(0)
}

pub(super) fn metric_json(s: &SeamMetric) -> Value {
    json!({
        "axis": s.axis.to_string(), "k": s.k, "c": s.c,
        "bandMinGrad": world_export_pipeline::json_number_formatting::js_num(s.band_min_grad),
        "interiorGrad": world_export_pipeline::json_number_formatting::js_num(s.interior_grad),
        "apronLeft": s.apron_left, "apronRight": s.apron_right,
        "anchorSafe": s.anchor_safe,
        "stepDeltaRgb": world_export_pipeline::json_number_formatting::js_num(s.step_delta_rgb),
        "evaluated": s.evaluated,
    })
}
