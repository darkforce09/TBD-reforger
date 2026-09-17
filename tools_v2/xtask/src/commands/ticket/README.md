# Ticket Command Forwarder (`xtask/src/commands/ticket`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Thin CLI adapter forwarding `cargo xtask ticket <verb>` commands directly to `ticket-engine::cli`.

Contains zero business logic or TOML serialization.
