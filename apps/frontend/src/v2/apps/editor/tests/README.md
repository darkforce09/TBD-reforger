# Editor Page Tests (`v2/apps/editor/tests`)

Sibling test files mounted at the bottom of `mission_editor.rs` with
`#[cfg(test)] #[path = "tests/<file>.rs"] mod ...;` — behavioural checks over the page's pure
helpers, and source pins that scrub a production file and assert on the code that ships.

**Depended on by:** nothing. Test files are mounted, never imported.

**Boundary:** a source pin names its subject through
`include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/..."))`, so moving a file repoints one
string instead of breaking a relative path. When a pin's subject moves, the pin follows it; a pin
is never deleted or loosened to make a suite pass.
