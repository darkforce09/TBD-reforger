//! Source-text pin: `comments_bind` uploads the comment lane and leaves the pick bridge
//! (`last_ids`, `slots_bind_soa`) alone.
//!
//! **Role:** reads the bind function bodies from the source text and asserts what they call.
//! **Position:** mounted from `lane_bind_source_pins/mod.rs` under the crate root.
//! **Signals & state:** none.
//! **Invariants:** the `include_str!` sources resolve from this folder and are exactly the files
//! that hold the pinned bodies; a missing signature fails the case rather than passing it.

const ENGINE: &str = include_str!(
    "../../../../../../crates/map_rendering/symbology_layers_gpu/src/slot_symbology/mission_lanes.rs"
);

fn comments_bind_body() -> String {
    let sig = "pub fn comments_bind";
    let start = ENGINE
        .find(sig)
        .unwrap_or_else(|| panic!("T-748: missing {sig}"));
    let after = &ENGINE[start..];
    let brace = after.find('{').expect("comments_bind body");
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
fn comments_bind_body_does_not_touch_last_ids() {
    let body = comments_bind_body();
    assert!(
        !body.contains("last_ids"),
        "T-748: comments_bind must not touch pick-bridge last_ids; body:\n{body}"
    );
    assert!(
        body.contains("MissionComments"),
        "T-748: comments_bind must upload LaneRole::MissionComments; body:\n{body}"
    );
    assert!(
        !body.contains("slots_bind_soa"),
        "T-748: comments_bind must not call slots_bind_soa; body:\n{body}"
    );
}
