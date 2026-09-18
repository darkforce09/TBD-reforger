use super::*;

use crate::repository_layout::map_scratch_dir;

pub(super) fn sap_dir() -> PathBuf {
    map_scratch_dir(&repo_root(), "everon").join("sap") // E2c-allow (SAP lane is Eden-only)
}

pub(super) fn r2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

pub(super) fn lum_sum3(buf: &[u8], o: usize) -> f64 {
    f64::from(buf[o]) + f64::from(buf[o + 1]) + f64::from(buf[o + 2])
}

pub(super) fn col_grad(img: &Rgb8, x: usize) -> f64 {
    let stride = img.w * 3;
    let mut s = 0.0;
    for y in 0..img.h {
        let o = y * stride + x * 3;
        s += (lum_sum3(&img.data, o) - lum_sum3(&img.data, o + 3)).abs();
    }
    s / (3.0 * img.h as f64)
}

pub(super) fn row_grad(img: &Rgb8, y: usize) -> f64 {
    let stride = img.w * 3;
    let base = y * stride;
    let mut s = 0.0;
    for x in 0..img.w {
        let o = base + x * 3;
        s += (lum_sum3(&img.data, o) - lum_sum3(&img.data, o + stride)).abs();
    }
    s / (3.0 * img.w as f64)
}

pub(super) fn col_strip_mean(img: &Rgb8, x0: usize, x1: usize) -> [f64; 3] {
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
            col_grad(img, i)
        } else {
            row_grad(img, i)
        }
    };
    let mut cache: std::collections::HashMap<usize, f64> = std::collections::HashMap::new();
    let mut g_at = |i: usize| -> f64 { *cache.entry(i).or_insert_with(|| grad(i)) };

    let mut band_min = f64::INFINITY;
    for i in c - HW..=c + HW - 1 {
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
        if g_at(i) < FLAT_EPS {
            apron_left += 1;
        } else {
            break;
        }
    }
    let mut apron_right = 0usize;
    for i in c..=c + max_scan - 1 {
        if g_at(i) < FLAT_EPS {
            apron_right += 1;
        } else {
            break;
        }
    }
    let anchor_safe = !(interior > DETAIL_MIN && (apron_left >= ANCHOR || apron_right >= ANCHOR));
    let (left, right) = if axis == 'v' {
        (
            col_strip_mean(img, c - 12, c - 4),
            col_strip_mean(img, c + 4, c + 12),
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
        k: c / CELL_PX,
        c,
        band_min_grad: r2(band_min),
        interior_grad: r2(interior),
        apron_left,
        apron_right,
        anchor_safe,
        step_delta_rgb: r2(step),
        evaluated: interior > DETAIL_MIN,
    }
}

pub(super) fn control_metric(img: &Rgb8, c: usize, axis: char) -> ControlMetric {
    let grad = |i: usize| -> f64 {
        if axis == 'v' {
            col_grad(img, i)
        } else {
            row_grad(img, i)
        }
    };
    let mut band_min = f64::INFINITY;
    for i in c - HW..=c + HW - 1 {
        band_min = band_min.min(grad(i));
    }
    ControlMetric {
        axis,
        at: c,
        band_min_grad: r2(band_min),
    }
}

pub fn analyze_seams(img: &Rgb8) -> SeamAnalysis {
    let mut vertical = Vec::new();
    let mut horizontal = Vec::new();
    for k in 1..GRID {
        vertical.push(seam_metric(img, k * CELL_PX, 'v'));
        horizontal.push(seam_metric(img, k * CELL_PX, 'h'));
    }
    let controls = vec![
        control_metric(img, 25 * CELL_PX + 128, 'v'),
        control_metric(img, 30 * CELL_PX + 128, 'v'),
        control_metric(img, 25 * CELL_PX + 128, 'h'),
    ];
    SeamAnalysis {
        vertical,
        horizontal,
        controls,
    }
}

