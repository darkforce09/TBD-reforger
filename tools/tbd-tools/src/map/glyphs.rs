//! T-165.9 — world-glyph atlas builder (port of `build-glyph-atlas.mjs`): SVG sources →
//! one lossless-WebP atlas + Deck-ready JSON mapping. Rasterization is resvg (replaces the
//! magick RSVG delegate); layout contract unchanged (sorted keys, 128 px cells, row-major
//! grid on a power-of-two canvas, GL-G4 4096² cap).

use anyhow::{Context, Result};
use serde_json::{Map, Value, json};

use super::img::{self, Rgba8};
use crate::serve::repo_root;

const CELL_PX: u32 = 128;
const MAX_ATLAS_PX: u32 = 4096;

pub fn build_glyph_atlas() -> Result<u8> {
    let glyph_dir = repo_root().join("packages/map-assets/glyphs");
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
    let width = next_pow2((keys.len() as f64).sqrt().ceil() as u32 * CELL_PX);
    let cols = width / CELL_PX;
    let rows = (keys.len() as u32).div_ceil(cols);
    let height = next_pow2(rows * CELL_PX);
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
            .map_err(|e| anyhow::anyhow!("rasterize '{key}' ({svg_rel}): {e}"))?;
        let size = tree.size();
        // Fit into the CELL_PX box preserving aspect, centered (magick -resize + -gravity
        // center -extent).
        let scale = (f64::from(CELL_PX) / f64::from(size.width()))
            .min(f64::from(CELL_PX) / f64::from(size.height()));
        let out_w = (f64::from(size.width()) * scale).round().max(1.0) as u32;
        let out_h = (f64::from(size.height()) * scale).round().max(1.0) as u32;
        let mut pixmap = resvg::tiny_skia::Pixmap::new(out_w, out_h)
            .ok_or_else(|| anyhow::anyhow!("pixmap {out_w}x{out_h}"))?;
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_scale(scale as f32, scale as f32),
            &mut pixmap.as_mut(),
        );
        // tiny-skia is premultiplied — demultiply to straight alpha for the atlas.
        let cell_x = (i as u32 % cols) * CELL_PX;
        let cell_y = (i as u32 / cols) * CELL_PX;
        let off_x = cell_x + (CELL_PX - out_w) / 2;
        let off_y = cell_y + (CELL_PX - out_h) / 2;
        for y in 0..out_h {
            for x in 0..out_w {
                let px = pixmap.pixel(x, y).unwrap();
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
                "x": cell_x, "y": cell_y, "width": CELL_PX, "height": CELL_PX,
                "anchorX": crate::world::jsval::js_math_round(ax * f64::from(CELL_PX)) as i64,
                "anchorY": crate::world::jsval::js_math_round(ay * f64::from(CELL_PX)) as i64,
                "mask": g["tintable"] == true,
            }),
        );
    }

    std::fs::create_dir_all(&atlas_dir)?;
    let webp_path = atlas_dir.join("world-glyphs.webp");
    let webp = img::encode_webp_lossless_rgba(&Rgba8 {
        w: width as usize,
        h: height as usize,
        data: canvas,
    })?;
    std::fs::write(&webp_path, &webp)?;

    let mapping = json!({
        "meta": {
            "schemaVersion": if manifest["schemaVersion"].is_string() { manifest["schemaVersion"].clone() } else { json!("1.0.0") },
            "refZoom": if manifest["refZoom"].is_number() { manifest["refZoom"].clone() } else { json!(3) },
            "width": width,
            "height": height,
            "cellPx": CELL_PX,
        },
        "icons": icons,
    });
    std::fs::write(
        atlas_dir.join("world-glyphs.json"),
        serde_json::to_string_pretty(&mapping)? + "\n",
    )?;

    if webp.len() < 12 || &webp[0..4] != b"RIFF" || &webp[8..12] != b"WEBP" {
        return Ok(fail("emitted atlas is not a RIFF/WEBP file"));
    }
    println!(
        "build-glyph-atlas: OK — {} glyphs → {width}×{height} atlas ({:.1} KB) @ {}",
        keys.len(),
        webp.len() as f64 / 1024.0,
        atlas_dir.display()
    );
    Ok(0)
}
