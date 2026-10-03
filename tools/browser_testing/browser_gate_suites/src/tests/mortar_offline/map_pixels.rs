use super::*;

fn png_of(width: u32, height: u32, pixel: impl Fn(u32, u32) -> [u8; 3]) -> Vec<u8> {
    let image = image::RgbImage::from_fn(width, height, |x, y| image::Rgb(pixel(x, y)));
    let mut out = std::io::Cursor::new(Vec::new());
    image.write_to(&mut out, image::ImageFormat::Png).unwrap();
    out.into_inner()
}

const MAP: CssRect = CssRect {
    x: 10.0,
    y: 20.0,
    width: 100.0,
    height: 80.0,
};

#[test]
fn mortar_offline_a_flat_canvas_is_not_drawn_imagery() {
    let png = png_of(200, 150, |_, _| [30, 34, 40]);
    let stats = region_stats(&png, MAP).unwrap();
    assert_eq!(stats.distinct_colours, 1);
    assert!(stats.luma_std_dev < 1e-9);
    assert!(!imagery_drew(stats));
}

#[test]
fn mortar_offline_varied_imagery_inside_the_map_counts_as_drawn() {
    let png = png_of(200, 150, |x, y| {
        if (10..110).contains(&x) && (20..100).contains(&y) {
            [
                (x * 7 % 256) as u8,
                (y * 13 % 256) as u8,
                ((x + y) * 3 % 256) as u8,
            ]
        } else {
            [0, 0, 0]
        }
    });
    let stats = region_stats(&png, MAP).unwrap();
    assert!(stats.distinct_colours >= MIN_DISTINCT_COLOURS, "{stats:?}");
    assert!(imagery_drew(stats), "{stats:?}");
}

#[test]
fn mortar_offline_imagery_outside_the_map_does_not_count() {
    let png = png_of(200, 150, |x, y| {
        if (10..110).contains(&x) && (20..100).contains(&y) {
            [30, 34, 40]
        } else {
            [(x % 256) as u8, (y % 256) as u8, 7]
        }
    });
    assert!(!imagery_drew(region_stats(&png, MAP).unwrap()));
}

#[test]
fn mortar_offline_a_rectangle_off_the_screenshot_is_an_error() {
    let png = png_of(50, 50, |_, _| [1, 2, 3]);
    let off = CssRect {
        x: 60.0,
        y: 0.0,
        width: 10.0,
        height: 10.0,
    };
    assert!(region_stats(&png, off).is_err());
    assert!(region_stats(b"not a png", MAP).is_err());
}
