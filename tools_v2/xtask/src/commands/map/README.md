# Map Command Forwarder (`xtask/src/commands/map`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Thin CLI adapter forwarding map imagery and 3D blueprint compilation tasks directly to `developer-tools`.

Eliminates the need for `xtask` to depend on heavy 3D mesh rendering libraries.
