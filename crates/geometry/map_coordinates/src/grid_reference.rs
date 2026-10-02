//! **Role:** the map's grid reference — the text printed on a map pane's edge labels, and the
//! 6-, 8- and 10-figure references a user reads off the map or types in.
//! **Position:** `map_coordinates`; the map engine's edge labels and selection digest, the Mission
//! Creator's toolbelt and exporters, and the mortar page format and parse through here.
//! **Signals & state:** none; pure formatting and parsing over world metres.
//! **Invariants:** ONE convention. Every precision is `floor(m / cell) mod (100 km / cell)` per
//! axis, easting first, so the 6-figure halves are exactly [`grid_ref_3digit`] and a longer
//! reference only appends digits to a shorter one. A parsed reference names the centre of its
//! cell inside the first 100 km wrap. A second spelling of "which grid square is this" that
//! disagreed with the on-screen labels would be a confident wrong answer, which is worse than none.

/// The drawn grid's line spacing in world metres — the procedural 1 km grid. Grid-reference labels
/// enumerate lines at world multiples of this, so a label can never drift off the line it names.
pub const GRID_STEP_M: f64 = 1000.0;

/// The span after which a grid reference repeats, in world metres: every precision keeps the
/// digits of `m mod 100 km`.
pub const GRID_WRAP_M: f64 = 100_000.0;

/// How many figures a full grid reference carries (easting and northing together).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum GridFigures {
    /// 6 figures: three per axis, a 100 m cell.
    Six,
    /// 8 figures: four per axis, a 10 m cell.
    Eight,
    /// 10 figures: five per axis, a 1 m cell.
    Ten,
}

impl GridFigures {
    /// Every supported precision, coarsest first.
    pub const ALL: [Self; 3] = [Self::Six, Self::Eight, Self::Ten];

    /// The precision whose full reference carries `figures` digits (6, 8 or 10).
    #[must_use]
    pub const fn from_figure_count(figures: usize) -> Option<Self> {
        match figures {
            6 => Some(Self::Six),
            8 => Some(Self::Eight),
            10 => Some(Self::Ten),
            _ => None,
        }
    }

    /// Digits in the full reference: 6, 8 or 10.
    #[must_use]
    pub const fn figure_count(self) -> usize {
        self.digits_per_axis() * 2
    }

    /// Digits each axis contributes: 3, 4 or 5.
    #[must_use]
    pub const fn digits_per_axis(self) -> usize {
        match self {
            Self::Six => 3,
            Self::Eight => 4,
            Self::Ten => 5,
        }
    }

    /// The side of one grid cell at this precision, in world metres: 100, 10 or 1.
    #[must_use]
    pub const fn cell_m(self) -> f64 {
        match self {
            Self::Six => 100.0,
            Self::Eight => 10.0,
            Self::Ten => 1.0,
        }
    }

    /// Cells per axis before the reference wraps: `GRID_WRAP_M / cell_m`.
    const fn cells_per_wrap(self) -> i64 {
        match self {
            Self::Six => 1_000,
            Self::Eight => 10_000,
            Self::Ten => 100_000,
        }
    }
}

/// Why a typed grid reference was refused by [`parse_grid`].
#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)]
pub enum GridParseError {
    /// The text holds nothing but whitespace.
    #[error("grid reference is empty")]
    Empty,
    /// A character other than an ASCII digit or the whitespace between the two halves.
    #[error("grid reference holds {0:?}; only digits and one space are allowed")]
    InvalidCharacter(char),
    /// The digits add up to a count other than 6, 8 or 10.
    #[error("grid reference has {0} digits; expected 6, 8 or 10")]
    UnsupportedDigitCount(usize),
    /// The easting and northing halves have different lengths.
    #[error(
        "grid reference halves differ: {easting_digits} easting digits, {northing_digits} northing digits"
    )]
    MismatchedHalves {
        /// Digits in the easting half.
        easting_digits: usize,
        /// Digits in the northing half.
        northing_digits: usize,
    },
    /// More than two whitespace-separated groups.
    #[error("grid reference has {0} groups; expected easting and northing")]
    TooManyGroups(usize),
}

/// One axis of a grid reference at `figures` precision: `floor(m / cell) mod cells_per_wrap`,
/// zero-padded to the axis digit count. Negative or non-finite yields all zeros — the
/// off-terrain guard.
fn axis_digits(world_m: f64, figures: GridFigures) -> String {
    let width = figures.digits_per_axis();
    if !world_m.is_finite() || world_m < 0.0 {
        return "0".repeat(width);
    }
    let cells = (world_m / figures.cell_m()).floor() as i64;
    let wrapped = cells.rem_euclid(figures.cells_per_wrap());
    format!("{wrapped:0width$}")
}

