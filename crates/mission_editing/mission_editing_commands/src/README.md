# Mission editing commands source

The source of `mission_editing_commands`: the commands that run against the installed editing
host, the pure document texts, the crate's error and the crate root.

## Contents

```text
crates/mission_editing/mission_editing_commands/src/
├── document_text/    export bytes, compile summary, merge and save reports, selection digests (pure)
├── error.rs          `Error` and `Result`: why a faction apply or a compiled export refuses
├── hosted_commands/  commands that open the hosted document, commit one transaction and run the tail
├── lib.rs            the crate root: module header, `mod` lines and the error re-export
└── prelude.rs        the names most readers import
```

## How it works

`hosted_commands/` reaches the document only through `mission_editing_session::host` and ends
every edit in `mission_editing_session::history::after_local_edit`; `document_text/` names no
host and no session, and is pure over the strings and rows it is handed. `error.rs` carries the
two refusals a caller renders verbatim.

## Boundaries

- Depends on: the crates named in the [crate README](../README.md).
- Used by: the crate's callers through `lib.rs` and `prelude.rs`.
- Rules: every hosted command opens one document borrow and drops it before the tail runs;
  nothing here touches a browser or a GPU.
