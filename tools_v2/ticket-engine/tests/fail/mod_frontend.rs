use ticket_engine::Domain;

fn main() {
    // Must not compile: `frontend` is a LAYER in the scope vocabulary, never a
    // domain. The Domain enum is closed; adding `Frontend` to it would make this
    // fixture compile, and trybuild fails when a compile-fail fixture compiles.
    // The refusal is the pin.
    let _ = Domain::Frontend;
}
