# Repository refactor commands

The `cargo xtask refactor` group: tools that reshape the checkout itself. Its one command,
`relocate`, moves tracked files and folders and rewrites every reference to them from a manifest,
so no path is ever rewritten by hand, and proves afterwards that no retired spelling is left in a
live file.

## Contents

```text
tools/xtask/src/commands/refactor/
├── cli.rs       the `RefactorCmd` clap enum and the `relocate` flags
├── dispatch.rs  finds the checkout root, checks the flags, runs the chosen mode
├── mod.rs       the module tree
└── relocate/    the manifest, the plan, the rewrite passes, the moves and the verification
```

## How it works

`dispatch.rs` resolves the checkout root from the working directory and hands the mode to
`relocate/`: `--dry-run` and `--apply` need `--manifest`; `--verify` takes it optionally and
otherwise judges every committed manifest. All three return the process exit code; the work, the
file formats and the rules live in the `relocate/` README.

## Commands

### refactor relocate

- Synopsis: `cargo xtask refactor relocate --manifest <path.tsv> (--dry-run | --apply)` and
  `cargo xtask refactor relocate --verify [--manifest <path.tsv>]`
- Does: `--dry-run` prints, per manifest row, the tracked files a `path` row moves and the
  references each row rewrites by file kind, then every unresolved literal as `path:line`, then
  runs the `--verify` checks over the tree the plan would leave (in memory) and lists every
  retired spelling it would keep as `path:line` at the file's new path; it writes nothing.
  `--apply` prints the same plan and runs the same two checks, refuses the plan while either finds
  anything, makes the `git mv` moves, writes every rewritten file, then verifies the manifest on
  the checkout. `--verify` checks that no live tracked file spells a `path` row's retired `from`
  and that no `rust_path` prefix is left in its scope; with no `--manifest` it judges every `.tsv`
  in `documentation/restructure/manifests/` except the format sample `example.tsv`.
- Exit codes: 0 clean (dry run: nothing unresolved and the planned tree verifies clean; apply and
  verify: every row held); 1 unresolved references or retired spellings in the planned tree (dry
  run, apply: nothing written) or retired spellings in the checkout (verify, apply); 2 did not
  run: an invalid manifest, a move the tree cannot take (a `from` that is not tracked, a `to` that
  exists), an unreadable checkout or git failure, or a missing manifests folder. A failed move or
  write during `--apply` undoes what was done and exits 2.
- Example: `cargo xtask refactor relocate --manifest documentation/restructure/manifests/example.tsv --dry-run`

## Boundaries

- Depends on: clap; `find_repo_root` in `tools/xtask/src/core/repository_root.rs`; the
  `relocate/` modules.
- Used by: `tools/xtask/src/cli/dispatch.rs`, which routes the `refactor` group; the
  restructure program's stages, which run every move through it.
- Rules: the group's flags stay in `cli.rs`, and exactly one mode flag is accepted per run (clap's
  required `mode` group); a mode never writes outside `--apply`, and `--apply` writes only a plan
  whose dry run would exit 0.

## Related documentation

- [Relocation manifests](/documentation/restructure/manifests/README.md) — the manifest format
  and where the stage manifests live.
- [Laws and gates](/documentation/restructure/laws_and_gates.md) — the relocation law the
  verification enforces between stages.
