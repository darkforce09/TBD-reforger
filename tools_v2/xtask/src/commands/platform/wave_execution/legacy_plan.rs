//! Historical plan lookup for the platform wave driver.
pub fn tickets_at(rev: &str, n: i64) -> Vec<String> {
    ticket_engine::wave_lock::legacy_plan::tickets_at(std::path::Path::new("."), rev, n)
}
