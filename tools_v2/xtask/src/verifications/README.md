# Verifications

Repository checks are grouped by the invariant they enforce, one folder per group. These checks return explicit failures for unreadable inputs; operational commands live in `commands/`. Engine-backed asset checks delegate to developer-tools.

Folders: `api_readiness/` (API completion evidence for every registered requirement), `architecture/` (API route tags, editor ORBAT coherency, engine layer boundaries), `ci/` (CI schema parity and workflow shell rules), `database/` (faction-library and wiki seeds, API SQL shapes), `deployment/` (staging compose paths), `documentation/` (the `readme-coverage`, `markdown-placement` and `link-check` documentation gates), `language_bans/` (tracked-language and CI shell policy), `licensing/` (upstream code and GUID leaks), `map_assets/` (map asset checks forwarded to developer-tools), `mod_scripts/` (Enfusion mod script checks), `registry/` (object-registry aliases), `schemas/` (contract checks), and `tests/` (unit tests for `property_test_configuration.rs`).

`mod.rs` registers the groups. `property_test_configuration.rs` reads the property-test seed (`PROPTEST_RNG_SEED`, or a fixed default) for `api_readiness/` and `cargo xtask db test-it`, and refuses a `PROPTEST_CASES` override, since each test suite owns its case count.

Source modules: `mod.rs`, `property_test_configuration.rs`.
