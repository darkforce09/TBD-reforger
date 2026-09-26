**Status:** live

# TBD Framework documentation

The design documents of the TBD Framework, the shipping game [mod](/documentation_v2/glossary/g_to_m.md#mod)
that runs a one-life [event](/documentation_v2/glossary/a_to_f.md#event) inside Arma Reforger: what it is
for, which CRF capabilities it supplies, where vanilla source comes from, and one specification per
in-game screen. Developers and agents read it before starting a mod slice.

## Contents

```text
documentation_v2/mod/tbd-framework/
├── UI/                          the in-game screens: the UI index and one folder per screen
├── capability_verdicts.md       index of the verdict table: format, verdicts, the check
├── capability_verdicts.tsv      the verdict for every CRF source file, read by `enf capability`
├── mod_design.md                what the framework is, its non-negotiables and Enfusion facts
└── vanilla_source_coverage.md   the four lanes that recover vanilla script, and what each reaches
```

## How it works

Read [mod_design.md](/documentation_v2/mod/tbd-framework/mod_design.md) first: it wins over any
slice that conflicts with it, and code comments cite its numbered sections (§2 non-negotiables,
§5 Enfusion facts, §6 deferrals), so those headings keep their numbers and wording. Its `@idx`
citations resolve through `cargo run -q -p developer-tools --bin enf -- citations`, and the
verdict table through `cargo run -q -p developer-tools --bin enf -- capability`, which fails on a
CRF file with no verdict. The [UI index](/documentation_v2/mod/tbd-framework/UI/README.md) lists
the screens; each screen folder holds its `<screen>_specification.md` feature doc and its
`visual_references/`.

## Code

- [Framework addon](/apps/mod/tbd-framework/) — the scripts, layouts, prefabs and configs the
  design and the screen specifications describe.
- [Mod suite](/apps/mod/README.md) — the addons beside the framework; the CRF checkout the verdict
  table triages sits beside them in `apps/mod/crf_framework/`, gitignored and reference only.
- [Enfusion tooling](/tools_v2/developer-tools/src/enfusion_tooling/) — `enf citations`,
  `enf capability` and the vanilla lanes.

## Boundaries

- Depends on: the [feature doc template](/documentation_v2/standards/templates/feature_doc.md)
  for the screen specifications; the CRF and vanilla symbol indexes in `.ai/artifacts/enf-index/`.
- Used by: `MOD_DESIGN` in `tools_v2/xtask/src/core/repository_layout.rs` (named by
  `cargo xtask verify no-crf-leak`); `CAPABILITY_VERDICTS` in
  `tools_v2/developer-tools/src/repository_layout.rs`; EnfScript comments that cite
  `mod_design.md` sections; the in-code READMEs under `apps/mod/tbd-framework/`.
- Rules: `mod_design.md` and `capability_verdicts.tsv` keep their paths, since code names them;
  the headings of §2, §5 and §6 of the mod design stay as spelled; every `@idx` citation resolves
  and every CRF file has a verdict.

## Related documentation

- [Mod slice workflow](/documentation_v2/runbooks/mod_slice_workflow.md) — how a mod slice is
  built and verified
- [TBD Framework program](/documentation_v2/tickets/specs/t181_event_mod_program.md) — the
  program spec the design serves
