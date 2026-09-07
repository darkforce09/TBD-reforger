# September 2026 factory run — brief artifacts

One directory per wave, named by its **ledger label** — the number in the `wave N CLOSED` commit and
in `docs/platform/FACTORY_RUN_2026-09.md`. Each holds the slice briefs (`T-XXX.md`), the agent
reports (`REPORT-T-XXX.md`) and the adversarial verify (`VERIFY.md`).

## The renumbering of 2026-09-08 — read this before trusting an old path

Through waves 253–255 the **lock row ran one ahead of the ledger label**, because wave 253 shipped one
ticket at a time and its single repack emptied two lock rows at once, leaving a pending `[[emptied]]`
that every later close inherited. Directories were being named by lock row, so:

| Old directory | Ledger label | Now |
|---|---|---|
| `wave255/` | 254 | `wave254/` |
| `wave256/` | 255 | `wave255/` |

Both were renamed on 2026-09-08 during wave 256 setup, and the `VERIFY.md` titles and the internal
`Write .../waveNNN/REPORT-*.md` self-references were corrected with them. Nothing outside these
directories pointed at the old paths.

**The offset is gone.** `wave.lock` after the wave 255 close carries `wave_base = 255` and no pending
`[[emptied]]`, so lock row 256 == ledger label 256, and directory names track both from here on.

Directories `wave236` … `wave253` were always named by ledger label and were not touched.
