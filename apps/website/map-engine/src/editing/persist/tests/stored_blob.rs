//! Role: what the content verdict says about an empty, a corrupt, and an authored blob.
//! Position: `editing/persist/tests` in the map engine.
//! Signals & state: documents built in the test body; nothing shared between tests.
//! Invariants: the verdict is about CONTENT. The empty-document case is the one a byte length gets
//! wrong, so it is asserted against the real encode rather than against a hand-written constant.

use super::*;

fn authored_document() -> MissionDocCore {
    let core = MissionDocCore::new();
    core.set_origin_init(true);
    core.seed_random(8, 12_800.0, 12_800.0, 42);
    core.set_origin_init(false);
    core
}

#[test]
fn nothing_at_all_holds_no_content() {
    assert!(!restores_to_authored_content(&[]));
}

/// The crux: an empty document does NOT encode to zero bytes, so a byte-length test would wave its
/// blob straight onto a record holding real work.
#[test]
fn an_empty_document_encodes_to_non_empty_bytes_and_still_holds_no_content() {
    let bytes = MissionDocCore::new().encode_state();
    assert!(
        !bytes.is_empty(),
        "an empty document encodes to a non-empty stream; a length test cannot see this"
    );
    assert!(!restores_to_authored_content(&bytes));
}

#[test]
fn an_authored_document_holds_content() {
    let bytes = authored_document().encode_state();
    assert!(restores_to_authored_content(&bytes));
}

/// A blob that cannot be replayed is not a backup, whatever its length.
#[test]
fn a_corrupt_or_truncated_blob_holds_no_content() {
    let bytes = authored_document().encode_state();
    let truncated = &bytes[..bytes.len() / 2];
    assert!(
        !restores_to_authored_content(truncated),
        "a half-written blob must not pass as a backup"
    );
    // An unterminated var-int run: every byte sets the continuation bit, so the header never ends.
    assert!(!restores_to_authored_content(&[0x80; 16]));
    assert!(!restores_to_authored_content(&[0xff, 0xfe, 0xfd, 0xfc]));
}

/// The verdict survives a round trip through a fresh peer, which is what a restore actually is.
#[test]
fn a_replayed_authored_document_still_holds_content() {
    let bytes = authored_document().encode_state();
    let peer = MissionDocCore::new();
    peer.set_origin_init(true);
    peer.apply_update(&bytes).expect("a peer can replay it");
    peer.set_origin_init(false);
    assert!(peer.has_content());
    assert!(restores_to_authored_content(&peer.encode_state()));
}
