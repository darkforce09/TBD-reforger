# Unified Enfusion PAK VFS (`developer-tools/src/enfusion_pak`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Unified reader and virtual filesystem for Bohemia Interactive's proprietary `.pak` game archive containers.

Eliminates code duplication between `tools/tbd-tools/src/world/pak.rs` (570 LOC) and `xtask/src/map_blueprint/pak.rs` (679 LOC).

---

## Submodules

- **`archive_reader.rs`** (<350 LOC): Parses `FORM` and `PAC1` IFF archive headers, validates checksums, and decompresses LZ4 streams.
- **`virtual_filesystem.rs`** (<300 LOC): Multi-pak layered filesystem supporting path resolution across base game and mod addons.
