**Status:** live

# Enfusion code policy

Rules ENF-1 to ENF-4: how [EnfScript](/documentation/glossary/a_to_f.md#enfscript) code in the
[mod](/documentation/glossary/g_to_m.md#mod) addons under `mod/` behaves. The comment tags that
networked code carries (`@authority`, `@rpc`, `@replicated`, `@contract`) and the Doxygen comment
rules belong to the
[documentation standards](/documentation/standards/documentation_standards.md#6-enfusion-comments)
and are not restated here.

## Rules

- **ENF-1 (Debuggability) — Logging is disciplined, and development switches ship off.** Every
  `Print` carries an explicit `LogLevel`; nothing logs once per frame or once per replication tick
  on a hot path; a developer test switch defaults to off (`[Attribute("0")]`). The framework's
  logger, `mod/tbd-framework/Scripts/Game/TBD/Core/TBD_Log.c`, and the phase managers cite
  ENF-1 at the places they keep a path silent. Gate: MANUAL. Log levels and attribute defaults
  show only when [Workbench](/documentation/glossary/n_to_z.md#workbench) or a server runs the code;
  no static analyser for EnfScript exists in this repository.
- **ENF-2 (Debuggability) — An authority gate says why.** Every early return on the wrong
  replication side, such as `if (RplSession.Mode() == RplMode.Client) return;`, carries a comment
  of the form `// Authority only — <reason>`. Gate: MANUAL, for the same reason as ENF-1.
- **ENF-3 (Readability) — Networked-code tags resolve.** Every `@contract` citation in a `.c`
  file names a schema definition that exists, and the `@authority`, `@rpc`, `@replicated`,
  `@route` and `@contract` tags sit where they belong. Status: live, unenforced (recommended
  style); review holds it.
- **ENF-4 (Usability) — A JSON document the mod parses has a golden sample that validates.** The
  ten samples in `contracts/fixtures/enfusion_samples/` cover the parts of the [mission](/documentation/glossary/g_to_m.md#mission) schema
  the mod's DTO classes read; the schema gate validates each against its definition, and a sample
  whose name has no definition fails. Gate: CI-SCRIPT, the Enfusion DTO branch of
  `cargo xtask ci schema-validate`. That every DTO class has a sample is unenforced: the gate
  validates the samples that exist and does not look for DTO classes without one. The
  [samples README](/contracts/fixtures/enfusion_samples/README.md) describes the folder.

ENF-1 and ENF-2 are the only MANUAL rules in these standards, and they stay the only ones: a new
rule names an automated check or is stated as unenforced.

## Checking mod code

No gate of `cargo xtask ci ci-local` compiles EnfScript: the API's tests and the app build never
compile a `.c` file. A mod change is checked by `cargo xtask mod compile` (the compile gate, which
also probes whether an engine API exists) and a pass in Workbench or on a dedicated server for the
MANUAL rules; the mod world boot runs nightly or on demand. The procedure is in
[Mod slice workflow](/documentation/runbooks/mod_slice_workflow.md).

`cargo xtask verify file-length` warns about `.c` files of the pinned mod Scripts roots past the
500-line guidance of CLAUDE.md law 7 (see
[File size and complexity](/documentation/standards/coding_standards/file_size_and_complexity.md)).
