//! T-165.9 — shared image primitives: PNG I/O, WebP header parse (the hand-rolled reader
//! from verify-tile-pyramid.mjs/verify-unified-satellite.mjs), WebP encode (lossless via
//! image-webp, lossy via the vendored-libwebp `webp` crate — N3), Lanczos resize, stddev,
//! box blur, HSL — the ops the Node lane ran through ImageMagick.

use anyhow::{Context, Result, bail};

pub struct Rgb8 {
    pub w: usize,
    pub h: usize,
    /// row-major, 3 B/px
    pub data: Vec<u8>,
}

pub struct Rgba8 {
    pub w: usize,
    pub h: usize,
    /// row-major, 4 B/px
    pub data: Vec<u8>,
}

fn open_unlimited(path: &std::path::Path) -> Result<image::DynamicImage> {
    let mut reader = image::ImageReader::open(path).with_context(|| path.display().to_string())?;
    // The 12800² orthos exceed the crate's default decode budget — the pipeline owns its
    // memory envelope (a single full-res RGB ≈ 470 MB, never two at once).
    reader.no_limits();
    reader.decode().with_context(|| path.display().to_string())
}

/// Decode any PNG to RGB8 (alpha dropped — the `-alpha off` normalize).
pub fn load_png_rgb(path: &std::path::Path) -> Result<Rgb8> {
    let img = open_unlimited(path)?;
    let rgb = img.to_rgb8();
    Ok(Rgb8 {
        w: rgb.width() as usize,
        h: rgb.height() as usize,
        data: rgb.into_raw(),
    })
}

pub fn load_png_rgba(path: &std::path::Path) -> Result<Rgba8> {
    let img = open_unlimited(path)?;
    let rgba = img.to_rgba8();
    Ok(Rgba8 {
        w: rgba.width() as usize,
        h: rgba.height() as usize,
        data: rgba.into_raw(),
    })
}

pub fn save_png_rgb(path: &std::path::Path, img: &Rgb8) -> Result<()> {
    let buf: image::RgbImage =
        image::ImageBuffer::from_raw(img.w as u32, img.h as u32, img.data.clone())
            .ok_or_else(|| anyhow::anyhow!("bad buffer"))?;
    buf.save(path)?;
    Ok(())
}

/// 8-bit grayscale PNG (the mask writers used colorType 0).
pub fn save_png_gray(path: &std::path::Path, w: usize, h: usize, data: &[u8]) -> Result<()> {
    let buf: image::GrayImage = image::ImageBuffer::from_raw(w as u32, h as u32, data.to_vec())
        .ok_or_else(|| anyhow::anyhow!("bad buffer"))?;
    buf.save(path)?;
    Ok(())
}

/// Lanczos3 resize (the magick `-resize` used on this lane).
pub fn resize_rgb(img: &Rgb8, w: usize, h: usize) -> Rgb8 {
    let src: image::RgbImage =
        image::ImageBuffer::from_raw(img.w as u32, img.h as u32, img.data.clone()).expect("buffer");
    let out = image::imageops::resize(
        &src,
        w as u32,
        h as u32,
        image::imageops::FilterType::Lanczos3,
    );
    Rgb8 {
        w,
        h,
        data: out.into_raw(),
    }
}

/// Nearest-neighbour sample (the magick `-sample` used by the landcover classifier —
/// thresholds tuned on true pixel values, never averaged).
pub fn sample_rgb(img: &Rgb8, w: usize, h: usize) -> Rgb8 {
    let mut data = vec![0u8; w * h * 3];
    for y in 0..h {
        let sy = y * img.h / h;
        for x in 0..w {
            let sx = x * img.w / w;
            let s = (sy * img.w + sx) * 3;
            let d = (y * w + x) * 3;
            data[d..d + 3].copy_from_slice(&img.data[s..s + 3]);
        }
    }
    Rgb8 { w, h, data }
}

