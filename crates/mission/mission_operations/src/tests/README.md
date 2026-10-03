# tests

Native regression cases for the document operations that keep session state of their own: the
installed cargo defaults, the loadout buffer and its Apply seed, and the tactical-graphics draw
machine. `cargo.rs` and `tactical_graphics.rs` are siblings of the production modules they prove,
wired by `#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`.

Three files are mounted from the crate root: `prelude_surface.rs` proves the entity authoring
commands and the session state are reachable through the prelude alone; `identity_source_scrub.rs`
scrubs `entity/identity.rs` and proves `slot_attrs_exists` answers from the raw slot map, never from
a materialised view; `source_scrub.rs` is the scrub it reads the source through, blanking comments
and string literals character for character.
