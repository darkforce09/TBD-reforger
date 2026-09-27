//! Measured row offsets support variable-height cards and scroll anchoring.
use std::collections::BTreeMap;
#[derive(Clone, Default)]
pub struct RowLayout {
    pub heights: BTreeMap<usize, f64>,
}
impl RowLayout {
    pub fn height(&self, row: usize) -> f64 {
        self.heights.get(&row).copied().unwrap_or(520.)
    }
    pub fn top(&self, row: usize) -> f64 {
        (0..row).map(|r| self.height(r) + 16.).sum()
    }
    pub fn row_at(&self, top: f64, count: usize) -> usize {
        let mut position = 0.;
        for row in 0..count {
            position += self.height(row) + 16.;
            if position > top {
                return row;
            }
        }
        count.saturating_sub(1)
    }
    pub fn visible(&self, top: f64, height: f64, count: usize) -> Vec<usize> {
        if count == 0 {
            return vec![];
        }
        let first = self.row_at(top, count).saturating_sub(1);
        let last = (self.row_at(top + height, count) + 2).min(count);
        (first..last).collect()
    }
}
