//! Measured row offsets support variable-height cards and scroll anchoring.
use std::collections::BTreeMap;
/// The measured heights of card grid rows. An unmeasured row counts as 520 pixels, and a
/// 16-pixel gap follows every row.
#[derive(Clone, Default)]
pub(crate) struct RowLayout {
    pub heights: BTreeMap<usize, f64>,
}
impl RowLayout {
    /// The measured height of `row`, or 520 pixels until it is measured.
    pub(crate) fn height(&self, row: usize) -> f64 {
        self.heights.get(&row).copied().unwrap_or(520.)
    }
    /// The offset of `row`'s top edge: the heights and gaps of every row above it.
    pub(crate) fn top(&self, row: usize) -> f64 {
        (0..row).map(|r| self.height(r) + 16.).sum()
    }
    /// The row, among the first `count`, whose extent (gap included) contains the vertical
    /// offset `top`, clamped to the last row.
    pub(crate) fn row_at(&self, top: f64, count: usize) -> usize {
        let mut position = 0.;
        for row in 0..count {
            position += self.height(row) + 16.;
            if position > top {
                return row;
            }
        }
        count.saturating_sub(1)
    }
    /// The rows to mount for a viewport of `height` pixels scrolled to `top`: those it covers
    /// plus one row of overscan on each side, bounded to the first `count` rows.
    pub(crate) fn visible(&self, top: f64, height: f64, count: usize) -> Vec<usize> {
        if count == 0 {
            return vec![];
        }
        let first = self.row_at(top, count).saturating_sub(1);
        let last = (self.row_at(top + height, count) + 2).min(count);
        (first..last).collect()
    }
}
