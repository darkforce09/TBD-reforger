**Status:** live

# Shared agent brief

The brief every implementing agent of the restructure program reads first; each stage's agent
document points here. `<scratch>` is a scratch folder outside the repository that the
orchestrator names when it launches the agent.

You are one implementing agent of the workspace restructure program of the TBD Reforger monorepo.
The program documents are in `documentation/restructure/` (README,
program_plan.md, target_file_tree.md, crate_catalogue.md, laws_and_gates.md, progress.md). Read
only the parts your prompt names; do not re-derive the plan.

## Rules
- The repository checkout, branch `main`. Never create
  branches, commit, stage, stash, reset, clean, checkout or restore. The orchestrator commits.
- Read `CLAUDE.md` §1 (laws) once. Laws 3, 4, 7, 8 and 10 bind you: clean architecture, self-describing
  names, production files ≤ 500 lines and test files ≤ 1000 lines (sibling `tests/` files via
  `#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`, never inline test modules), module headers
  with Role / Position / Signals & state / Invariants, `///` docs on public items, present-tense
  comments with no history and no ticket ids, and the README.md of every folder whose contents
  change updated in the same change (its Contents block must match the folder's tracked children).
- Edit only the files your prompt owns. Four agents (T1–T5 minus you) work in the same checkout at
  the same time. Files listed in `<scratch>/foreign_baseline.sha256` belong to nobody in this
  wave and are never edited.
- Shared registration files: add only your own lines, re-read the file right before each edit,
  never reorder or reformat other lines. This wave's shared files are:
  `tools/xtask/src/cli/mod.rs`, `tools/xtask/src/cli/dispatch.rs`,
  `tools/xtask/src/commands/mod.rs`, `tools/xtask/src/commands/README.md`,
  `tools/xtask/src/commands/ci/task_definitions.rs`, `.github/workflows/ci.yml`,
  `tools/xtask/src/commands/build/recipes.rs` (T3 only), `tools/xtask/README.md`.
  Cargo manifests and `Cargo.lock` belong to T2 alone; if you need a dependency added, do not edit a
  manifest: report it as a FIX finding and work without it (or with an existing dependency).
- Keep every crate compiling at every save point: write complete new files first, add the `mod`
  line last.
- Tests are never weakened, skipped, ignored, deleted or given a looser tolerance. Missing
  infrastructure is a failure with its cause, never a pass. Run each new test suite twice.
- Every new check (law, gate, verify mode) gets a perturbation proof: save the target file's
  sha256 to `<scratch>/perturb/<ID>-<name>.sha256`, plant one deliberate defect, run the
  narrowest check, record which cases go red, restore the file, prove it byte-equal with
  `sha256sum -c`.
- Commands: `source <scratch>/env.sh` first in every shell call when the orchestrator provides one
  (machine-specific settings such as disk-saving build flags or a Chromium path). One build, test or gate per shell call,
  output to `<scratch>/logs/<ID>-<step>.log`, read back through `grep` / `tail` of the result
  lines. A build waiting on the cargo lock is normal; never start a second one in parallel.
- Formatting: `cargo fmt` only with `--check` on the workspace; to format your own leaf files use
  `rustfmt --edition 2024 <file>`. Never format a file that declares child modules (`mod.rs`,
  `lib.rs`, `cli/mod.rs`), since that rewrites other agents' lines.
- Throwaway probes live in `<scratch>/probes/<ID>/`, never in the repository.
- Before the report: `sha256sum -c <scratch>/foreign_baseline.sha256 --quiet`;
  `git status --short` with every file you changed listed in the report;
  `cargo xtask ci verify-editorconfig` once (it runs editorconfig-checker over the repository).
- Findings are triaged FIX (small, in a file you own, or turns a check red), NOTE (needs an
  operator decision, is its own refactor, or sits in a file you do not own), or CLOSE (not a
  defect; cite evidence).

## Efficiency (mandatory)
- Read only what the prompt names, and only the line ranges you need: search first (`grep -n`,
  capped with `| head`), then read with an offset and limit. Never reread a file you just edited.
- Write whole files in one write; batch independent edits.
- Develop against the narrowest loop: `cargo test -p <package> <name filter>`. Run the package's
  full suite and `cargo clippy -p <package> --all-targets -- -D warnings` once at the end. Never run
  workspace-wide suites or `cargo xtask ci ci-local`; those are the orchestrator's gates.
- Stop at your budget line and report what is done and not done.

## Spec (authoritative, verified)
- Packages today: `xtask` (tools/xtask), `verification_core` (tools/verification_core),
  `ticket_engine`, `developer_tools`, `api` (apps/api), `frontend`,
  `map_engine`, `graphics_engine`, `offline_service_worker`,
  `fleet_host_agent`, `ticketboard`. Rust 1.95.0, edition 2024 except the frontend (2021).
- `<scratch>` = `<scratch>`.
- Documents: a live doc starts with `**Status:** live`, stays ≤ 500 lines, uses repository-root
  links (`/documentation/...`), and a backticked path under an existing top-level folder must
  exist; a `cargo xtask …` command written in any doc must exist in the CLI. Terms follow the
  glossary (the editor is the Mission Creator; the authored document is a mission).
- The planned laws, gate set and end-state checks are in
  `documentation/restructure/laws_and_gates.md`; the end-state tree in `target_file_tree.md`.

## Report format (≤ 250 words, these numbered sections)
1 files created or changed; 2 commands run, exit codes, counts, log paths; 3 tests added (name
prefix, count, run twice); 4 perturbations (defect, red cases, restored); 5 findings (id,
file:line, FIX/NOTE/CLOSE, action); 6 not run or deviations, with the reason.
