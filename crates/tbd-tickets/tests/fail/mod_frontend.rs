use tbd_tickets::Domain;

fn main() {
    // Must not compile: `frontend` is a LAYER (vocabulary data), never a domain. If
    // someone adds `Frontend` to the closed Domain enum, this fixture starts
    // compiling and trybuild fails — the T-917.2 successor of the old
    // `ModLayer::Frontend` pin.
    let _ = Domain::Frontend;
}
