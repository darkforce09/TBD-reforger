# Arma 3 mission framework analysis — WOG · FNF · OFCRA

**Started 2026-08-01.** Five agents document how three established Arma 3 communities author
missions, so TBD-Reforger's web-based Mission Creator can take the best of each.

This README is **provenance and method**. The analyses themselves are the sibling files.

## Why these three

The operator plays in / follows all three. They are the frameworks whose authoring models are worth
copying, and they are deliberately different in character: FNF is large-scale PvP, OFCRA is
rules-heavy competitive, WOG is large-scale co-op with a long mission back-catalogue.

**All three are Arma 3, not Reforger.** So the question is *"what does this authoring model teach
us"*, never *"what can we port"*. The analyses weight the **mission-maker's workflow and the
configuration surface** over runtime internals, because that is what a web editor has to replace.

## The five analyses

| File | Subject | Why separate |
|---|---|---|
| [`fnf_v3.md`](fnf_v3.md) | **FNF v3.6.9** — the pre-revamp era | Documented on its own terms |
| [`fnf_v4.md`](fnf_v4.md) | **FNF v4.7.0** — the revamped era, **plus the v3→v4 delta** | Owns the diff; v3 agent does not write it |
| [`fnf_tooling.md`](fnf_tooling.md) | **FNF-MissionAnalyzer** + **FNF-DTAS-Altis** | The Analyzer is a *mission validator* — its rule set is a candidate feature list for live validation in our editor |
| [`ofcra_omtk.md`](ofcra_omtk.md) | **OFCRA `omtk`** v2.13.7 | French-language source; rules-heavy, so §12 (what the framework forbids) is expected to be richest here |
| [`wog.md`](wog.md) | **WOG** — 50 extracted addons + 171 real missions | No repo, no docs — reverse-engineered from shipped binaries and played missions |

"Two FNF frameworks" is **one repo at two eras**, not two projects: `v3.6.9` predates a substantial
rewrite, `v4.7.0` follows it. Both are wanted — v4 for its improvements, v3 because things were
lost in the rewrite.

## Where the source material lives

**Not in this repo.** It is 485 MB of third-party code and mission archives, staged on a scratch
disk:

```
/run/media/system/Disk_2/tbd-framework-analysis/
├── unpbo.py                  # extractor, also committed to tools/pbo/
├── fnf/
│   ├── FNF-full/             # full clone, all 118 tags — for git diff between eras
│   ├── FNF-v3.6.9/           # worktree @ tag v3.6.9   (client_mod, server_mods, tools, FNF_MissionTemplate.VR)
│   ├── FNF-v4.7.0/           # worktree @ tag v4.7.0   (client_mod, server_mod, External Scripts, Kit Mission Files, FNF_Mission_Template.VR)
│   ├── FNF-MissionAnalyzer/
│   └── FNF-DTAS-Altis/
├── ofcra/omtk/               # @ v2.13.7
└── wog/
    ├── pbo/                  # 50 framework PBOs, copied from the operator's install
    ├── extracted/            # unpacked — 757 files, 366 readable source
    └── missions_pbo/         # 171 real mission PBOs, still packed
```

**The operator's own Arma install was never modified.** WOG material was *copied* out of
`/home/Samuel/Games/Arma3/WogMods/` (53 GB) and `/home/Samuel/Games/Arma3/Old_Missions/`. Only the
50 framework-class PBOs were taken — `wog*.pbo` under 2 MB, 5 MB in total — because the other
24 GB is weapons, uniforms and vehicle models with no authoring content.

Agents were instructed to treat Disk_2 as read-only and to extract into a session scratchpad.

## Reproducing the extraction

Three zero-dependency tools, no `armake2` build and no Mikero tools required:

```bash
python3 tools/pbo/unpbo.py <file.pbo|dir-of-pbos> <outdir>   # --list to inspect only
python3 tools/pbo/derap.py <file.bin|mission.sqm>            # \0raP → readable config source
python3 tools/pbo/orbat.py <mission-dir>                     # slot/side census over a mission
```