/// Normalized [0,1] standard deviation — magick's `%[fx:standard_deviation]` semantics:
/// per-channel population stddev, channels averaged (pooling all samples would inflate the
/// value by the between-channel mean spread).
pub fn stddev_norm(img: &Rgb8) -> f64 {
    let n = (img.w * img.h) as f64;
    let mut mean = [0f64; 3];
    for px in img.data.chunks_exact(3) {
        for c in 0..3 {
            mean[c] += f64::from(px[c]);
        }
    }
    for m in &mut mean {
        *m /= n;
    }
    let mut var = [0f64; 3];
    for px in img.data.chunks_exact(3) {
        for c in 0..3 {
            let d = f64::from(px[c]) - mean[c];
            var[c] += d * d;
        }
    }
    let sd: f64 = var.iter().map(|v| (v / n).sqrt()).sum::<f64>() / 3.0;
    sd / 255.0
}

/// magick's unqualified `%[fx:standard_deviation]` averages over EVERY channel of the
/// source — including a constant alpha plane (stddev 0), which divides the RGB mean by 4/3
/// on TrueColorAlpha files. This variant reproduces that by consulting the file's own
/// channel count.
pub fn stddev_norm_magick(path: &std::path::Path) -> Result<f64> {
    let img = open_unlimited(path)?;
    let has_alpha = img.color().has_alpha();
    let rgb = Rgb8 {
        w: img.width() as usize,
        h: img.height() as usize,
        data: img.to_rgb8().into_raw(),
    };
    let rgb_mean_sd = stddev_norm(&rgb) * 3.0; // sum of the 3 channel stddevs
    Ok(if has_alpha {
        // constant-alpha plane contributes 0; magick still divides by 4
        rgb_mean_sd / 4.0
    } else {
        rgb_mean_sd / 3.0
    })
}

/// Two-pass separable box blur on an f32 plane (the shared .mjs shape — clamped edges).
pub fn box_blur_f32(src: &[f32], w: usize, h: usize, radius: usize) -> Vec<f32> {
    let r = radius as isize;
    let win = (2 * r + 1) as f32;
    let mut tmp = vec![0f32; src.len()];
    let mut dst = vec![0f32; src.len()];
    for y in 0..h {
        let row = y * w;
        let mut acc = 0f32;
        for x in -r..=r {
            acc += src[row + (x.clamp(0, w as isize - 1)) as usize];
        }
        for x in 0..w {
            tmp[row + x] = acc / win;
            let add = ((x as isize) + r + 1).min(w as isize - 1) as usize;
            let sub = ((x as isize) - r).max(0) as usize;
            acc += src[row + add] - src[row + sub];
        }
    }
    for x in 0..w {
        let mut acc = 0f32;
        for y in -r..=r {
            acc += tmp[(y.clamp(0, h as isize - 1)) as usize * w + x];
        }
        for y in 0..h {
            dst[y * w + x] = acc / win;
            let add = ((y as isize) + r + 1).min(h as isize - 1) as usize;
            let sub = ((y as isize) - r).max(0) as usize;
            acc += tmp[add * w + x] - tmp[sub * w + x];
        }
    }
    dst
}

/// HSL saturation + lightness from 8-bit RGB (the classifier space).
pub fn hsl_sat_lum(r: u8, g: u8, b: u8) -> (f32, f32) {
    let (r, g, b) = (
        f32::from(r) / 255.0,
        f32::from(g) / 255.0,
        f32::from(b) / 255.0,
    );
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let den = 1.0 - (2.0 * l - 1.0).abs();
    let s = if den < 1e-6 { 0.0 } else { (max - min) / den };
    (s, l)
}

/* ─────────────────────────── WebP ─────────────────────────── */

pub struct WebpInfo {
    pub fourcc: [u8; 4],
    pub w: u32,
    pub h: u32,
}

