use sha2::{Digest, Sha256};

use super::*;

/// T-171: the Node driver these payloads were byte-copied from is gone (T-165.6), so the
/// consts ARE the source of truth — pin their sha256 so a well-meaning "cleanup" can't
/// silently break byte-identity with the frozen V-suite goldens they serialized
/// (tools_v2/developer-tools/fixtures/t159/oracle-freeze/). An intentional payload change requires
/// re-pinning BOTH hashes here and re-accepting every affected golden via
/// `gate v-suite accept`.
#[test]
fn payloads_are_pinned() {
    let sha = |s: &str| -> String {
        let mut h = Sha256::new();
        h.update(s.as_bytes());
        h.finalize().iter().map(|b| format!("{b:02x}")).collect()
    };
    assert_eq!(
        sha(FREEZE_SRC),
        "316d38c6fa6dad0d1e458e5ec300b08f2ea40094a75b27f4a3424c4d5604b1bd",
        "FREEZE_SRC drifted from the payload that serialized the frozen goldens"
    );
    assert_eq!(
        sha(DOM_SERIALIZER_SRC),
        "231a51b88dcf92bf372526fdd09e335126401bd5f6b958f71bd25869e837ea48",
        "DOM_SERIALIZER_SRC drifted from the payload that serialized the frozen goldens"
    );
}
