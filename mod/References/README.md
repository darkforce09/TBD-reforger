# Upstream reference lanes

The licensed upstream code the mod is designed against, kept beside the addons so tools and agents
can read it, and kept out of git, out of every deploy and out of the shipped addons. Only this
README is tracked; each lane is a gitignored folder filled on the machine that needs it.

## Contents

```text
mod/References/
```

## How it works

Three lanes, each a gitignored folder next to this README:

| Lane | Holds | Licence | How it is filled |
| --- | --- | --- | --- |
| `crf_framework/` | the Coalition Reforger Framework scripts and assets | Arma Public License: read, cite and design-mirror, never copy | copied from a Coalition Reforger Framework checkout |
| `vanilla_reference/` | vanilla Arma Reforger scripts and the official Script API pages | Bohemia Interactive copyright: read only | the fetch and `enf` commands below |
| `playable_selector/` | a PlayableSelector checkout | none at all, so default copyright: design-mirror only, no copying | copied from a PlayableSelector checkout |

The vanilla lane is written by commands, each into its own folder of the lane:

| Command | Writes |
| --- | --- |
| `cargo xtask fetch vanilla-api` | `apidoc/`, the Script API pages |
| `cargo xtask fetch vanilla-source` | `source_html/`, the source pages |
| `cargo run -q -p developer_tools --bin enf -- extract` | `Scripts/`, scripts read by name from the game paks |
| `cargo run -q -p developer_tools --bin enf -- source` | `Source/`, `.c` files rebuilt from the source pages |
| `cargo run -q -p developer_tools --bin enf -- carve --game <install>` | `Carved/`, script text scanned out of the paks |

Every one of them refuses to run when this folder is missing, and the `enf` commands refuse an
output folder outside it. `enf extract` and `enf carve` replace a previous output only when given
`--replace`.

A slice worktree, made by the ticket manager's runner, links each lane in from the main checkout
as a symlink, and refuses when any lane is missing. `TBD_PS_ORACLE`, when set and
not empty, names another PlayableSelector checkout in place of `playable_selector/`, for the slice
worktree and for the leak check alike.

## Boundaries

- Depends on: the upstream checkouts and the game install the lanes are filled from.
- Used by: `cargo xtask verify no-crf-leak` (the CRF and PlayableSelector lanes), the `enf` index
  and lookup commands (`enf index crf`, `enf apidoc`), slice worktrees, and anyone reading
  upstream code to check an Enfusion API.
- Rules:
  - Nothing in a lane is ever committed: the root `.gitignore` ignores everything in this folder
    but this README, with no trailing slash so a worktree's lane symlinks are ignored too.
  - Nothing here is ever deployed: `cargo xtask deploy website` and `cargo xtask deploy staging`
    exclude the whole folder (pinned by `rsync_excludes_the_licensed_reference_lanes` and
    `rsync_argv_keeps_every_exclude_in_order`).
  - No lane code or lane-only asset GUID enters a shipped addon: `cargo xtask verify no-crf-leak`
    checks it, and exits 2 (did not run) when the CRF or PlayableSelector lane is missing or holds
    no `UI/` or `Prefabs/` folder.

## Related documentation

- [Enfusion script oracle](/documentation/tools/enfusion/enfusion_script_index.md): the
  `enf` indexes built from the lanes.
