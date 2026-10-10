# Reference symbol index

The symbol tables of the Enfusion reference lanes under `mod/References/`: one tab-separated table
per lane and kind (files, symbols, `modded` classes, replicated properties, Script API classes and
members), plus the capability matrix. They hold names and coordinates only, never script bodies,
so they are committed while the licensed lanes they index are not; a checkout without the lanes
still answers `enf lookup`, `enf dirs` and `enf capability`.

## Contents

```text
mod/reference_symbol_index/
├── capability_matrix.tsv      the upstream framework's files joined with their capability verdicts
├── crf_files.tsv              Coalition Reforger Framework: one row per script file
├── crf_modded.tsv             its `modded class` declarations
├── crf_rplprops.tsv           its replicated properties
├── crf_symbols.tsv            its classes, methods and fields (the default table of `enf lookup`)
├── vanilla_api_classes.tsv    the vanilla Script API pages: one row per class
├── vanilla_api_members.tsv    their members
├── vanilla_files.tsv          the vanilla scripts: one row per file
├── vanilla_modded.tsv         their `modded class` declarations
├── vanilla_rplprops.tsv       their replicated properties
└── vanilla_symbols.tsv        their classes, methods and fields
```

## How it works

`enf index <lane> --root <lane folder> --out mod/reference_symbol_index` rewrites a lane's tables
from the lane under `mod/References/`; `enf apidoc` rewrites the Script API tables. The `enf`
binary (`tools/developer_tools`, over `tools/enfusion/enfusion_script_index`) reads them as its
argument defaults.

## Boundaries

- Written and read by: `tools/enfusion/enfusion_script_index` (`ENF_INDEX_DIR`).
- Built from: `mod/References/` (gitignored, see its README).
