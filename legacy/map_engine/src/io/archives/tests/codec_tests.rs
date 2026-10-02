//! Role: codec tests.
//! Position: `io/archives/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::io::archives::codec::*;

#[test]
fn magic_renders_as_ascii_when_printable() {
    let e = BinaryError::BadMagic {
        what: "TBDC",
        expected: *b"TBDC",
        actual: *b"vers",
    };
    let s = e.to_string();
    assert!(s.contains("b\"TBDC\""), "{s}");
    assert!(s.contains("b\"vers\""), "{s}");
}

#[test]
fn magic_renders_as_bytes_when_not_printable() {
    let e = BinaryError::BadMagic {
        what: "TBDE",
        expected: *b"TBDE",
        actual: [0, 1, 2, 3],
    };
    assert!(e.to_string().contains("[0, 1, 2, 3]"), "{e}");
}

#[test]
fn every_variant_renders_and_is_an_error() {
    let cases = [
        BinaryError::Truncated {
            what: "TBDC",
            expected: 32,
            actual: 4,
        },
        BinaryError::UnsupportedVersion {
            what: "TBDC",
            expected: 1,
            actual: 9,
        },
        BinaryError::Misaligned {
            what: "ObjectInstancePod",
            align: 4,
        },
        BinaryError::LengthMismatch {
            what: "TBDC",
            expected: 64,
            actual: 63,
        },
        BinaryError::Archive {
            what: "RoadNetworkArchive",
            cause: "boom".to_string(),
        },
    ];
    for c in &cases {
        let s = c.to_string();
        assert!(!s.is_empty(), "{c:?} rendered empty");
        let _: &dyn core::error::Error = c;
    }
}