pub fn summarize(res: &SeamAnalysis) -> SeamSummary<'_> {
    let all: Vec<&SeamMetric> = res.vertical.iter().chain(res.horizontal.iter()).collect();
    let evaluated: Vec<&SeamMetric> = all.iter().copied().filter(|s| s.evaluated).collect();
    let worst = evaluated
        .iter()
        .copied()
        .min_by(|a, b| a.band_min_grad.partial_cmp(&b.band_min_grad).unwrap());
    let fill_failures: Vec<&SeamMetric> = evaluated
        .iter()
        .copied()
        .filter(|s| {
            s.apron_left > 1 || s.apron_right > 1 || s.band_min_grad < REL_FLOOR * s.interior_grad
        })
        .collect();
    let step_failures: Vec<&SeamMetric> = all
        .iter()
        .copied()
        .filter(|s| s.step_delta_rgb > STEP_CAP)
        .collect();
    let anchor_unsafe: Vec<&SeamMetric> = all.iter().copied().filter(|s| !s.anchor_safe).collect();
    let mean_band = if evaluated.is_empty() {
        None
    } else {
        Some(r2(
            evaluated.iter().map(|s| s.band_min_grad).sum::<f64>() / evaluated.len() as f64
        ))
    };
    let max_step = r2(all
        .iter()
        .map(|s| s.step_delta_rgb)
        .fold(f64::MIN, f64::max));
    let absolute_floor_met = evaluated
        .iter()
        .filter(|s| s.band_min_grad >= FILL_FLOOR)
        .count();
    let worst_apron = evaluated
        .iter()
        .map(|s| s.apron_left.max(s.apron_right))
        .max()
        .unwrap_or(0);
    let worst_ratio = if evaluated.is_empty() {
        None
    } else {
        Some(r2(evaluated
            .iter()
            .map(|s| {
                if s.interior_grad > 0.0 {
                    s.band_min_grad / s.interior_grad
                } else {
                    1.0
                }
            })
            .fold(f64::INFINITY, f64::min)))
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

pub(super) fn fmt2(v: f64) -> String {
    // JS prints round2 numbers via shortest repr (0.5 not 0.50).
    let n = crate::world_export_pipeline::json_number_formatting::js_num(v);
    n.to_string()
}

pub fn verify_sap_seams(terrain: &str) -> Result<u8> {
    if terrain != "everon" {
        eprintln!("only everon supported this slice (got {terrain})");
        return Ok(1);
    }
    let ortho_path = sap_dir().join("everon-sap-ortho.png");
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
    if ortho.w != ORTHO_PX || ortho.h != ORTHO_PX {
        errors.push(format!("ortho {}x{} != 12800²", ortho.w, ortho.h));
    } else {
        ok(&format!("ortho {}x{}", ortho.w, ortho.h));
    }
    let stddev = image_operations::stddev_norm_magick(&ortho_path)?;

    let res = analyze_seams(&ortho);
    let sum = summarize(&res);

    if sum.evaluated_count == 0 {
        errors.push(format!(
            "no textured seams evaluated (interiorGrad > {DETAIL_MIN}) — unexpected for everon"
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
                    fmt2(s.band_min_grad)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        errors.push(format!(
            "FILL: {}/{} textured seams still flat (apron > 1 or recovery < {REL_FLOOR}× interior): {sample}",
            sum.fill_failures.len(),
            sum.evaluated_count
        ));
    } else {
        ok(&format!(
            "FILL: flat band removed on all {} textured seams — worst apron {} (≤1), worst recovery {} (≥ {REL_FLOOR}); abs bandMinGrad ≥ {FILL_FLOOR} on {}/{}",
            sum.evaluated_count,
            sum.worst_apron,
            sum.worst_ratio.map(fmt2).unwrap_or_default(),
            sum.absolute_floor_met,
            sum.evaluated_count
        ));
    }

    if !sum.step_failures.is_empty() {
        let sample = sum
            .step_failures
            .iter()
            .take(6)
            .map(|s| format!("{}k={}({})", s.axis, s.k, fmt2(s.step_delta_rgb)))
            .collect::<Vec<_>>()
            .join(", ");
        errors.push(format!(
            "STEP: {} seams exceed STEP_CAP {STEP_CAP}: {sample}",
            sum.step_failures.len()
        ));
    } else {
        ok(&format!(
            "STEP guard: max cross-seam ΔRGB {} ≤ STEP_CAP {STEP_CAP}",
            fmt2(sum.max_step_delta)
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
        .filter(|c| c.band_min_grad < FILL_FLOOR)
        .collect();
    if !flat_controls.is_empty() {
        errors.push(format!(
            "CONTROL: interior line(s) unexpectedly flat (< FILL_FLOOR {FILL_FLOOR}): {}",
            flat_controls
                .iter()
                .map(|c| format!("{}@{}({})", c.axis, c.at, fmt2(c.band_min_grad)))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    } else {
        ok(&format!(
            "control interior lines textured ({})",
            res.controls
                .iter()
                .map(|c| fmt2(c.band_min_grad))
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
        "bandMinGrad": crate::world_export_pipeline::json_number_formatting::js_num(s.band_min_grad),
        "interiorGrad": crate::world_export_pipeline::json_number_formatting::js_num(s.interior_grad),
        "apronLeft": s.apron_left, "apronRight": s.apron_right,
        "anchorSafe": s.anchor_safe,
        "stepDeltaRgb": crate::world_export_pipeline::json_number_formatting::js_num(s.step_delta_rgb),
        "evaluated": s.evaluated,
    })
}