`unpbo.py` handles the `Vers` properties block and `Cprs` LZSS compression. Verified against all 50
WOG framework PBOs: **757 entries, 757 written, 0 failed**.

`derap.py` and `orbat.py` were written by the WOG agent mid-analysis and rescued into the repo —
they are why the WOG numbers are corpus-wide rather than plaintext-only. See the section below.

### Binarized content — mostly solved

The original staging note said the binarized files were opaque and that any WOG claim about one
should be treated as unsupported. **That is now substantially wrong, and the correction matters.**

The WOG agent wrote a de-rapifier for the `\0raP` format — `tools/pbo/derap.py` — which decoded:

| Input | Decoded | Still opaque |
|---|---|---|
| 107 `.bin` configs | **76** | 31 — `Texheaders.bin`, `stringtable.bin`, and deliberately obfuscated blobs |
| 171 `mission.sqm` | **86** were binarized, all decoded | — |

Independently re-verified: **25 of 25** sampled `config.bin` files decode to readable
`class CfgPatches { … }` source.

**Why this mattered rather than being a nicety:** an early plaintext-only grep concluded that
`wog3_presets` had *zero* users across the whole 171-mission corpus. It actually has two — their
`mission.sqm` files were binarized. A coverage limit had silently turned into a false negative, and
"no one uses this feature" is exactly the kind of conclusion that would have shaped the synthesis.
Every corpus statistic in [`wog.md`](wog.md) is therefore over **all 171 missions machine-parsed**,
not over the plaintext subset.

**What remains genuinely opaque**, and must still be treated as unsupported: the 31 undecodable
blobs, the 21 `.sqfc` compiled SQF, and in particular **`wmt_main`** — owner of every `WMT_*`
module — which is absent from the corpus entirely, and **`wog_main`**, which is deliberately
obfuscated with scrambled filenames in the PBO header. `wog.md` reconstructs WMT's parameter
surface from observed mission usage instead: parameter names and values are hard evidence,
semantics are labelled `INFERRED:`.

Readable-as-shipped formats were `.sqf` (150), `.hpp` (194), `.cpp` (22) and `.xml` (41).

## Method

Every analysis follows one **14-section schema** so the five are diffable:

1. Identity · 2. Mission file layout · 3. **Authoring workflow** (the priority) · 4. Slotting/ORBAT
· 5. Loadouts/arsenal · 6. Briefing/intel · 7. Objectives/game modes · 8. Respawn/tickets/medical ·
9. Zones/areas/triggers · 10. **Configuration surface** (enumerated exhaustively, never "and more")
· 11. Tooling · 12. Conventions and house rules the framework *encodes* · 13. What it does best ·
14. Friction and complaints

Plus per-agent extras: `fnf_v4.md` §15 is the v3→v4 delta; `wog.md` §15 is evidence from the
171-mission corpus.

**Evidence standard**, given to every agent: cite `file:line` for substantive claims, prefix
inference with `INFERRED:`, distinguish "read in source" from "stated in docs" from "guessed from
naming", and never guess at binarized content. Each analysis opens with a **source inventory table**
listing what was actually read.

## Reading the five — two things that look like defects and are not, and one that might be

**`fnf_tooling.md` does not follow the 14-section schema, deliberately.** It had a different brief:
Part 1 is the MissionAnalyzer rule set (§1.1–1.6), Part 2 is DTAS (§2.0–2.5). A completeness check
that looks for §4–14 will report it as missing eleven sections. It is not.

**`ofcra_omtk.md` skips from §4 to §6 in the numbering**, because §5 (loadouts) is covered at
length inside the authoring workflow and the arsenal discussion rather than as a standalone
heading. Content is present; the numbering is not contiguous.

**`fnf_v3.md` carries zero `INFERRED:` markers across 1,198 lines.** Compare: `wog.md` 17,
`ofcra_omtk.md` 8, `fnf_v4.md` 5, `fnf_tooling.md` 3. WOG legitimately needs the most — no repo, no
docs, reverse-engineered from binaries. But *zero* over 1,198 lines is either unusual rigour or
unlabelled inference, and the two are indistinguishable from outside. **Flagged for the adversarial
pass** — it is the one analysis whose evidence discipline has not demonstrated itself. Note also
that this agent spawned three sub-agents and consumed 407 k tokens, roughly double any other, so
some of its content is second-hand from workers whose own labelling was not enforced.

