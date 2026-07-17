//! T-165.9 — the SAP ortho lane: seam metrics (port of `lib/sap-seam-metrics.mjs`),
//! `verify-sap-seams` / `analyze-sap-seams` / `verify-sap-ortho`, the stitcher
//! (`stitch-sap-ortho.mjs` — 2500-cell EDDS decode → north-up canvas), and the seam bridge
//! (`blend-sap-seams.mjs`, both the in-canvas op and the CLI fallback).
//!
//! The magick shell-outs are native ops here (decode/encode via image/png, stddev/HSL/
//! threshold/resize in `img`). Numeric gate thresholds are unchanged; the two derived
//! readouts that ImageMagick computed (global stddev, orientation AE ratio) are recomputed
//! natively — same formulas, huge gate margins (floor 0.02 vs observed ~0.1; ORIENT_MAX 0.2
//! vs match ~0.08), so verdicts are stable even where the resampler differs in the last dp.

use std::path::PathBuf;

use anyhow::{Result, bail};
use serde_json::{Value, json};

use super::img::{self, Rgb8};
use crate::serve::repo_root;
use crate::world::aux::iso_from_system_time;
use crate::world::edds;
use crate::world::pak::PakVfs;

pub const HW: usize = 4;
pub const ANCHOR: usize = HW + 1;
pub const FILL_FLOOR: f64 = 0.25;
pub const REL_FLOOR: f64 = 0.05;
pub const STEP_CAP: f64 = 6.0;
pub const DETAIL_MIN: f64 = 1.0;
pub const FLAT_EPS: f64 = 0.15;
pub const CELL_PX: usize = 256;
pub const GRID: usize = 50;
pub const ORTHO_PX: usize = GRID * CELL_PX;
const MIN_STDDEV: f64 = 0.02;
const ORIENT_MAX: f64 = 0.2;

fn sap_dir() -> PathBuf {
    repo_root().join("packages/map-assets/everon/staging/sap") // E2c-allow (SAP lane is Eden-only)
}

fn r2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/* ─────────────────────────── seam metrics ─────────────────────────── */

pub struct SeamMetric {
    pub axis: char,
    pub k: usize,
    pub c: usize,
    pub band_min_grad: f64,
    pub interior_grad: f64,
    pub apron_left: usize,
    pub apron_right: usize,
    pub anchor_safe: bool,
    pub step_delta_rgb: f64,
    pub evaluated: bool,
}

pub struct ControlMetric {
    pub axis: char,
    pub at: usize,
    pub band_min_grad: f64,
}

pub struct SeamAnalysis {
    pub vertical: Vec<SeamMetric>,
    pub horizontal: Vec<SeamMetric>,
    pub controls: Vec<ControlMetric>,
}

fn lum_sum3(buf: &[u8], o: usize) -> f64 {
    f64::from(buf[o]) + f64::from(buf[o + 1]) + f64::from(buf[o + 2])
}

fn col_grad(img: &Rgb8, x: usize) -> f64 {
    let stride = img.w * 3;
    let mut s = 0.0;
    for y in 0..img.h {
        let o = y * stride + x * 3;
        s += (lum_sum3(&img.data, o) - lum_sum3(&img.data, o + 3)).abs();
    }
    s / (3.0 * img.h as f64)
}

fn row_grad(img: &Rgb8, y: usize) -> f64 {
    let stride = img.w * 3;
    let base = y * stride;
    let mut s = 0.0;
    for x in 0..img.w {
        let o = base + x * 3;
        s += (lum_sum3(&img.data, o) - lum_sum3(&img.data, o + stride)).abs();
    }
    s / (3.0 * img.w as f64)
}

