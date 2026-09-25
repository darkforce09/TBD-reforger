**Status:** live

# Enfusion script oracle

The mechanically generated answer to "does this [Enfusion](/documentation_v2/glossary.md#enfusion)
symbol exist, and where?". The `enf` binary indexes the upstream framework's scripts and the
vanilla game scripts into committed TSV tables, answers lookups against them, and fails a
document that cites a symbol the tables do not hold. Mod developers and the AI agents that write
[mod](/documentation_v2/glossary.md#mod) code and its documentation rely on it instead of memory.

## Where it lives

- Code: [`tools_v2/developer-tools/src/enfusion_tooling/`](/tools_v2/developer-tools/src/enfusion_tooling/README.md)
  (the indexer, the lookups and the two checks) and
  [`tools_v2/developer-tools/src/enfusion_pak/`](/tools_v2/developer-tools/src/enfusion_pak/README.md)
  (the game archive reader behind `enf extract`).
- Entry: `cargo run -q -p developer-tools --bin enf -- <command>`; the ten commands are in the
  [executables README](/tools_v2/developer-tools/src/bin/README.md#enf).
- Related features: the [mod slice workflow](/documentation_v2/runbooks/mod_slice_workflow.md),
  which sets when a slice consults the oracle and which oracle lanes it may read; the
  [capability verdicts](/documentation_v2/mod/tbd-framework/capability_verdicts.md), the table
  `enf capability` checks; the [Enfusion MCP tooling runbook](/documentation_v2/runbooks/enfusion_mcp_tooling.md),
  for the live [Workbench](/documentation_v2/glossary.md#workbench) half of the same tooling.

## Behaviour

### Building the indexes

```text
apps/mod/crf_framework/   ──▶ enf index crf     ─┐
apps/mod/vanilla_reference/ ─▶ enf index vanilla ─┴▶ .ai/artifacts/enf-index/<lane>_{symbols,files,modded,rplprops}.tsv
game paks ─▶ enf extract | enf carve ─▶ apps/mod/vanilla_reference/Scripts/
cargo xtask fetch vanilla-api    ─▶ cached Script API pages ─▶ enf apidoc ─▶ vanilla_api_{classes,members}.tsv
cargo xtask fetch vanilla-source ─▶ cached source pages     ─▶ enf source ─▶ apps/mod/vanilla_reference/Source/
```

1. The script sources are the upstream framework in `apps/mod/crf_framework/` and the vanilla
   scripts, got out of the game's `.pak` archives (`enf extract` reads by name from the archive's
   file table; `enf carve` scans the raw bytes) or rebuilt from the cached Script API pages. Both
   source trees stay gitignored.
2. `enf index <lane> --root <dir>` runs the one scanner over every `.c` file and writes four TSVs
   per lane: declarations, files, `modded` classes and replicated properties. They hold names and
   `file:line` coordinates, never code bodies, so they are committed while the sources are not.
3. A command whose result is empty refuses to write, so a broken extraction never overwrites a
   good committed index.

### Asking the oracle

- `enf lookup <symbol>` and `enf dirs` answer against `crf_symbols.tsv` by default.
- The index knows scripted classes only. An engine-native class (`BaseWorld`, `Widget`,
  `IEntity`) has no source, so "not found" is no proof it does not exist; the mod slice workflow's
  compile probe (`cargo xtask mod compile --probe`) answers for native symbols.

### Checking documents and the framework

- `enf citations` walks every Markdown file under `documentation_v2/` and resolves each `@idx`
  marker, written as the marker, a lane (`crf`, `vanilla` or `api`), `#` and a symbol, against the
  lane's table. An unresolved marker exits 1. A document names the symbol and the tool supplies
  the coordinates, so no line number is typed by hand.
- `enf capability` joins the framework index with the rules in
  `documentation_v2/mod/tbd-framework/capability_verdicts.tsv`, writes
  `.ai/artifacts/enf-index/capability_matrix.tsv`, and exits 1 when a framework file matches no
  rule, so no upstream subsystem goes untriaged.

### Where the checks run

Both checks run by hand. The mod wave gate
(`tools_v2/xtask/src/commands/mod_ops/wave_execution/execution.rs`) lists them as steps, but
reaches them through `make` targets that no tracked Makefile defines, and its oracle unit-test step
filters on a module path no test has. No CI workflow runs them.

### Known discrepancies

- The mod wave gate runs `cargo test -p developer-tools --lib enf::`
  (`tools_v2/xtask/src/commands/mod_ops/wave_execution/execution.rs:375-385`) — the module is
  `enfusion_tooling`, so the step passes with zero tests.
- The wave gate's capability and citation steps call `make verify-capability` and
  `make verify-oracle` (same file, lines 352-367) — no Makefile is tracked, so the steps always
  fail.

## Data

- `.ai/artifacts/enf-index/`: the committed tables, `crf_*.tsv` and `vanilla_*.tsv` (symbols,
  files, `modded`, `rplprops`), `vanilla_api_classes.tsv` and `vanilla_api_members.tsv`, and the
  generated `capability_matrix.tsv`.
- `documentation_v2/mod/tbd-framework/capability_verdicts.tsv`: the hand-kept verdict per
  framework path prefix; its format is in the capability verdicts document.
- `apps/mod/crf_framework/` and `apps/mod/vanilla_reference/`: the gitignored source lanes,
  linked into each slice worktree by `cargo xtask platform slice-worktree`.

## Design

The oracle exists because a model's knowledge of Enfusion is unreliable: an agent summarising one
upstream file once invented four APIs that do not exist. Every index is generated from source,
every citation in the documentation resolves against an index, and a check with nothing to read
refuses rather than passes. The indexes carry coordinates rather than code, which keeps licensed
and unlicensed source out of the repository while the facts stay checkable.

## Open work

- [T-1001 — Mod wave gate calls make targets with no Makefile](/.ai/tickets/T-1001.toml) (idea,
  no plan): the gate calls `enf citations`, `enf capability`, the schema validation and
  `verify no-crf-leak` directly, so the oracle checks run on every mod wave.
- [T-1106 — Fix mod wave gate enf test filter matching no tests](/.ai/tickets/T-1106.toml) (idea,
  no plan): the gate's unit-test step selects the `enfusion_tooling` tests.

## Decisions

- Indexes are generated and committed, sources are gitignored: the repository carries only names
  and coordinates, never upstream code.
- One scanner serves every lane: the framework and vanilla indexes cannot disagree about what a
  declaration is.
- Documents cite symbols through markers the tool resolves: a wrong API name fails a check
  instead of misleading the next reader.
