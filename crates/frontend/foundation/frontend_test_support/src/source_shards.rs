//! Production source shards joined into the one text a source pin reads.
//!
//! **Role:** turns the shards of one logical source file, embedded by the owning module's
//! `source_pins` test module, into a single text with the test-module declarations removed.
//! **Position:** test-only support; every area's `tests/source_pins.rs` calls it, and the guard
//! tests read what those pins return through `class_r_scrub`.
//! **Signals & state:** none; pure functions.
//! **Invariants:** shards are joined in the order given and each appears once, which keeps the
//! "exactly one definition" property `only_item` and `only_body` depend on.

/// One shard's text with its test-module declaration removed.
///
/// A production file declares its tests as `#[cfg(test)] #[path = "tests/…"] mod …;`. Those lines
/// carry no behaviour, and leaving them in would make a scrubber cut every shard concatenated after
/// the first one, hiding most of the file from the guard that reads it.
pub fn production_shard(shard: &str) -> String {
    let mut out = String::with_capacity(shard.len());
    let mut lines = shard.lines().peekable();
    while let Some(line) = lines.next() {
        if line.trim_start().starts_with("#[cfg(test)]") {
            for skipped in lines.by_ref() {
                if skipped.trim_end().ends_with(';') {
                    break;
                }
            }
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// The shards of one logical source file, each through [`production_shard`], joined in order.
pub fn production_source(shards: &[&str]) -> String {
    shards.iter().map(|shard| production_shard(shard)).collect()
}