fn col_strip_mean(img: &Rgb8, x0: usize, x1: usize) -> [f64; 3] {
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

fn row_strip_mean(img: &Rgb8, y0: usize, y1: usize) -> [f64; 3] {
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

fn seam_metric(img: &Rgb8, c: usize, axis: char) -> SeamMetric {
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

fn control_metric(img: &Rgb8, c: usize, axis: char) -> ControlMetric {
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

pub struct SeamSummary<'a> {
    pub seam_count: usize,
    pub evaluated_count: usize,
    pub worst_evaluated: Option<&'a SeamMetric>,
    pub mean_band_min_grad_eval: Option<f64>,
    pub max_step_delta: f64,
    pub absolute_floor_met: usize,
    pub worst_apron: usize,
    pub worst_ratio: Option<f64>,
    pub fill_failures: Vec<&'a SeamMetric>,
    pub step_failures: Vec<&'a SeamMetric>,
    pub anchor_unsafe: Vec<&'a SeamMetric>,
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

fn fmt2(v: f64) -> String {
    // JS prints round2 numbers via shortest repr (0.5 not 0.50).
    let n = crate::world::jsval::js_num(v);
    n.to_string()
}

/* ─────────────────────────── verify-sap-seams ─────────────────────────── */

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

    let ortho = img::load_png_rgb(&ortho_path)?;
    if ortho.w != ORTHO_PX || ortho.h != ORTHO_PX {
        errors.push(format!("ortho {}x{} != 12800²", ortho.w, ortho.h));
    } else {
        ok(&format!("ortho {}x{}", ortho.w, ortho.h));
    }
    let stddev = img::stddev_norm_magick(&ortho_path)?;

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

/* ─────────────────────────── analyze-sap-seams ─────────────────────────── */

fn metric_json(s: &SeamMetric) -> Value {
    json!({
        "axis": s.axis.to_string(), "k": s.k, "c": s.c,
        "bandMinGrad": crate::world::jsval::js_num(s.band_min_grad),
        "interiorGrad": crate::world::jsval::js_num(s.interior_grad),
        "apronLeft": s.apron_left, "apronRight": s.apron_right,
        "anchorSafe": s.anchor_safe,
        "stepDeltaRgb": crate::world::jsval::js_num(s.step_delta_rgb),
        "evaluated": s.evaluated,
    })
}

pub fn analyze_sap_seams(terrain: &str) -> Result<u8> {
    if terrain != "everon" {
        eprintln!("only everon supported this slice (got {terrain})");
        return Ok(1);
    }
    let ortho_path = sap_dir().join("everon-sap-ortho.png");
    let out_path = repo_root().join(".ai/artifacts/t090_1_2_2_seam_analysis.json");
    eprintln!("analyze-sap-seams: decoding {} …", ortho_path.display());
    let ortho = img::load_png_rgb(&ortho_path)?;
    let res = analyze_seams(&ortho);
    let sum = summarize(&res);
    let stddev = img::stddev_norm_magick(&ortho_path)?;

    let textured_with_apron: Vec<&SeamMetric> = res
        .vertical
        .iter()
        .chain(res.horizontal.iter())
        .filter(|s| {
            s.evaluated && (s.apron_left + s.apron_right) >= 2 && s.band_min_grad < FILL_FLOOR
        })
        .collect();
    let diagnosis = if sum.max_step_delta > STEP_CAP {
        "exposure_mismatch"
    } else if !textured_with_apron.is_empty() {
        "baked_apron_flat_band"
    } else {
        "clean"
    };

    let generated_at = {
        let full = iso_from_system_time(std::time::SystemTime::now());
        format!("{}Z", &full[..19])
    };
    let pick = |arr: &[SeamMetric], k: usize| -> Option<Value> {
        arr.iter().find(|s| s.k == k).map(metric_json)
    };
    let mut spot: Vec<Value> = Vec::new();
    for v in [
        pick(&res.vertical, 1),
        pick(&res.vertical, 49),
        pick(&res.horizontal, 1),
        pick(&res.horizontal, 49),
        sum.worst_evaluated.map(metric_json),
    ]
    .into_iter()
    .flatten()
    {
        spot.push(json!({
            "axis": v["axis"], "k": v["k"], "interiorGrad": v["interiorGrad"],
            "apronLeft": v["apronLeft"], "apronRight": v["apronRight"],
            "bandMinGrad": v["bandMinGrad"], "anchorSafe": v["anchorSafe"],
        }));
    }

    let report = json!({
        "slice": "T-090.1.2.2",
        "terrain": terrain,
        "orthoPath": "packages/map-assets/everon/staging/sap/everon-sap-ortho.png",
        "gridPx": 256,
        "bandPx": 8,
        "thresholds": { "FILL_FLOOR": FILL_FLOOR, "REL_FLOOR": REL_FLOOR, "STEP_CAP": crate::world::jsval::js_num(STEP_CAP), "DETAIL_MIN": crate::world::jsval::js_num(DETAIL_MIN) },
        "diagnosis": diagnosis,
        "globalStddev": stddev,
        "summary": {
            "seamCount": sum.seam_count,
            "evaluatedCount": sum.evaluated_count,
            "worstApron": sum.worst_apron,
            "worstRatio": sum.worst_ratio.map(crate::world::jsval::js_num),
            "worstBandMinGrad": sum.worst_evaluated.map(|s| crate::world::jsval::js_num(s.band_min_grad)),
            "meanBandMinGradEval": sum.mean_band_min_grad_eval.map(crate::world::jsval::js_num),
            "absoluteFloorMet": sum.absolute_floor_met,
            "maxStepDelta": crate::world::jsval::js_num(sum.max_step_delta),
            "fillFailureCount": sum.fill_failures.len(),
            "stepFailureCount": sum.step_failures.len(),
            "anchorUnsafeCount": sum.anchor_unsafe.len(),
        },
        "worstEvaluated": sum.worst_evaluated.map(metric_json),
        "anchorSpotCheck": spot,
        "vertical": res.vertical.iter().map(metric_json).collect::<Vec<_>>(),
        "horizontal": res.horizontal.iter().map(metric_json).collect::<Vec<_>>(),
        "controls": res.controls.iter().map(|c| json!({ "axis": c.axis.to_string(), "at": c.at, "bandMinGrad": crate::world::jsval::js_num(c.band_min_grad) })).collect::<Vec<_>>(),
        "generatedAt": generated_at,
    });
    std::fs::create_dir_all(out_path.parent().unwrap())?;
    std::fs::write(&out_path, serde_json::to_string_pretty(&report)? + "\n")?;

    println!("\nanalyze-sap-seams — {terrain}");
    println!("  diagnosis:            {diagnosis}");
    println!(
        "  seams:                {} (evaluated/textured: {})",
        sum.seam_count, sum.evaluated_count
    );
    println!(
        "  worst apron (flat run): {}  (primary FILL: apron ≤ 1)",
        sum.worst_apron
    );
    println!(
        "  worst recovery ratio: {}  (REL_FLOOR {REL_FLOOR})",
        sum.worst_ratio.map(fmt2).unwrap_or_default()
    );
    println!(
        "  worst bandMinGrad:    {}  (abs FILL_FLOOR {FILL_FLOOR}: {}/{} met)",
        sum.worst_evaluated
            .map(|s| fmt2(s.band_min_grad))
            .unwrap_or_default(),
        sum.absolute_floor_met,
        sum.evaluated_count
    );
    println!(
        "  mean bandMinGrad:     {}",
        sum.mean_band_min_grad_eval.map(fmt2).unwrap_or_default()
    );
    println!(
        "  max stepΔRGB:         {}  (STEP_CAP {STEP_CAP})",
        fmt2(sum.max_step_delta)
    );
    println!("  global stddev:        {stddev}");
    println!("  fill failures:        {}", sum.fill_failures.len());
    println!("  step failures:        {}", sum.step_failures.len());
    println!("  anchor-unsafe seams:  {}", sum.anchor_unsafe.len());
    println!(
        "  controls (interior bandMinGrad): {}",
        res.controls
            .iter()
            .map(|c| fmt2(c.band_min_grad))
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!(
        "\n  NIT-1 anchor spot-check (apronL/apronR — anchors at c-5/c+4 must clear the apron):"
    );
    for a in &spot {
        println!(
            "    {} k={}: interior={} apron {}/{} bandMin={} anchorSafe={}",
            a["axis"].as_str().unwrap_or(""),
            a["k"],
            a["interiorGrad"],
            a["apronLeft"],
            a["apronRight"],
            a["bandMinGrad"],
            a["anchorSafe"]
        );
    }
    if !sum.anchor_unsafe.is_empty() {
        println!(
            "\n  WARN: {} seam(s) anchor-unsafe (apron ≥ 5) — widen anchors or reduce HW.",
            sum.anchor_unsafe.len()
        );
    }
    println!("\n  wrote {}", out_path.display());
    Ok(0)
}

/* ─────────────────────────── verify-sap-ortho ─────────────────────────── */

pub fn verify_sap_ortho(terrain: &str) -> Result<u8> {
    if terrain != "everon" {
        eprintln!("only everon supported this slice (got {terrain})");
        return Ok(1);
    }
    let root = repo_root();
    let sap = sap_dir();
    let catalog_path = sap.join("cell-catalog.json");
    let meta_path = sap.join("TBD_SatExport_meta.json");
    let ortho_path = sap.join("everon-sap-ortho.png");
    let manifest_path = root.join("packages/map-assets/everon/manifest.json"); // E2c-allow
    let z000 = root.join("packages/map-assets/everon/tiles/satellite/0/0/0.webp"); // E2c-allow

    const EXPECT_CELLS: u64 = 2500;
    const EXPECT_DIM: usize = 12800;

    let mut errors: Vec<String> = Vec::new();
    let ok = |m: &str| println!("  ok: {m}");

    if !catalog_path.exists() {
        errors.push(format!(
            "missing {} — run 'cargo run -q -p tbd-tools --bin world -- sap-catalog' first",
            catalog_path.display()
        ));
    } else {
        let cat: Value = serde_json::from_str(&std::fs::read_to_string(&catalog_path)?)?;
        let cells = cat["cells"].as_array().map(Vec::len).unwrap_or(0) as u64;
        if cat["cellCount"].as_u64() != Some(EXPECT_CELLS) || cells != EXPECT_CELLS {
            errors.push(format!(
                "catalog cellCount {}/{cells} != {EXPECT_CELLS}",
                cat["cellCount"]
            ));
        } else {
            ok(&format!("catalog {EXPECT_CELLS} cells"));
        }
    }

    let mut water_composited = false;
    if !meta_path.exists() {
        errors.push(format!(
            "missing {} — run the stitch first",
            meta_path.display()
        ));
    } else {
        let m: Value = serde_json::from_str(&std::fs::read_to_string(&meta_path)?)?;
        water_composited = m["waterComposite"].is_object();
        if m["source"] != "sap-supertexture-stitch" {
            errors.push(format!(
                "meta.source={}",
                m["source"].as_str().unwrap_or("")
            ));
        }
        if m["cellsDecoded"].as_u64() != Some(EXPECT_CELLS) {
            errors.push(format!(
                "meta.cellsDecoded {} != {EXPECT_CELLS}",
                m["cellsDecoded"]
            ));
        }
        if m["dimensions"] != json!([EXPECT_DIM, EXPECT_DIM]) {
            errors.push(format!("meta.dimensions {}", m["dimensions"]));
        }
        if m["metersPerPixel"] != 1 {
            errors.push(format!("meta.metersPerPixel {} != 1", m["metersPerPixel"]));
        }
        if m["worldBounds"] != json!([0, 0, EXPECT_DIM, EXPECT_DIM]) {
            errors.push(format!("meta.worldBounds {}", m["worldBounds"]));
        }
        if errors.is_empty() {
            ok("meta source/cells/dims/mpp/bounds");
        }
    }

    let mut ortho: Option<Rgb8> = None;
    if !ortho_path.exists() {
        errors.push(format!(
            "missing {} — run the stitch first",
            ortho_path.display()
        ));
    } else {
        let o = img::load_png_rgb(&ortho_path)?;
        if o.w != EXPECT_DIM || o.h != EXPECT_DIM {
            errors.push(format!("ortho {}x{} != {EXPECT_DIM}^2", o.w, o.h));
        } else {
            ok(&format!("ortho {}x{}", o.w, o.h));
        }
        let sd = img::stddev_norm_magick(&ortho_path)?;
        if sd <= MIN_STDDEV {
            errors.push(format!("ortho stddev {sd} <= {MIN_STDDEV} (flat?)"));
        } else {
            ok(&format!("ortho stddev {sd:.4} (> {MIN_STDDEV})"));
        }
        ortho = Some(o);
    }

    // Orientation guard — ortho land-mask vs DEM land-mask rendered north-up (natively).
    if let Some(o) = &ortho
        && manifest_path.exists()
    {
        let manifest: Value = serde_json::from_str(&std::fs::read_to_string(&manifest_path)?)?;
        let dem_path = root
            .join("packages/map-assets/everon") // E2c-allow
            .join(manifest["dem"]["path"].as_str().unwrap_or(""));
        if !dem_path.exists() {
            errors.push(format!(
                "orientation guard: DEM missing at {}",
                dem_path.display()
            ));
        } else {
            let lo = manifest["dem"]["heightRangeMinM"].as_f64().unwrap_or(0.0);
            let hi = manifest["dem"]["heightRangeMaxM"].as_f64().unwrap_or(1.0);
            let sea_frac = (0.0 - lo) / (hi - lo);
            const S: usize = 512;
            let small = img::resize_rgb(o, S, S);
            let mut sap_mask = vec![0u8; S * S]; // 1 = land
            #[allow(clippy::needless_range_loop)]
            for i in 0..S * S {
                let (r, g, b) = (
                    small.data[i * 3],
                    small.data[i * 3 + 1],
                    small.data[i * 3 + 2],
                );
                let land = if water_composited {
                    // land = NOT(blue-water hue window ~[0.50,0.68] with sat floor)
                    let (rf, gf, bf) = (
                        f32::from(r) / 255.0,
                        f32::from(g) / 255.0,
                        f32::from(b) / 255.0,
                    );
                    let max = rf.max(gf).max(bf);
                    let min = rf.min(gf).min(bf);
                    let l = (max + min) / 2.0;
                    let den = 1.0 - (2.0 * l - 1.0).abs();
                    let s = if den < 1e-6 { 0.0 } else { (max - min) / den };
                    let hue = if (max - min).abs() < 1e-6 {
                        0.0
                    } else if (max - rf).abs() < 1e-6 {
                        (((gf - bf) / (max - min)).rem_euclid(6.0)) / 6.0
                    } else if (max - gf).abs() < 1e-6 {
                        ((bf - rf) / (max - min) + 2.0) / 6.0
                    } else {
                        ((rf - gf) / (max - min) + 4.0) / 6.0
                    };
                    !((0.50..=0.68).contains(&hue) && s > 0.05)
                } else {
                    let (s, _) = img::hsl_sat_lum(r, g, b);
                    s > 0.12
                };
                sap_mask[i] = u8::from(land);
            }
            // DEM land = elevation above sea; DEM raster row 0 = south → flip to north-up.
            let (raster, dw, dh) = {
                let dec = png::Decoder::new(std::fs::File::open(&dem_path)?);
                let mut reader = dec.read_info()?;
                let mut data = vec![0u8; reader.output_buffer_size()];
                let info = reader.next_frame(&mut data)?;
                let (w, h) = (info.width as usize, info.height as usize);
                let mut r16 = vec![0u16; w * h];
                for (i, px) in r16.iter_mut().enumerate() {
                    *px = u16::from_be_bytes([data[i * 2], data[i * 2 + 1]]);
                }
                (r16, w, h)
            };
            let sea_u16 = (sea_frac * 65535.0) as f64;
            let mut dem_mask = vec![0u8; S * S];
            for y in 0..S {
                // -flip: north-up display row y ← DEM row (S-1-y) scaled
                let sy = ((S - 1 - y) * dh) / S;
                for x in 0..S {
                    let sx = (x * dw) / S;
                    dem_mask[y * S + x] = u8::from(f64::from(raster[sy * dw + sx]) > sea_u16);
                }
            }
            let ae = sap_mask
                .iter()
                .zip(dem_mask.iter())
                .filter(|(a, b)| a != b)
                .count();
            let ratio = ae as f64 / (S * S) as f64;
            if ratio >= ORIENT_MAX {
                errors.push(format!(
                    "orientation guard: ortho vs north-up DEM AE ratio {ratio:.3} >= {ORIENT_MAX} (basemap upside-down?)"
                ));
            } else {
                ok(&format!(
                    "orientation guard: ortho matches north-up DEM (AE ratio {ratio:.3} < {ORIENT_MAX})"
                ));
            }
        }
    }

    if !z000.exists() || std::fs::metadata(&z000)?.len() == 0 {
        errors.push(format!("missing/empty committed tile {}", z000.display()));
    } else {
        ok(&format!(
            "committed satellite/0/0/0.webp ({} B)",
            std::fs::metadata(&z000)?.len()
        ));
    }

    if !errors.is_empty() {
        eprintln!("\nverify-sap-ortho FAIL ({}):", errors.len());
        for e in &errors {
            eprintln!("  - {e}");
        }
        return Ok(1);
    }
    println!("\nverify-sap-ortho OK");
    Ok(0)
}

/* ─────────────────────────── seam bridge + stitch ─────────────────────────── */

/// `bridgeSeams` — the apron-bridge feather, in place on an interleaved buffer (RGB or RGBA).
pub fn bridge_seams(canvas: &mut [u8], ortho_px: usize, channels: usize) -> Result<usize> {
    let hw = HW;
    let anchor = ANCHOR;
    let span = (2 * hw + 1) as f64;
    let stride = ortho_px * channels;
    if canvas.len() != stride * ortho_px {
        bail!(
            "bridgeSeams: canvas {} B != {} ({ortho_px}²×{channels})",
            canvas.len(),
            stride * ortho_px
        );
    }
    let seams: Vec<usize> = (1..GRID).map(|k| k * CELL_PX).collect();
    for &c in &seams {
        let a_l = c - anchor;
        let a_r = c + hw;
        for y in 0..ortho_px {
            let row = y * stride;
            let o_l = row + a_l * channels;
            let o_r = row + a_r * channels;
            for x in c - hw..=c + hw - 1 {
                let t = (x - a_l) as f64 / span;
                let o = row + x * channels;
                for k in 0..3 {
                    let l = f64::from(canvas[o_l + k]);
                    let r = f64::from(canvas[o_r + k]);
                    canvas[o + k] = crate::world::jsval::js_math_round(l + (r - l) * t) as u8;
                }
            }
        }
    }
    for &c in &seams {
        let a_t = c - anchor;
        let a_b = c + hw;
        for x in 0..ortho_px {
            let col = x * channels;
            let o_t = a_t * stride + col;
            let o_b = a_b * stride + col;
            for y in c - hw..=c + hw - 1 {
                let t = (y - a_t) as f64 / span;
                let o = y * stride + col;
                for k in 0..3 {
                    let tt = f64::from(canvas[o_t + k]);
                    let bb = f64::from(canvas[o_b + k]);
                    canvas[o + k] = crate::world::jsval::js_math_round(tt + (bb - tt) * t) as u8;
                }
            }
        }
    }
    Ok(seams.len())
}

/// stitch-sap-ortho.mjs port: decode all 2500 cells, assemble north-up, bridge seams, write
/// PNG + TBD_SatExport_meta.json.
pub fn stitch_sap_ortho(terrain: &str) -> Result<u8> {
    if terrain != "everon" {
        eprintln!("only everon supported this slice (got {terrain})");
        return Ok(1);
    }
    let out_dir = sap_dir();
    let t0 = std::time::Instant::now();
    let vfs = PakVfs::open_default()?;
    let cells = edds::list_eden_cells(&vfs);
    if cells.len() as u32 != edds::CELL_COUNT {
        eprintln!(
            "FAIL: found {} Eden cells, expected {} — aborting (no holes)",
            cells.len(),
            edds::CELL_COUNT
        );
        return Ok(1);
    }
    let cell_px = edds::CELL_PX as usize;
    let grid = edds::GRID as usize;
    let ortho_px = grid * cell_px;
    let stride = ortho_px * 4;
    let mut canvas = vec![0u8; ortho_px * ortho_px * 4];
    let mut decoded = 0u32;
    for (n, _) in &cells {
        let cell = match edds::decode_cell_rgba(&vfs, *n) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("FAIL: cell {n} decode error: {e} — aborting (no grey fill)");
                return Ok(1);
            }
        };
        if cell.side != cell_px || cell.rgba.len() != cell_px * cell_px * 4 {
            eprintln!("FAIL: cell {n} wrong size (side {}) — aborting", cell.side);
            return Ok(1);
        }
        let (gx, gy) = edds::cell_grid(*n);
        let px = gx as usize * cell_px;
        let py_top = (grid - 1 - gy as usize) * cell_px;
        let cell_stride = cell_px * 4;
        for row in 0..cell_px {
            let dst_row = py_top + (cell_px - 1 - row);
            let dst = dst_row * stride + px * 4;
            canvas[dst..dst + cell_stride]
                .copy_from_slice(&cell.rgba[row * cell_stride..(row + 1) * cell_stride]);
        }
        decoded += 1;
        if decoded.is_multiple_of(250) {
            eprintln!("  decoded {decoded}/{}", edds::CELL_COUNT);
        }
    }
    let seams = bridge_seams(&mut canvas, ortho_px, 4)?;
    eprintln!("  seam repair: bridged {seams} interior seams/axis (apron feather HW=4)");

    std::fs::create_dir_all(&out_dir)?;
    // RGBA → RGB (drop alpha; the ortho is opaque).
    let mut rgb = Vec::with_capacity(ortho_px * ortho_px * 3);
    for px in canvas.chunks_exact(4) {
        rgb.extend_from_slice(&px[..3]);
    }
    let png_path = out_dir.join("everon-sap-ortho.png");
    img::save_png_rgb(
        &png_path,
        &Rgb8 {
            w: ortho_px,
            h: ortho_px,
            data: rgb,
        },
    )?;

    let elapsed = t0.elapsed().as_secs();
    let generated_at = {
        let full = iso_from_system_time(std::time::SystemTime::now());
        format!("{}Z", &full[..19])
    };
    let meta = json!({
        "slice": "T-090.1.2",
        "source": "sap-supertexture-stitch",
        "captureMethodId": 6,
        "terrain": terrain,
        "dimensions": [ortho_px, ortho_px],
        "metersPerPixel": 1,
        "worldBounds": [0, 0, edds::WORLD_M, edds::WORLD_M],
        "grid": grid,
        "cellsDecoded": decoded,
        "cellPx": cell_px,
        "cellMeters": edds::CELL_M,
        "gridMapping": "row-major N=y*50+x; cell gridY=0 = world Z=0 (south); assembled north-up (south at image bottom)",
        "decoder": "tbd-tools world::edds (bcdec_rs BC7 + Rust LZ4) — T-165.9",
        "seamRepair": "T-090.1.2.2",
        "seamRepairStrategy": format!("A-apron-bridge-{HW}px"),
        "seamRepairParams": { "halfWidthPx": HW, "anchorOffsetPx": ANCHOR, "interiorSeamsOnly": true },
        "pngPath": "packages/map-assets/everon/staging/sap/everon-sap-ortho.png",
        "buildSeconds": elapsed,
        "generatedAt": generated_at,
    });
    std::fs::write(
        out_dir.join("TBD_SatExport_meta.json"),
        serde_json::to_string_pretty(&meta)? + "\n",
    )?;
    println!(
        "wrote {} ({ortho_px}x{ortho_px}, {decoded} cells, {elapsed}s)",
        png_path.display()
    );
    Ok(0)
}

/// blend-sap-seams CLI fallback: bridge the EXISTING ortho PNG in place.
pub fn blend_sap_seams_cli(terrain: &str) -> Result<u8> {
    if terrain != "everon" {
        eprintln!("only everon supported this slice (got {terrain})");
        return Ok(1);
    }
    let sap = sap_dir();
    let png_path = sap.join("everon-sap-ortho.png");
    let meta_path = sap.join("TBD_SatExport_meta.json");
    if !png_path.exists() {
        eprintln!(
            "FAIL: {} missing — run the stitch first",
            png_path.display()
        );
        return Ok(1);
    }
    eprintln!(
        "blend-sap-seams (CLI fallback): decoding {} …",
        png_path.display()
    );
    let mut ortho = img::load_png_rgb(&png_path)?;
    let n = bridge_seams(&mut ortho.data, ORTHO_PX, 3)?;
    img::save_png_rgb(&png_path, &ortho)?;
    if meta_path.exists() {
        let mut meta: Value = serde_json::from_str(&std::fs::read_to_string(&meta_path)?)?;
        meta["seamRepair"] = json!("T-090.1.2.2");
        meta["seamRepairStrategy"] = json!(format!("A-apron-bridge-{HW}px"));
        meta["seamRepairParams"] =
            json!({ "halfWidthPx": HW, "anchorOffsetPx": ANCHOR, "interiorSeamsOnly": true });
        std::fs::write(&meta_path, serde_json::to_string_pretty(&meta)? + "\n")?;
    }
    println!(
        "blend-sap-seams: bridged {n} interior seams/axis in {}",
        png_path.display()
    );
    Ok(0)
}
