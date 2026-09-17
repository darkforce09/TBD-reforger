# Enfusion Tooling (`developer-tools/src/enfusion_tooling`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Reverse-engineering tools and static analysis oracles for Bohemia Interactive's Enfusion script engine (`.c`).

---

## Submodules

- **`script_indexer.rs`** (<200 LOC): Traverses Enfusion scripts and emits high-speed TSV symbol lookup tables.
- **`symbol_scanner.rs`** (<400 LOC): Lexical scanner parsing classes, methods, and component decorators.
- **`apidoc_scraper.rs`** (<300 LOC): Scrapes Bohemia's Doxygen HTML documentation to extract class members and inheritance graphs.
- **`citation_verifier.rs`** (<200 LOC): Enforces `@idx` documentation citations against scraped API indexes.
- **`capability_matrix.rs`** (<200 LOC): Validates Enfusion script engine API capabilities across version matrices.
