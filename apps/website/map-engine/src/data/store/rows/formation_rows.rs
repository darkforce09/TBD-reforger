//! Role: formation rows.
//! Position: `doc/store` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

/// Canonical formation spacing m value.
pub const FORMATION_SPACING_M: f64 = 10.0;

/// An unknown or empty token falls back to `column` — deliberately the shape whose result is unmistakable at any count (a straight trail behind the leader) and which cannot be confused with any of the other eight. A typo therefore produces a visibly wrong arrangement the operator notices, rather than a plausible formation they did not ask for.
#[must_use]
pub fn formation_offsets(formation: &str, n: usize) -> Vec<(f64, f64)> {
    let s = FORMATION_SPACING_M;

    let alternating = |i: usize| -> (f64, f64) {
        #[allow(clippy::cast_precision_loss)]
        let rank = (i / 2 + 1) as f64;
        let side = if i.is_multiple_of(2) { 1.0 } else { -1.0 };
        (side, rank)
    };
    #[allow(clippy::cast_precision_loss)]
    let trail = |i: usize| (i + 1) as f64;

    (0..n)
        .map(|i| match formation {
            "stagger_column" => {
                let side = if i.is_multiple_of(2) { 0.5 } else { -0.5 };
                (side * s, -trail(i) * s)
            }

            "wedge" => {
                let (side, rank) = alternating(i);
                (side * rank * s, -rank * s)
            }

            "vee" => {
                let (side, rank) = alternating(i);
                (side * rank * s, rank * s)
            }

            "echelon_left" => (-trail(i) * s, -trail(i) * s),
            "echelon_right" => (trail(i) * s, -trail(i) * s),

            "line" => {
                let (side, rank) = alternating(i);
                (side * rank * s, 0.0)
            }

            "file" => (0.0, -trail(i) * s * 0.5),

            "diamond" => {
                #[allow(clippy::cast_precision_loss)]
                let ring = (i / 3 + 1) as f64;
                match i % 3 {
                    0 => (ring * s, -ring * s),
                    1 => (-ring * s, -ring * s),
                    _ => (0.0, -2.0 * ring * s),
                }
            }

            _ => (0.0, -trail(i) * s),
        })
        .collect()
}
