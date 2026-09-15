//! Role: chunk math tests.
//! Position: `streaming/scheduler/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::streaming::scheduler::chunk_math::*;

const EVERON: TerrainSizeM = TerrainSizeM {
    width: 12800.0,
    height: 12800.0,
};
const CHUNK: f64 = 512.0;

#[test]
fn chunk_rect_pinned_cases() {
    assert_eq!(
        chunk_rect_for_bbox([0.0, 0.0, 511.9, 511.9], EVERON, CHUNK),
        ChunkRect {
            cx0: 0,
            cy0: 0,
            cx1: 0,
            cy1: 0
        }
    );
    assert_eq!(
        chunk_rect_for_bbox([512.0, 512.0, 1024.0, 1024.0], EVERON, CHUNK),
        ChunkRect {
            cx0: 1,
            cy0: 1,
            cx1: 2,
            cy1: 2
        }
    );

    assert_eq!(
        chunk_rect_for_bbox([12799.0, 12799.0, 99999.0, 99999.0], EVERON, CHUNK),
        ChunkRect {
            cx0: 24,
            cy0: 24,
            cx1: 24,
            cy1: 24
        }
    );
}

#[test]
fn preload_margin_pinned_cases() {
    assert_eq!(preload_margin_m([0.0, 0.0, 2000.0, 2000.0], CHUNK), 512.0);
    assert_eq!(preload_margin_m([0.0, 0.0, 12800.0, 12800.0], CHUNK), 640.0);
}

#[test]
fn viewport_ids_length_and_order() {
    let ids = chunk_ids_for_viewport([1024.0, 1024.0, 1536.0, 1536.0], EVERON, CHUNK, 0);
    assert_eq!(ids.len(), 16);
    assert_eq!(ids[0], "1_1");
    assert_eq!(ids[1], "2_1");
    assert_eq!(ids[4], "1_2");
    assert_eq!(ids[15], "4_4");
}

#[test]
fn oversized_ring_expands_rect() {
    let rect = chunk_rect_for_bbox([2048.0, 2048.0, 2048.0, 2048.0], EVERON, CHUNK);
    assert_eq!(
        rect,
        ChunkRect {
            cx0: 4,
            cy0: 4,
            cx1: 4,
            cy1: 4
        }
    );
    let ringed = expand_chunk_rect(rect, 1, EVERON, CHUNK);
    assert_eq!(
        ringed,
        ChunkRect {
            cx0: 3,
            cy0: 3,
            cx1: 5,
            cy1: 5
        }
    );
}

#[test]
fn ids_for_rect_row_major() {
    let ids = chunk_ids_for_rect(ChunkRect {
        cx0: 0,
        cy0: 0,
        cx1: 1,
        cy1: 1,
    });
    assert_eq!(ids, vec!["0_0", "1_0", "0_1", "1_1"]);
}