## Cross-analysis caveats

Findings that affect how to read more than one of the five. Recorded here because no single
analysis owns them.

**The WOG mission corpus is not one community's work.** Of the 171 archived missions, roughly
**78 are WOG-native, 33 are imported OFCRA missions** (carrying a rival embedded toolkit — `omtk`,
the same codebase [`ofcra_omtk.md`](ofcra_omtk.md) documents), and **60 are third-party**. Two
consequences:

- Any corpus-wide statistic in [`wog.md`](wog.md) is over a **mixed** population unless it says
  otherwise. Treat "the corpus does X" as "missions WOG hosted do X", not "WOG's framework does X".
- The 33 OFCRA missions are an **independent cross-check** on `ofcra_omtk.md` — real authored
  output from the framework whose repo the OFCRA agent read. Worth exploiting in the adversarial
  pass: a claim about how OFCRA *says* missions are authored can be tested against 33 that actually
  were. The tell that these are genuinely OFCRA's: **all 437 `readme.md` files in the corpus belong
  to OFCRA missions; zero are WOG's.**

**One tool is confirmed broken; the second claim did not survive verification.**

**FNF — HOLDS, verified against source.** `AnalyzeSQM.ps1` declares 27 checks and runs 14. Two are
disabled by bugs (the `$MarkObjs` typo at `:902`, the `$ReqCoreObjs.name` guard at `:943`), which
means objective-existence has gone unchecked for a long time — *how* long is a **lower bound only**:
the clone is shallow, so the landing date is not recoverable. The earlier "five years" figure is
not established.

**WOG — the `/g` claim FAILS, and I wrote it.** This README previously asserted that WOG's Med/Eng
slot auto-tagger contains a JavaScript `/g` flag pasted into an SQF regex where it is a literal
character, so the pattern never matches. **That mechanism is unsupported.** SQF's `regexMatch` does
accept trailing `/flags` syntax (BIKI: flags are `/`-prefixed at the end of the pattern), under
which `".*\| (Med|Eng).*/g"` parses as pattern-plus-flag and *matches*.

The code quote itself is accurate and re-verified —
`wog3_3den/functions/fn_onMissionSaveEH.sqf:5` — and it is the **only** `regexMatch` anywhere in
the 757 extracted files, so the corpus offers no internal precedent either way. **Residual
uncertainty is real:** the correction rests on documented BIKI semantics, not on execution. A
30-second `regexMatch` in an Arma debug console would settle it conclusively, and until someone
runs it neither reading is proven.

**What this costs.** The tidy "two rival frameworks, same defect, neither community knows" framing
was wrong — it is **one** confirmed framework, and "neither community appears to know" has one
community too many. The claim had crossed four documents (`wog.md` → this README → the synthesis's
Decision 2 → T-656's rationale), borrowing credibility from the genuinely solid FNF finding it was
presented alongside.

**The conclusion survives on the FNF leg alone**, which was verified directly: build validation
that can be **made to fail on demand**. But it is now a one-framework argument plus TBD-internal
reasoning, not a convergence argument — and that is a weaker thing that should be stated as the
weaker thing.

## What happens next

1. **Barrier** — all five land before anything is synthesised.
2. **Synthesis** — best mechanism per dimension, with reasoning, mapped onto the Mission Creator.
3. **Fable 5 adversarial pass** over the *whole* planning corpus — these five, plus the Eden
   screenshot analysis, the 3den Enhanced catalogue, `eden/gap_analysis.md`, the program plan and
   the ticket drafts — hunting for gaps, unsupported claims, contradictions and missing coverage.
   It documents findings; it does not fix them.

Related: [`../editor_ui_program_plan.md`](../editor_ui_program_plan.md) ·
[`../eden_screenshots/README.md`](../eden_screenshots/README.md) ·
[`../3den_enhanced_feature_catalogue.md`](../3den_enhanced_feature_catalogue.md)
