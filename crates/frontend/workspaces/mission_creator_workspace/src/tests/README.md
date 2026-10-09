# Mission Creator workspace tests

Sibling test files mounted at the bottom of `mission_editor.rs` with
`#[cfg(test)] #[path = "tests/<file>.rs"] mod ...;` — behavioural checks over the page's pure
helpers, and source pins that scrub a production file and assert on the code that ships; plus
`test_support.rs`, the unit tests of the editor's source-pin support (`../../../../../../crates/frontend/workspaces/mission_creator_engine_bridge/src/test_support/mod.rs`). `review_mode/read_only_review.rs` holds the review mode's
workspace-wide pins: every write path of the editor, from the session to the docks and the page,
consults `state::review_mode`. `frontend_source_roots.rs` names the frontend source trees the
whole-frontend source pins walk: the `src` of every frontend crate, the app's in `shell/` among
them, each package once (`the_whole_frontend_walk_reads_every_package_once`).

**Depended on by:** nothing. Test files are mounted, never imported.

**Boundary:** a source pin names its subject through
`include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/..."))`, so moving a file repoints one
string instead of breaking a relative path. When a pin's subject moves, the pin follows it; a pin
is never deleted or loosened to make a suite pass.
