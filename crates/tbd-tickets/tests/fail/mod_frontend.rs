use tbd_tickets::ModLayer;

fn main() {
    // Must not compile: Mod has no frontend layer. If someone adds `Frontend` to
    // ModLayer, this fixture starts compiling and trybuild fails.
    let _ = ModLayer::Frontend;
}
