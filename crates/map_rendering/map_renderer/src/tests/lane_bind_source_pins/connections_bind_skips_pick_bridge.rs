//! Source-text pin: `connections_bind` uploads the connection lane and leaves the pick bridge
//! (`last_ids`, `slots_bind_soa`) alone.
//!
//! **Role:** reads the bind function bodies from the source text and asserts what they call.
//! **Position:** mounted from `lane_bind_source_pins/mod.rs` under the crate root.
//! **Signals & state:** none.
//! **Invariants:** the `include_str!` sources resolve from this folder and are exactly the files
//! that hold the pinned bodies; a missing signature fails the case rather than passing it.

const ENGINE: &str = include_str!("../../upload/hairlines.rs");

fn connections_bind_body() -> String {
    let sig = "pub fn connections_bind";
    let start = ENGINE
        .find(sig)
        .unwrap_or_else(|| panic!("T-780: missing {sig}"));
    let after = &ENGINE[start..];
    let brace = after.find('{').expect("connections_bind body");
    let mut depth = 0usize;
    let mut end = brace;
    for (i, ch) in after[brace..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = brace + i + 1;
                    break;
                }
            }
            _ => {}
        }
    }
    after[..end].to_string()
}

#[test]
fn connections_bind_body_uploads_its_lane_and_skips_the_pick_bridge() {
    let body = connections_bind_body();
    assert!(
        body.contains("MissionConnections"),
        "T-780: connections_bind must upload LaneRole::MissionConnections; body:\n{body}"
    );
    assert!(
        !body.contains("last_ids"),
        "T-780: connections_bind must not touch pick-bridge last_ids; body:\n{body}"
    );
    assert!(
        !body.contains("slots_bind_soa"),
        "T-780: connections_bind must not call slots_bind_soa; body:\n{body}"
    );
}
