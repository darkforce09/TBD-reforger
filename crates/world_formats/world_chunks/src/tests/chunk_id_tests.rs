//! Unit tests of the chunk identifier: its spelling and its byte-equal serialised form.

use crate::chunk_id::ChunkId;

#[test]
fn chunk_id_of_a_cell_is_cx_underscore_cy() {
    assert_eq!(ChunkId::of_cell(18, 0), "18_0");
    assert_eq!(ChunkId::of_cell(-3, 4), "-3_4");
    assert_eq!(ChunkId::of_cell(-3, 4), ChunkId::from("-3_4"));
}

#[test]
fn chunk_id_serialises_exactly_as_its_string() {
    for id in ["18_0", "-3_4", "0_0"] {
        let typed = serde_json::to_vec(&ChunkId::from(id)).expect("serialise");
        let bare = serde_json::to_vec(&id.to_string()).expect("serialise");
        assert_eq!(typed, bare);
        let back: ChunkId = serde_json::from_slice(&bare).expect("deserialise");
        assert_eq!(back, id);
    }
}
