# 3B-K — `arsenal/` and `ui/arsenal/`

Read `00_rules_every_agent_obeys.md` first. Every rule there applies to this brief.

CLAUDE.md's atlas gives the arsenal its own top-level folder — "loadout forms, gear catalog, 3D
paper doll UI" — and separately names `ui/arsenal/` for the panel surface. The split follows what
the files already are.

## The move

| from `v2/apps/editor/arsenal/` | to |
|---|---|
| `mod.rs` | `arsenal/mod.rs` |
| `asset_catalog.rs` | `arsenal/asset_catalog.rs` |
| `loadout.rs` | `arsenal/loadout.rs` |
| `loadout_commands.rs` | `arsenal/loadout_commands.rs` |
| `arsenal_rules.rs` | `arsenal/rules.rs` |
| `arsenal_doll.rs` | `arsenal/doll.rs` |
| `panels.rs` | `ui/arsenal/panels.rs` |

`arsenal_rules` and `arsenal_doll` lose the stutter inside `arsenal/` — Law 4. `ui/arsenal/mod.rs`
declares the panel surface; `ui/mod.rs` gains `arsenal`.

Nothing is split: `asset_catalog.rs` (3,052), `loadout.rs` (2,973), `rules.rs` (2,959) and
`mod.rs` (1,788) carry allowlist rows and are Phase 3C's subject.

## Pins

`arsenal/mod.rs` holds 30 pins, ten of them cross-file — map-engine `hosted_commands/slot_loadouts`
and `slot_attributes` among them — and `loadout_commands.rs` is pinned from elsewhere in the tree,
including `xtask/src/gate_t180.rs`, whose tests `fs::copy(...).unwrap()` every row. Repoint the
gate's row, repoint the anchored suffixes of anything you moved, leave self-pins alone, and grep
`xtask/` and `tools/` before you finish.

## What travels, what to repoint

Sibling test files with their bottom-of-file declarations. Allowlist rows updated in the same
commit. `//!` headers rewritten where they name an old path. Every
`crate::v2::apps::editor::arsenal::…` call site swept — the rename of two modules means some of
those change spelling, so sweep by symbol, not only by prefix. `mission_editor.rs`'s re-exports and
mounts repointed, never deleted.

## Verification — run once, at the end, from the repo root

```
CARGO_TARGET_DIR=target-container cargo test -p website-frontend
CARGO_TARGET_DIR=target-container cargo check --target wasm32-unknown-unknown -p website-frontend
CARGO_TARGET_DIR=target-container cargo fmt --all -- --check
CARGO_TARGET_DIR=target-container cargo test -p xtask
```

Expected: frontend pass count unchanged, 0 failed; wasm32 clean; fmt silent; `cargo test -p xtask`
failing only the eight known names. Paste all four verbatim plus `git status --porcelain` for your
staged paths.

Commit directly to `main`:

```
refactor(engine-split): the arsenal splits its domain from its panel surface (3B)
```
