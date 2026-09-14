//! Production source, addressed by the file it used to be, for the crate's guard tests.
//!
//! **Role:** each function returns the full text of one logical source file. Where that file has
//! been split into shards, the function concatenates them in declaration order, so a guard that
//! used to `include_str!` the single file still sees every definition exactly once.
//! **Position:** test-only support, called from the guard tests of the modules it names.
//! **Signals & state:** none — every include is resolved at compile time.
//! **Invariants:** concatenation preserves the "exactly one definition" property the
//! `only_item` and `only_body` helpers depend on; a shard must therefore never be listed twice.

/// One shard's text with its test-module declaration removed.
///
/// A production file declares its tests as `#[cfg(test)] #[path = "tests/…"] mod …;`. Those lines
/// carry no behaviour, and leaving them in would make a scrubber cut every shard concatenated after
/// the first one, hiding most of the file from the guard that reads it.
fn production(shard: &str) -> String {
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

/// The HTTP client, as one text: the failure types, the refresh policy and the request verbs.
pub(crate) fn client_source() -> String {
    [
        include_str!("../api/client/mod.rs"),
        include_str!("../api/client/errors.rs"),
        include_str!("../api/client/fetched.rs"),
        include_str!("../api/client/refresh.rs"),
        include_str!("../api/client/requests.rs"),
    ]
    .map(production)
    .concat()
}

/// The telemetry stream consumer.
pub(crate) fn sse_source() -> String {
    production(include_str!("../api/sse.rs"))
}

/// The shared visual primitives, as one text: the small components, the three form controls, the
/// two overlay surfaces, and the registry they share.
pub(crate) fn ui_source() -> String {
    [
        include_str!("../ui/mod.rs"),
        include_str!("../ui/badge.rs"),
        include_str!("../ui/dialog.rs"),
        include_str!("../ui/gates.rs"),
        include_str!("../ui/icons.rs"),
        include_str!("../ui/modal_stack.rs"),
        include_str!("../ui/page_header.rs"),
        include_str!("../ui/search_box.rs"),
        include_str!("../ui/select.rs"),
        include_str!("../ui/sheet.rs"),
        include_str!("../ui/slider.rs"),
    ]
    .map(production)
    .concat()
}

/// The session store and everything it is built from.
pub(crate) fn auth_source() -> String {
    [
        include_str!("../auth/mod.rs"),
        include_str!("../auth/route_guard.rs"),
        include_str!("../auth/session.rs"),
        include_str!("../auth/single_flight.rs"),
        include_str!("../auth/store.rs"),
    ]
    .map(production)
    .concat()
}
