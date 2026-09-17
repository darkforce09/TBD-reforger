# 3B-A — anchor every cross-file source pin

Read `.ai/artifacts/engine_split_phase3b/00_rules_every_agent_obeys.md` first. Every rule there
applies to this brief.

## Why this is the first brief

Phase 3B moves 110 frontend files. The frontend crate pins behaviour by source inspection —
`include_str!` over another file's text, then a scrub and an assertion — and **537 of those pins
live in `editor/` alone**. A pin written as a relative path (`include_str!("../canvas/commands.rs")`,
`include_str!("../../../../map-engine/src/...")`) is a function of how deep its holder sits in the
tree. Move either end and it breaks, silently at first and then as a wall of compile errors.

This brief removes that coupling **before anything moves**, so every later brief edits module
declarations and `use` paths only.

## The idiom — already live in this crate

`apps/website/frontend/src/v2/core/test_support/editor_operations.rs` already does exactly this:

```rust
include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/editor/state/armed_placement/mod.rs"
))
```

`CARGO_MANIFEST_DIR` is `apps/website/frontend`, so the suffixes are:

| pin target | anchored suffix |
|---|---|
| anything under the frontend's `src/` | `"/src/..."` |
| `website-map-engine` sources | `"/../map-engine/src/..."` |
| the frontend's own test fixtures | `"/tests/fixtures/..."` |
| `packages/tbd-schema/schema/*.json` | `"/../../../packages/tbd-schema/schema/..."` |
| `apps/mod/tbd-framework/Data/registry.json` | `"/../../../apps/mod/tbd-framework/Data/registry.json"` |

## Scope — exactly this, nothing else

**254 cross-file `include_str!` sites across 53 files.** A *cross-file* pin is one whose resolved
target is a file other than the file holding it. Derive the list yourself with this script, run
from `apps/website/frontend`:

```python
import re, os, collections
c = collections.Counter()
for dp, _, fs in os.walk('src'):
    for f in fs:
        if not f.endswith('.rs'):
            continue
        p = os.path.join(dp, f)
        for m in re.finditer(r'include_str!\("([^"]+)"\)', open(p, encoding='utf8').read()):
            if os.path.normpath(os.path.join(dp, m.group(1))) != os.path.normpath(p):
                c[p] += 1
for k, v in sorted(c.items()):
    print(f"{v:4d}  {k}")
print("TOTAL", sum(c.values()), "sites in", len(c), "files")
```

Densest holders: `mission_editor_tests/t628_boot_progress.rs` 42, `t629_satellite_resolution.rs` 18,
`panels/help_modal.rs` 14, `t649_select_all_and_multi_edit.rs` 12, `arsenal/mod.rs` 10,
`t780_connection_line.rs` 9, `panels/dock_right.rs` 8, `t647`/`t648`/`t784` 8 each.

**Two exclusions, both deliberate:**

1. **Self-pins stay exactly as they are.** 201 sites in `editor/` are a file pinning its own text
   (`include_str!("dock_right.rs")` inside `dock_right.rs`). A file always travels with itself, so
   these are already move-proof. Rewriting them would be 201 lines of churn that buys nothing.
2. **`src/v2/core/test_support/pins.rs` is out of scope** (112 sites). Neither it nor anything it
   pins moves in Phase 3B — every one of its targets is under `v2/core/` or `v2/pages/`. Leave it.

## What to do

For each of the 254 sites, replace the relative-path literal with the anchored `concat!` form
naming the same file. The text `include_str!` yields must be byte-identical afterwards — this is a
pure addressing change, so **no assertion, scrub, census row or test name changes**.

Watch for these shapes, all of which occur:

- a bare `let src = include_str!("../mission_editor.rs");`
- a `const SRC: &str = include_str!("...");`
- a pin inside a tuple table (`("commands.rs", include_str!("../canvas/commands.rs"), 1)` in
  `help_modal.rs`'s keymap census)
- a pin already inside a `concat!` of several files

`rustfmt` will reflow the multi-line `concat!` — run fmt and commit its result, do not hand-wrap.

**Do not move, rename, split or delete any file in this brief.** `git status` at the end must show
only modified `.rs` files, no adds and no deletes.

## Verification — run once, at the end, from the repo root

```
CARGO_TARGET_DIR=target-container cargo test -p website-frontend
CARGO_TARGET_DIR=target-container cargo check --target wasm32-unknown-unknown -p website-frontend
CARGO_TARGET_DIR=target-container cargo fmt --all -- --check
```

Expected: `website-frontend` >= 1317 passed, 0 failed (it was 1317 at HEAD `9ae1eedf0`; the count
must not drop — a pin that stopped being compiled is a pin you deleted). wasm32 check clean.
`fmt --check` silent.

Then re-run the detection script and paste its output: the TOTAL must be **112 sites in 1 file**
(`pins.rs` alone). Anything else means you missed a site.

This is the first cargo invocation since `target-container/` was cleaned after 3A, so the build is
a full one and will take a while. That is expected — do not interpret it as a hang.

## Report

Paste all four outputs verbatim. State the before/after site counts. Name any file where the
anchored form could not be used and why (there should be none).

Commit directly to `main`:

```
refactor(engine-split): source pins address their subject from the crate root (3B)
```
