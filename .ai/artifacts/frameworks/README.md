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

`tools/pbo/unpbo.py` is a zero-dependency PBO extractor (no `armake2` build, no Mikero tools):

```bash
python3 tools/pbo/unpbo.py <file.pbo|dir-of-pbos> <outdir>   # --list to inspect only
```

It handles the `Vers` properties block and `Cprs` LZSS compression. Verified against all 50 WOG
framework PBOs: **757 entries, 757 written, 0 failed**.

### What it cannot give you

Of the 757 extracted WOG files, **107 are `.bin`** (binarized configs) and **21 are `.sqfc`**
(compiled SQF). Neither is readable as text, and neither is derapified by this tool. Any WOG system
that lives only in those files is **opaque**, and the analysis is instructed to say so rather than
infer contents. Readable formats are `.sqf` (150), `.hpp` (194), `.cpp` (22) and `.xml` (41).

This is a real coverage limit, not a formality — treat any WOG claim about a binarized config as
unsupported unless it cites a readable file.

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