/// Arma grid reference for a world coordinate: 3-digit **hundreds-of-metres**, wrapping every
/// 100 km (`floor(m / 100) mod 1000`, zero-padded). E.g. `6400 m → 064`, `12000 m → 120`,
/// `0 m → 000`. This is the half a single axis contributes to a six-figure military grid.
/// Negative or non-finite yields `000` — the off-terrain guard.
#[must_use]
pub fn grid_ref_3digit(world_m: f64) -> String {
    axis_digits(world_m, GridFigures::Six)
}

/// The full grid reference of a world point, `"EEE NNN"`, `"EEEE NNNN"` or `"EEEEE NNNNN"`:
/// easting digits, one space, northing digits. The 6-figure halves equal [`grid_ref_3digit`] of
/// each axis, and each longer precision appends digits to the shorter one.
#[must_use]
pub fn format_grid(x: f64, y: f64, figures: GridFigures) -> String {
    format!("{} {}", axis_digits(x, figures), axis_digits(y, figures))
}

/// Parses a typed grid reference into the world coordinates of its cell centre, `(x, y)` metres,
/// within the first 100 km wrap. Accepts 6, 8 or 10 digits, either as two equal halves separated
/// by whitespace (`"064 129"`) or contiguous (`"064129"`), with surrounding whitespace ignored.
///
/// # Errors
/// A [`GridParseError`] for empty text, a non-digit character, a digit count other than 6, 8 or
/// 10, halves of different lengths, or more than two groups.
pub fn parse_grid(text: &str) -> Result<(f64, f64), GridParseError> {
    let groups: Vec<&str> = text.split_whitespace().collect();
    for group in &groups {
        if let Some(bad) = group.chars().find(|c| !c.is_ascii_digit()) {
            return Err(GridParseError::InvalidCharacter(bad));
        }
    }
    let (easting, northing) = match groups.as_slice() {
        [] => return Err(GridParseError::Empty),
        [joined] => {
            if GridFigures::from_figure_count(joined.len()).is_none() {
                return Err(GridParseError::UnsupportedDigitCount(joined.len()));
            }
            joined.split_at(joined.len() / 2)
        }
        [easting, northing] => {
            if easting.len() != northing.len() {
                return Err(GridParseError::MismatchedHalves {
                    easting_digits: easting.len(),
                    northing_digits: northing.len(),
                });
            }
            (*easting, *northing)
        }
        more => return Err(GridParseError::TooManyGroups(more.len())),
    };
    let total = easting.len() + northing.len();
    let figures = GridFigures::from_figure_count(total)
        .ok_or(GridParseError::UnsupportedDigitCount(total))?;
    Ok((
        cell_centre_m(easting, figures),
        cell_centre_m(northing, figures),
    ))
}

/// The world metre at the centre of the cell an axis's digits name. The digits are ASCII and at
/// most five long, so the parse cannot fail.
fn cell_centre_m(digits: &str, figures: GridFigures) -> f64 {
    let cells = digits
        .bytes()
        .fold(0u32, |acc, b| acc * 10 + u32::from(b - b'0'));
    (f64::from(cells) + 0.5) * figures.cell_m()
}

/// The world coordinates of grid lines (multiples of [`GRID_STEP_M`]) within `[lo, hi]` world
/// metres, inclusive — the eastings/northings whose lines cross a map-pane edge. `lo`/`hi` are the
/// visible world span of that edge; the returned values are exactly the drawn line positions, so
/// labelling them can never drift from the grid.
#[must_use]
pub fn grid_lines_in_range(lo: f64, hi: f64) -> Vec<f64> {
    let (lo, hi) = if lo <= hi { (lo, hi) } else { (hi, lo) };
    if !lo.is_finite() || !hi.is_finite() {
        return Vec::new();
    }
    let first_k = (lo / GRID_STEP_M).ceil() as i64;
    let last_k = (hi / GRID_STEP_M).floor() as i64;
    if last_k < first_k {
        return Vec::new();
    }
    (first_k..=last_k).map(|k| k as f64 * GRID_STEP_M).collect()
}

#[cfg(test)]
#[path = "tests/grid_reference.rs"]
mod tests;
