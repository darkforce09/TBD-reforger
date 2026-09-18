use super::*;

/// `crop.sh` — crop a rectangle out of a screenshot (and optionally nearest-neighbour upscale it) so
/// it can be Read at full detail. Ported to the `image` crate (already a dependency); no ffmpeg and
/// no python.
///
/// The Read tool downscales any image over ~190,000 px, which makes small UI text unreadable — keep
/// `w * h * scale²` under that. This warns (does not fail) when the output would exceed it, matching
/// the shell tool's behaviour.
pub fn crop(img_path: &Path, x: u32, y: u32, w: u32, h: u32, scale: u32, out: &Path) -> Result<u8> {
    use image::GenericImageView as _;

    let img = image::ImageReader::open(img_path)
        .with_context(|| format!("open {}", img_path.display()))?
        .decode()
        .with_context(|| format!("decode {}", img_path.display()))?;
    let (iw, ih) = img.dimensions();
    if x + w > iw || y + h > ih {
        return Err(anyhow!(
            "crop {w}x{h}+{x}+{y} out of bounds for {iw}x{ih} image {}",
            img_path.display()
        ));
    }
    // ffmpeg `crop=W:H:X:Y` — the sub-rectangle at (x, y).
    let cropped = img.crop_imm(x, y, w, h);
    let (ow, oh) = (w * scale, h * scale);
    let scaled = if scale == 1 {
        cropped
    } else {
        // ffmpeg `scale=iw*S:ih*S:flags=neighbor` — integer upscale, nearest neighbour (crisp text).
        cropped.resize_exact(ow, oh, image::imageops::FilterType::Nearest)
    };
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    scaled
        .save(out)
        .with_context(|| format!("write {}", out.display()))?;

    let px = (ow as u64) * (oh as u64);
    println!("{}  ({ow}x{oh} = {px}px)", out.display());
    if px > 190_000 {
        eprintln!(
            "WARNING: over ~190000px, Read will downscale this. Use a smaller region or scale."
        );
    }
    Ok(0)
}