/// The hand-rolled RIFF/WEBP header reader (same parse as the Node verifiers): VP8L
/// lossless dims, `VP8 ` lossy keyframe dims, other fourccs magic-only.
pub fn webp_dims(buf: &[u8]) -> Option<WebpInfo> {
    if buf.len() < 30 || &buf[0..4] != b"RIFF" || &buf[8..12] != b"WEBP" {
        return None;
    }
    let fourcc: [u8; 4] = [buf[12], buf[13], buf[14], buf[15]];
    if &fourcc == b"VP8L" {
        if buf[20] != 0x2f {
            return Some(WebpInfo { fourcc, w: 0, h: 0 });
        }
        let v = u32::from_le_bytes([buf[21], buf[22], buf[23], buf[24]]);
        return Some(WebpInfo {
            fourcc,
            w: (v & 0x3fff) + 1,
            h: ((v >> 14) & 0x3fff) + 1,
        });
    }
    if &fourcc == b"VP8 " {
        if buf[23] != 0x9d || buf[24] != 0x01 || buf[25] != 0x2a {
            return Some(WebpInfo { fourcc, w: 0, h: 0 });
        }
        return Some(WebpInfo {
            fourcc,
            w: u32::from(u16::from_le_bytes([buf[26], buf[27]]) & 0x3fff),
            h: u32::from(u16::from_le_bytes([buf[28], buf[29]]) & 0x3fff),
        });
    }
    Some(WebpInfo { fourcc, w: 0, h: 0 })
}

/// Lossless WebP (VP8L) encode — pure Rust `image-webp` (the cwebp `-lossless` legs).
pub fn encode_webp_lossless_rgb(img: &Rgb8) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let enc = image_webp::WebPEncoder::new(std::io::Cursor::new(&mut out));
    enc.encode(
        &img.data,
        img.w as u32,
        img.h as u32,
        image_webp::ColorType::Rgb8,
    )
    .context("webp lossless encode")?;
    Ok(out)
}

pub fn encode_webp_lossless_rgba(img: &Rgba8) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let enc = image_webp::WebPEncoder::new(std::io::Cursor::new(&mut out));
    enc.encode(
        &img.data,
        img.w as u32,
        img.h as u32,
        image_webp::ColorType::Rgba8,
    )
    .context("webp lossless encode")?;
    Ok(out)
}

/// Lossy WebP (VP8) encode — the ONE vendored-C leg (N3: the map-view pyramid's
/// `webp-lossy` manifest contract; no pure-Rust lossy encoder exists).
pub fn encode_webp_lossy_rgb(img: &Rgb8, quality: f32) -> Vec<u8> {
    let enc = webp::Encoder::from_rgb(&img.data, img.w as u32, img.h as u32);
    enc.encode(quality).to_vec()
}

/// Decode a WebP block (either codec) to RGB8 via image-webp.
pub fn decode_webp_rgb(buf: &[u8]) -> Result<Rgb8> {
    let mut dec = image_webp::WebPDecoder::new(std::io::Cursor::new(buf)).context("webp decode")?;
    let (w, h) = dec.dimensions();
    let bpp = if dec.has_alpha() { 4 } else { 3 };
    let mut data = vec![0u8; w as usize * h as usize * bpp];
    dec.read_image(&mut data).context("webp read")?;
    let rgb = if bpp == 4 {
        let mut out = Vec::with_capacity(w as usize * h as usize * 3);
        for px in data.chunks_exact(4) {
            out.extend_from_slice(&px[..3]);
        }
        out
    } else {
        data
    };
    Ok(Rgb8 {
        w: w as usize,
        h: h as usize,
        data: rgb,
    })
}

/// Crop a sub-rect (clamped to bounds must hold).
pub fn crop_rgb(img: &Rgb8, x: usize, y: usize, w: usize, h: usize) -> Result<Rgb8> {
    if x + w > img.w || y + h > img.h {
        bail!("crop {x},{y} {w}x{h} exceeds {}x{}", img.w, img.h);
    }
    let mut data = Vec::with_capacity(w * h * 3);
    for row in y..y + h {
        let o = (row * img.w + x) * 3;
        data.extend_from_slice(&img.data[o..o + w * 3]);
    }
    Ok(Rgb8 { w, h, data })
}
