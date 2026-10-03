# Repository laws

The `repository_laws` crate: the structural engineering laws of the repository as pure checks over
a checkout — file length, sibling test placement, no exemption mechanism, the dependency
direction between the website crates, and the workspace laws (crate tiers, crate
anatomy, the strangler rule, frontend layering, Tailwind sources). The `cargo xtask verify` gates
print these results and the `api` engineering-law tests assert on them, so the two never disagree
about the tree.

## Contents

```text
tools/foundation/repository_laws/
├── Cargo.toml  the `repository_laws` library package: `verification_core`, `regex`, `thiserror`, layout tier 1
└── src/        every law, the manifest and member readers, the law roots, the error and the prelude
```

## How it works

Every law is a function over a repository root that returns its findings, or a
`verification_core::NotRun` when an input it needs is missing or unreadable, so a law that could
not look never reports a pass. Every law reads the same roots — the folder of every workspace
member the root `Cargo.toml` names, plus the pinned mod script roots — so a crate is judged from
the commit that makes it a member. The crate reads files and nothing else: no process, no network,
no environment variable. `src/README.md` lists each law, what it reads and what it finds.

## Getting started

Run these from the repository root:

```bash
cargo test -p repository_laws          # every law over planted throwaway checkouts and this repository
cargo xtask verify file-length         # the file-length law over this checkout
cargo xtask ci verify-workspace-laws   # crate tiers, crate anatomy and the other workspace laws
```

## Configuration

No feature and no environment variable.

## Public surface

- The modules `file_length`, `sibling_test_placement`, `exemption_mechanisms`,
  `crate_dependencies`, `source_roots`, `cargo_manifest`, `workspace_members` and
  `workspace_laws`; `Error` and `Result` at the crate root; `prelude` with each law's entry point.
  The [source README](/tools/foundation/repository_laws/src/README.md) lists their items.

## Boundaries

- Depends on: `verification_core` (the `NotRun` vocabulary, the scans and the patterns), `regex`
  and `thiserror`.
- Used by: `xtask` (`verify file-length`, the five workspace-law verbs,
  and the tooling tests that read the workspace members) and `api` as a dev-dependency
  (`apps/api/tests/engineering_laws.rs`).
- Rules: tier 1 of `tools/foundation`, depending only on `verification_core` among the workspace
  crates (`foundation_crates_depend_only_on_lower_foundation_crates` in
  `tools/checks/repository_checks/src/tests/tooling_dependency_boundaries.rs`; `cargo xtask verify crate-tiers`).

## Related documentation

- [Tooling foundation crates](/tools/foundation/README.md) — the three crates and their tiers.
- [Laws and gates](/documentation/restructure/laws_and_gates.md) — the crate-tier and crate-anatomy
  laws this crate implements.
- [Crate boundary rules](/documentation/standards/crate_boundary_rules.md) — the crate-level
  boundary laws.
