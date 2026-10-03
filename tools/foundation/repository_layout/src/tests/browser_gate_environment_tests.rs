use super::*;
use crate::find_repository_root;

/// The pin file is committed, so it exists in a real checkout.
#[test]
fn the_browser_gate_pin_file_exists_in_the_checkout() {
    let root = find_repository_root().expect("active checkout");
    let pins = root.join(BROWSER_GATE_ENVIRONMENT);
    assert!(pins.is_file(), "not a file: {}", pins.display());
}
