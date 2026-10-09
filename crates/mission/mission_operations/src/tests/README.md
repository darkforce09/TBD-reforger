# tests

Native regression cases for the document operations that keep session state of their own: the
installed cargo defaults, the loadout buffer and its Apply seed, and the tactical-graphics draw
machine. `cargo.rs` and `tactical_graphics.rs` are siblings of the production modules they prove,
wired by `#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`.
