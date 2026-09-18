//! Offset pagination: the query parameters list endpoints accept, and the clamping rule that
//! turns them into a bounded `LIMIT`/`OFFSET` pair.

use serde::Deserialize;

/// Offset-pagination query params shared by list endpoints.
#[derive(Debug, Deserialize)]
pub struct PageParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl PageParams {
    /// `(limit, offset)`, clamped: limit defaults to 20 and is capped at 100, offset defaults
    /// to 0. A value outside the accepted range falls back to the default rather than erroring,
    /// so a malformed page link still renders a page.
    pub fn bounds(&self) -> (i64, i64) {
        let limit = self.limit.filter(|&n| n > 0 && n <= 100).unwrap_or(20);
        let offset = self.offset.filter(|&n| n >= 0).unwrap_or(0);
        (limit, offset)
    }
}
