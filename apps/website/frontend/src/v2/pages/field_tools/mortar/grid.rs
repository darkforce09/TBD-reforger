//! The pure geometry and number formatting the calculator renders with.
//!
//! **Role:** owns the free-text grid encoding the `fire_missions` row stores its coordinates in,
//! its inverse, the projection that fits a gun and a target into the preview box, and the
//! thousands-separated integer the cards print distances with.
//! **Position:** below the panels — every function here is called from the view layer and none of
//! them touches a signal, the DOM or the network.
//! **Signals & state:** none; all four functions are pure.
//! **Invariants:** the grid encoding must round-trip every coordinate the number inputs accept,
//! and the projection returns percentages already clamped to `0..=100`, so a caller can drop them
//! straight into `left:`/`top:`.

/// Format a number the way `Math.round(n).toLocaleString()` does: rounded to the nearest integer
/// and grouped in threes by commas, with the sign in front.
pub(super) fn locale_int(n: f64) -> String {
    let v = n.round() as i64;
    let neg = v < 0;
    let digits = v.abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        let rem = digits.len() - i;
        out.push(c);
        if rem > 1 && (rem - 1) % 3 == 0 {
            out.push(',');
        }
    }
    if neg {
        format!("-{out}")
    } else {
        out
    }
}

/// Write a fire position or target into the single free-text field the `fire_missions` row keeps
/// for it.
///
/// The row also stores the four coordinates numerically, and [`restore`](super::saved_fires::restore)
/// prefers those — but this encoding is what every row written before those columns existed is read
/// back through, and it is what the migration that added them parses. So it stays lossless: a
/// quantised military grid ("012 020") would round 2200.4 down to the nearest hundred metres and
/// hand back a target nobody aimed at, permanently, in exactly the rows that have no other record.
///
/// `f64::to_string` prints `1000` for `1000.0` and keeps every fractional digit, so a whole-metre
/// grid reads back exactly as it was typed. The handler trims the value before storing it and this
/// format has no leading or trailing space, so the stored bytes are the bytes sent.
pub(super) fn fmt_grid(x: f64, y: f64) -> String {
    format!("{x}, {y}")
}

/// The inverse of [`fmt_grid`]. `None` for anything this page did not write — a grid typed by hand
/// into some other client, say — so such a row restores as "no coordinates" rather than as `(0, 0)`.
///
/// # This is wider than the backfill that filled the numeric columns
///
/// The migration that added `fp_x`/`fp_y`/`tgt_x`/`tgt_y` backfilled them from this encoding with
/// `^\s*-?\d+(\.\d+)?\s*,\s*-?\d+(\.\d+)?\s*$`. This function parses with `str::parse::<f64>`,
/// which accepts strictly more syntax. Measured against a live Postgres:
///
/// ```text
///   input            regex   parse_grid
///   '1000, 2000'       t         t        agree — what `fmt_grid` writes
///   '+1000, 2000'      f         t        `-?` has no `+`
///   '.5, 2'            f         t        `\d+` requires a digit before the point
///   '5., 2'            f         t        `(\.\d+)?` requires digits after it
///   '1e3, 500'         f         t        no exponent form
/// ```
///
/// **The divergence is under-permissive, which is the safe direction.** A backfill that accepted
/// more than this function would invent coordinates for rows the calculator has always shown as
/// unrestorable; one that accepts less only leaves a row where it already was, because
/// [`restore`](super::saved_fires::restore)
/// still falls back to this function whenever the numeric columns are null.
///
/// Widening the regex would also be dead code: `fmt_grid` is the only writer of this encoding, it
/// is `format!("{x}, {y}")`, and `f64`'s `Display` never emits a `+`, never a bare leading or
/// trailing point, and never an exponent.
///
/// # The one input where the regex is the wider of the two
///
/// The accept sets are not nested. A grid whose integer part exceeds `f64::MAX` — 309 digits is
/// already enough — matches the regex and then overflows `double precision`, which raises
/// "out of range for type double precision" and would have aborted the whole migration. This
/// function returns `None` for the same string, because `parse::<f64>` yields `inf` and the
/// `is_finite` guard rejects it. No row can have that shape, since `fmt_grid`'s longest possible
/// output is `f64::MAX`'s own 309 digits, which casts cleanly; it would take a hand-written
/// `INSERT` to produce one.
pub(super) fn parse_grid(s: &str) -> Option<(f64, f64)> {
    let (a, b) = s.split_once(',')?;
    let x: f64 = a.trim().parse().ok()?;
    let y: f64 = b.trim().parse().ok()?;
    (x.is_finite() && y.is_finite()).then_some((x, y))
}

/// Project a fire position and a target into the preview box as `(left%, top%)` pairs.
///
/// Pure, and tested by perturbation, because the whole point of the panel is that the markers move
/// with the inputs: a subtree that reads no signal renders perfectly and is still wrong.
///
/// The two points are *fitted* to the box rather than projected onto a terrain extent. There is no
/// terrain on this page — the four inputs are unbounded game-world metres with no mission and no
/// map behind them — so there is no absolute frame to project into, and a fitted frame has the
/// property that matters operationally: both markers are on screen at any separation.
///
/// The frame is square, so the gun-target line's bearing is not sheared; it is centred on the
/// midpoint and 1.6× the larger span, which leaves the pair occupying the middle ~62% with margin
/// for the marker glyphs. `top` is inverted because north is +y on the map and −y in CSS.
pub(super) fn preview_pos(fp: (f64, f64), tgt: (f64, f64)) -> ((f64, f64), (f64, f64)) {
    let mid = ((fp.0 + tgt.0) / 2.0, (fp.1 + tgt.1) / 2.0);
    let span = (fp.0 - tgt.0).abs().max((fp.1 - tgt.1).abs());
    // Coincident FP and TGT (or a non-finite input) has no scale to fit; both markers stack in the
    // centre, which is the truth — the gun is on the target.
    let side = if span.is_finite() && span > 0.0 {
        span * 1.6
    } else {
        return ((50.0, 50.0), (50.0, 50.0));
    };
    let place = |p: (f64, f64)| {
        (
            ((p.0 - (mid.0 - side / 2.0)) / side * 100.0).clamp(0.0, 100.0),
            (((mid.1 + side / 2.0) - p.1) / side * 100.0).clamp(0.0, 100.0),
        )
    };
    (place(fp), place(tgt))
}
