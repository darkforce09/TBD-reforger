use sha2::{Digest, Sha256};

use super::*;

/// These consts ARE the source of truth for the injected payloads, so their sha256 is pinned: a
/// well-meaning "cleanup" must not silently break byte-identity with the frozen V-suite goldens
/// they serialized (`tools_v2/developer-tools/fixtures/dom_oracle/oracle-freeze/`). A payload
/// change that alters the serialized DOM requires re-pinning BOTH hashes here AND re-accepting
/// every affected golden via `gate v-suite accept`.
#[test]
fn payloads_are_pinned() {
    let sha = |s: &str| -> String {
        let mut h = Sha256::new();
        h.update(s.as_bytes());
        h.finalize().iter().map(|b| format!("{b:02x}")).collect()
    };
    assert_eq!(
        sha(FREEZE_SRC),
        "6ca42b7e360f8884a55fbc3328d1521b3ead232a86b84608d0add7f12618e305",
        "FREEZE_SRC drifted from the payload that serialized the frozen goldens"
    );
    assert_eq!(
        sha(DOM_SERIALIZER_SRC),
        "8f2ab7e44f410d83d5d5c362a54fe0fadadb7ba6e4ab7869191c2d0dedf2ee5f",
        "DOM_SERIALIZER_SRC drifted from the payload that serialized the frozen goldens"
    );
}
