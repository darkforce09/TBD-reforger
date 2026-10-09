//! The world-glyph atlas builder: SVG sources → one lossless WebP atlas and a Deck-ready JSON
//! mapping, rasterised by resvg on a row-major grid of 128 px cells.
//!
//! **Role:** the `build-glyph-atlas` subcommand.
//! **Position:** reads `assets/glyphs/manifest.json` and its SVGs; writes `assets/glyphs/atlas/`.
//! **Signals & state:** none held.
//! **Invariants:** keys are sorted, cells are 128 px, the canvas is a power of two no larger than
//! 4096 px; an empty atlas is refused.

use crate::error::{Result, ResultExt};
use serde_json::{Map, Value, json};

use crate::image_operations::{self, Rgba8};
use ::repository_layout::glyph_assets_dir;
use ::repository_root::find_repository_root;

const CELL_PIXELS: u32 = 128;
const MAX_ATLAS_PX: u32 = 4096;

pub(crate) fn build_glyph_atlas() -> Result<u8> {
    let glyph_dir = glyph_assets_dir(&find_repository_root()?);
    let atlas_dir = glyph_dir.join("atlas");
    let fail = |m: &str| {
        eprintln!("build-glyph-atlas: FAIL — {m}");
        1u8
    };
    let manifest: Value =
        serde_json::from_str(&std::fs::read_to_string(glyph_dir.join("manifest.json"))?)?;
    let glyphs = manifest["glyphs"].as_object().cloned().unwrap_or_default();
    let mut keys: Vec<String> = glyphs.keys().cloned().collect();
    keys.sort();
    if keys.is_empty() {
        return Ok(fail("glyph manifest has no glyphs"));
    }

    let next_pow2 = |n: u32| -> u32 { 2u32.pow((n.max(1) as f64).log2().ceil() as u32) };
    let width = next_pow2((keys.len() as f64).sqrt().ceil() as u32 * CELL_PIXELS);
    let cols = width / CELL_PIXELS;
    let rows = (keys.len() as u32).div_ceil(cols);
    let height = next_pow2(rows * CELL_PIXELS);
    if width > MAX_ATLAS_PX || height > MAX_ATLAS_PX {
        return Ok(fail(&format!(
            "atlas {width}×{height} exceeds {MAX_ATLAS_PX}² (GL-G4) — shrink CELL_PX or split"
        )));
    }

    let mut canvas = vec![0u8; (width * height * 4) as usize];
    let mut icons: Map<String, Value> = Map::new();
    let opts = resvg::usvg::Options::default();
    for (i, key) in keys.iter().enumerate() {
        let g = &glyphs[key];
        let Some(svg_rel) = g["svg"].as_str() else {
            return Ok(fail(&format!("glyph '{key}' has no svg path")));
        };
        let svg_path = glyph_dir.join(svg_rel);
        let svg_data =
            std::fs::read(&svg_path).with_context(|| format!("rasterize '{key}' ({svg_rel})"))?;
        let tree = resvg::usvg::Tree::from_data(&svg_data, &opts)
            .map_err(|e| crate::error::refusal!("rasterize '{key}' ({svg_rel}): {e}"))?;
        let size = tree.size();
        // Fit into the CELL_PX box preserving aspect, centered (magick -resize + -gravity
        // center -extent).
        let scale = (f64::from(CELL_PIXELS) / f64::from(size.width()))
            .min(f64::from(CELL_PIXELS) / f64::from(size.height()));
        let out_w = (f64::from(size.width()) * scale).round().max(1.0) as u32;
        let out_h = (f64::from(size.height()) * scale).round().max(1.0) as u32;
        let mut pixmap = resvg::tiny_skia::Pixmap::new(out_w, out_h)
            .ok_or_else(|| crate::error::refusal!("pixmap {out_w}x{out_h}"))?;
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_scale(scale as f32, scale as f32),
            &mut pixmap.as_mut(),
        );
        // tiny-skia is premultiplied — demultiply to straight alpha for the atlas.
        let cell_x = (i as u32 % cols) * CELL_PIXELS;
        let cell_y = (i as u32 / cols) * CELL_PIXELS;
        let off_x = cell_x + (CELL_PIXELS - out_w) / 2;
        let off_y = cell_y + (CELL_PIXELS - out_h) / 2;
        for y in 0..out_h {
            for x in 0..out_w {
                let px = pixmap
                    .pixel(x, y)
                    .with_context(|| format!("pixel {x},{y} outside the {out_w}x{out_h} pixmap"))?;
                let a = px.alpha();
                let (r, g_, b) = if a == 0 {
                    (0, 0, 0)
                } else {
                    let de = |v: u8| ((u32::from(v) * 255) / u32::from(a)).min(255) as u8;
                    (de(px.red()), de(px.green()), de(px.blue()))
                };
                let o = (((off_y + y) * width + off_x + x) * 4) as usize;
                canvas[o] = r;
                canvas[o + 1] = g_;
                canvas[o + 2] = b;
                canvas[o + 3] = a;
            }
        }
        let anchor = g["anchor"].as_array().cloned().unwrap_or_default();
        let (ax, ay) = if anchor.len() == 2 {
            (
                anchor[0].as_f64().unwrap_or(0.5),
                anchor[1].as_f64().unwrap_or(0.5),
            )
        } else {
            (0.5, 0.5)
        };
        icons.insert(
            key.clone(),
            json!({
                "x": cell_x, "y": cell_y, "width": CELL_PIXELS, "height": CELL_PIXELS,
                "anchorX": world_export_pipeline::json_number_formatting::js_math_round(ax * f64::from(CELL_PIXELS)) as i64,
                "anchorY": world_export_pipeline::json_number_formatting::js_math_round(ay * f64::from(CELL_PIXELS)) as i64,
                "mask": g["tintable"] == true,
            }),
        );
    }

    std::fs::create_dir_all(&atlas_dir)?;
    let webp_path = atlas_dir.join("world-glyphs.webp");
    let webp = image_operations::encode_webp_lossless_rgba(&Rgba8 {
        w: width as usize,
        h: height as usize,
        data: canvas,
    })?;
    // Validate BEFORE write — used to write then fail, leaving a corrupt atlas.
    if webp.len() < 12 || &webp[0..4] != b"RIFF" || &webp[8..12] != b"WEBP" {
        return Ok(fail("emitted atlas is not a RIFF/WEBP file"));
    }
    crate::empty_write_refusal::refuse_empty_write(
        "build-glyph-atlas webp",
        webp.is_empty() || icons.is_empty(),
        "empty webp or zero icons — refusing overwrite of world-glyphs atlas",
    )?;
    std::fs::write(&webp_path, &webp)?;

    let mapping = json!({
        "meta": {
            "schemaVersion": if manifest["schemaVersion"].is_string() { manifest["schemaVersion"].clone() } else { json!("1.0.0") },
            "refZoom": if manifest["refZoom"].is_number() { manifest["refZoom"].clone() } else { json!(3) },
            "width": width,
            "height": height,
            "cellPx": CELL_PIXELS,
        },
        "icons": icons,
    });
    std::fs::write(
        atlas_dir.join("world-glyphs.json"),
        serde_json::to_string_pretty(&mapping)? + "\n",
    )?;

    println!(
        "build-glyph-atlas: OK — {} glyphs → {width}×{height} atlas ({:.1} KB) @ {}",
        keys.len(),
        webp.len() as f64 / 1024.0,
        atlas_dir.display()
    );
    Ok(0)
}
