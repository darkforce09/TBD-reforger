# Mission Creator workspace tests

Sibling test files of the editor page, mounted through `mod.rs` at the bottom of
`mission_editor.rs` with `#[cfg(test)] #[path = "tests/mod.rs"] mod tests;`. They test the pure
decisions the page and its lower crates make: boot progress, the satellite level, the transform
quantiser, the click router, the comment, connection and marker lanes, the hover cursor, the
selection universe and the registry session.

**Depended on by:** nothing. Test files are mounted, never imported.
