# Dom Oracle

DOM route inventory and browser verification modes share the parent oracle types. Route collection,
request answering and mode execution live in separate modules.

`fixture_router.rs` decides what every intercepted request receives. It is total by design: an API
call either has a committed fixture behind it or is reported as unanswered, so no route can be
captured — or accepted — while a page renders without its data.

Source modules: `routes.rs`, `fixture_router.rs`, `run_modes.rs`.
