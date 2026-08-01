# Adversarial verification — framework-analysis corpus vs primary source

**Date:** 2026-08-01 · **Verifier:** Fable 5 (adversarial pass, source re-derivation lens)
**Scope:** the 16 planning artifacts under `.ai/artifacts/` (5 framework analyses + synthesis +
editor-UI plan/drafts + 8 eden-screenshot batches + README + 3den catalogue) checked against
primary source on `/run/media/system/Disk_2/tbd-framework-analysis/` (read-only, untouched),
the 75 screenshots in `~/Documents/Arma_3_Screenshots/`, GitHub `R3voA3/3den-Enhanced@73f6868`,
and this repo's own code/registry. **Findings documented, nothing fixed.**

Method note: every verdict below was re-derived — patterns re-executed, counts re-counted,
line numbers re-read in source. Citations were never inherited. WOG / FNF-v4 / OFCRA
re-derivations were run by three parallel verification agents whose commands are quoted in §3;
MissionAnalyzer, DTAS, fnf_v3 sampling, screenshots, 3den spot-checks, synthesis and
editor-UI docs were re-derived directly.

---

## 1. Verdict table

Legend: **HOLDS** = re-derived exactly · **PARTIAL** = core true, stated form wrong somewhere ·
**FAILS** = source contradicts the claim · **UNVERIFIABLE** = cannot be re-derived from staged source.
Severity: does the failure change a recommendation, or is it cosmetic?

### FNF-MissionAnalyzer (`fnf_tooling.md`) — verified directly

| # | Claim | Doc | Verdict | Evidence | Severity |
|---|---|---|---|---|---|
| 1 | 27 checks exist, 14 execute, 13 dead (11 commented, 2 killed by bugs) | fnf_tooling §1.3 | **HOLDS** | Gates P1–P5 at `:288,:304,:314,:348,:369`; R1–R9 live; dead: D1 `:902` (bug), D2 `:943` (bug), D3 `:840–845`, D4–D8 `:846–867`, D9/D10 `:961–975`, D11 `:1694–1702` (HTML comment), D12 `:1493–1498`+guard `:1935`, D13 `:1235–1427`+render `:1958` — comment delimiters re-read | Taxonomy caveat only: D9/D10 duplicate live R7/R8, disclosed in doc |
| 2 | `$MarkObjs` typo `:902`, `$ReqCoreObjs.name` guard `:943` disable objective checks | fnf_tooling §1.3 C | **HOLDS** | `grep -n MarkObjs` → exactly one occurrence (`:902`); everything else is `$MarkerObjs`, so guard tests `$null`. `$ReqCoreObjs` is an array-of-arrays (`:934–941`); member enumeration of `.name` over `Object[]` elements yields an empty collection → falsy → loop never runs | The corpus's highest-value finding is genuine |
| 3 | "Objectives unverified for five years" (bug landing date) | fnf_tooling §1.3/§3.5 | **PARTIAL** | Clone is shallow: `.git/shallow` present, `rev-list --count HEAD` = 1, HEAD `3ee5b17` 2021-03-31. Bug is in that tip ⇒ ≥5.3 years shipped-broken. **Landing date is unknowable from this clone** — "when the bug landed" cannot be answered. Doc discloses the shallowness (§1.1 caveat) | Cosmetic — "five years" survives as a lower bound |
| 4 | Both shipped fix-it examples fail the tool's own regex | fnf_tooling §1.3 R2 | **HOLDS** | Patterns transcribed verbatim from `:545`/`:553`, re-run in Python `re`: help-text `:1742` **FAIL**, `:1743` **FAIL**; in-code comments `:554–556` all MATCH; R1 examples `:1726–1727` MATCH. Bonus: `FNF_DTAS_Altis` fails R1, as §2.5#6 claims | — |
| 5 | Role-description accordion can never render green (`:1760`) | fnf_tooling §1.3 vs §3.5 | **PARTIAL** | `:1760` verbatim: green requires **all four** lists non-empty, incl. both *unlabelled* lists ⇒ §1.3's precise claim ("a **perfect** mission can never go green") **HOLDS**. But §3.5's restatement "can never render green" is **false**: a mission with ≥1 labelled and ≥1 unlabelled unit in both C2 and G/H **does** render green — i.e. green rewards half-done work, which is worse | Cosmetic (overstated restatement); underlying defect real |
| 6 | Tool rewrites `mission.sqm` in place (`:330`, `:341`) | fnf_tooling §1.1 | **HOLDS** | `grep -n WriteAllLines` → `330:`, `341:`, both targeting `$FilePathSQM` | — |
| 7 | Gate P3 hard-exits on FNF's own DTAS mission | fnf_tooling §2.2 | **HOLDS** | `:314–318` = `Write-Error` + `Pause` + `exit` on missing `config.sqf`; `find FNF-DTAS-Altis -iname config.sqf` → nothing, in either variant | — |

### WOG (`wog.md`) — re-derived by verification agent over all 171 extracted mission PBOs

| # | Claim | Verdict | Evidence | Severity |
|---|---|---|---|---|
| 8 | Med/Eng auto-tagger regex `".*\| (Med\|Eng).*/g"` never matches (`/g` literal in SQF) | **FAILS** | Code quote and location are accurate (`extracted/wog3_3den/functions/fn_onMissionSaveEH.sqf:5`, `regexMatch`, `/g` inside the pattern string). **The semantics are wrong: SQF regex flags are legal trailing `/flags` syntax** — per BI's official docs ("flags are specified at the end of the pattern and start with /"; `g` and `i` are the defaults), `".*\| (Med\|Eng).*/g"` parses as pattern + global flag and **matches**. The guard works; the "dead strip branch / duplicates every save" corollary is unsupported. The doc's corroborating corpus fact (zero generated ` \| Med` tags in 171 missions) is true but does not establish this mechanism | **MAJOR for the record** — see §2 |
| 9 | `wog3_no_auto_long_range_radio = true` in 74/171 missions | **HOLDS** | 74 exact (all `= true`, 0 false/commented; one mission carries it twice → 75 lines) | — |
| 10 | 19,627 slots · median 137 · max 324 · W 52 / E 42 / I 6 | **PARTIAL** | Raw `isPlayable=1` over all 171 decoded SQMs = **19,775**, max **329**. The doc's numbers reproduce only after silently excluding 157 side-less virtual playables (140 `ace_spectator_virtual` + 17 `HeadlessClient_F`) → 19,618 (Δ9 = 0.05%); **median 137, p25 37, min 9, max 324 exact** under that exclusion; sides 51.7/42.3/5.9 ≈ doc | Cosmetic-to-minor — defensible definition, but the doc's stated method ("isPlayable = 1") does not reproduce its own headline number |
| 11 | 162 triggers vs 8,628 markers | **HOLDS** | **162 / 8,628 exact** over all 171 decoded SQMs | — |
| 12 | 78 WOG-native / 33 OFCRA / 60 third-party; all 437 `readme.md` OFCRA | **HOLDS** | WMT_Main missions = 78; `omtk/` dirs = 33; disjoint; remainder 60. `readme.md` = **437 exact**, all inside the 33 omtk missions, zero elsewhere (`comm -23` empty) | — |
| 13 | `WMT_Task_Point` ×166; `Jerrycans` ×373 | **HOLDS** | 166 in 73 missions exact; 373 `wog_editorLoadedJerryCans` properties exact, incl. the doc's value histogram (0×143, 2×87, 4×64) | — |
| 14 | Self-correction 182 → 28 USMC group presets; is 28 right? | **FAILS as briefed / 28 HOLDS** | **No "182" and no self-correction exist anywhere in wog.md** (grep → 0 hits); the doc states 28 cleanly. 28 re-derived correct: deraped `wog3_usmc/Config.bin` → `CfgGroups >> west` → **28 group classes** (12+12+2+2; 150 Unit classes below). 4+28+150 = **182 total classes in the subtree** — almost certainly the phantom's origin. The "self-correction" lives in a chat/brief, not the artifact — the §D.2 failure shape again | See §2 — meta-finding about the brief, not the doc |

### FNF v4 (`fnf_v4.md`) — re-derived by verification agent against FNF-full git + v4.7.0 tree

| # | Claim | Verdict | Evidence | Severity |
|---|---|---|---|---|
| 15 | v4.0.0 single squashed commit, parent = v3.6.9; 1504 files +1,045,833/−85,891 | **HOLDS** | `git show --shortstat v4.0.0` exact; v4.0.0 = `68e6d38d` (2023-12-16, "Manual Merge of 4.0.0"), `%P` = `285e4441` = v3.6.9 (2023-09-12); one commit between tags | — |
| 16 | Histogram 157 `Fixed:` vs 78 `Added:` | **PARTIAL** | Reproduces **exactly** with word-anchored `^Fixed`/`^Added` (strict `Fixed:` gives 156/73 — 5 subjects lack the colon). Range label off by one: `v4.0.0..v4.7.0` = **350** commits, 351 is `v3.6.9..v4.7.0` (fnf_v4:803 right, fnf_v4:707 + synthesis:831 wrong) | Cosmetic — ratio and conclusion unchanged |
| 17 | `fnf_distanceToDisable` written / `fnf_distanceForDisable` read — feature inert | **HOLDS** | Writer `modules.hpp:506-507`; reader `fn_initMobileSpawnPoints.sqf:67` `getVariable ["fnf_distanceForDisable", 250]`; no other writer/reader case-insensitive. Git: both names born in one commit `f8257b1d` (2026-05-27) — **never consistent**; always falls back to 250 | — |
| 18 | 5 placeable modules missing from `CfgPatches >> units[]` | **HOLDS** | units[] = 17, scope-2 modules = 22 (+1 abstract); set-difference exactly `miscOptions, respawnPosition, mobileSpawnPointHandeler, assetRestriction, personalRearm` | — |
| 19 | 85 kits · 77 playable slots · 62 typed attributes on 22 modules | **PARTIAL** | 85 ✓ (85× `editorCategory = "fnf_Kits"`); 77 ✓ (77× `isPlayable=1` in `USArmy[2020]/composition.sqe`); 62 live attrs ✓ (63 `property =` minus one commented `fnf_GlobalPoints`) across 22 scope-2 modules ✓. Sub-split off by one: 64 live / 21 dark kits, not 65/20 (`fnf_FinnishArmy2020` is commented-out Blufor). Doc-internal: fnf_v4:872 says "45 typed attributes" vs its own §10.5 62/46 | Cosmetic |
| 15b | Synthesis §B.1 hashes/dates; `v3.6.9..v4.7.0` = 1672 files +1,438,844/−91,349 | **PARTIAL** | Diffstat and hashes exact; "351 commits, 350 post-4.0.0" right in synthesis:807, mislabelled at synthesis:831 | Cosmetic |
| 15c | ORBAT flattening = two commits +865,608/−1,739,275 across 191 files, "more change than the other 349 combined" | **PARTIAL** | Both commits exist, stats **exact** (`f401c2dd` +543,432/−1,547,382; `778f2333` +322,176/−191,893). But "more than the other 349 combined" holds **only for deletions** (1.74M vs 1.46M); on total churn it is 49.1% (2.60M vs 2.70M); and `578ba60e` "Removed: Most kit mission files" (−943,897) outranks the second commit. A third large ORBAT commit (`0b7fc815`, +167k/−264k) is unmentioned | **Minor** — rhetorical support for §B.2 point 2 weakens; the point's conclusion (commit counts ≠ effort) survives, ironically reinforced |
| 15d | FNF-full = 1635 commits, "all 118 tags" | **PARTIAL** | 1635 exact; tags = **117** (`git tag \| wc -l`), README.md:41 off by one | Cosmetic |

### OFCRA (`ofcra_omtk.md`) — re-derived by verification agent against `ofcra/omtk`

| # | Claim | Verdict | Evidence | Severity |
|---|---|---|---|---|
| 20 | No NVGs for infantry across all 57 faction files | **HOLDS** | Exactly 57 `infantry/{bluefor,redfor}-loadouts-*.yml` (28+29); `grep -ri "nvg\|1PN\|pvs\|anvis"` → **0 hits** in all 57; `goggles:` = scarves/balaclavas only. Every NVG is `ACE_NVG_Gen4` in `vehicles/*-cargos.yml`, tiered exactly as documented (car 8, truck/heli 15, apc 10, mbt 3, attack air 2, plane_transport 50, boat 8 BLU / 4 RED) | — |
| 21 | AT gets exactly 3 spare rockets of 3 different warheads | **PARTIAL** | Doc's precise figures exact (3 spares in 48 files, 2 in 7) but headline over-generalizes: **2 files ship 0 spares** (NLAW `baf-dpm-tmp`, ERYX `french-ce`), "3 different warheads" holds only **38/57** (17 files carry 2 types; the 6 SMAW files count 2 spotting-rifle rounds as "rockets"). Cited RPG-7 example verbatim at `redfor-loadouts-ru-flora.yml:225-236` | **Minor** — "universal doctrine" reads stronger than source; the design lesson (typed spare-rocket policy) survives |
| 22 | Rifle mags 12 West / 8 Russian / 5–7 insurgent | **PARTIAL** | Re-derived: 12× in **33** (doc: 32), 8× in **11** (doc: 10), 7× in 5 ✓, 6×/5× bucket = 4 files (doc: 3); 4 files unbucketed (10× PLA ×2, 10×/9× grenade-mixed). Direction and side-asymmetry solid: West mode 12, RU regulars 6–8, insurgents 5–8 | Cosmetic — off-by-ones; the asymmetry finding stands |
| 23 | `"8x 30#classname"` grammar; explicit-null-as-remove | **HOLDS** | Regex **verbatim** at extracted `src/lib/omtk/loadout/infantry_manager.rb:148`: `/^((?<quantity>[0-9]+)x )?((?<rounds>[0-9]+)#)?(?<item>.*)$/`. Null-as-remove real: bare YAML key, nil survives `merge_loadouts` (:160-176), emission skips nils (:100-110) → inherited default suppressed. Re-derived by re-extracting `omtk-loadouts.exe` (Ocra stub, LZMA overlay @0x9a00, 13,326,664 B / 574 files — both matching the doc) | — |
| 24 | SQM version whitelist `[12,51,52]` | **HOLDS** | Exact at extracted gem `sqm2json-0.0.3/lib/sqm2json/version.rb:5`; consumer `src/lib/omtk/config/manager.rb:68-69` with both warning strings quoted verbatim by the doc | — |

### The convergence claim (synthesis §A.4 / Decision 4)

| # | Leg | Verdict | Evidence | Severity |
|---|---|---|---|---|
| 25a | Analyzer splits description on `@` (`AnalyzeSQM.ps1:448-449`) | **HOLDS** | Read verbatim: `($_.Attributes.description -split '@')[0]` / `[1]` | — |
| 25b | FNF v3 `Role@Callsign` in primary source | **HOLDS** | 261 `description="…@…"` values in `FNF_MissionTemplate.VR/mission.sqm` (`Machine Gunner@Delta 1`, `Vehicle Commander@Golf 1`, …); runtime parser `server/init/fn_serverInit.sqf:197-199` verbatim: `(roleDescription _killer) splitString '@' select 1` | — |
| 25c | FNF v4 `Role@Group` + export warning at `init3DEN.sqf:176` | **HOLDS** | `init3DEN.sqf:160` `_splitString = _roleDescription splitString "@";`; **:176 verbatim** `systemChat ("WARNING: Group " + _groupID + " leader does not have its role description set properly")`. Six `splitString "@"` sites in v4 (`fn_updateOrbat.sqf:77,91`, `ORBATChange.sqf:14`, `fn_spawnCustomSidedKit.sqf:220,283`); kit slots carry `description="Platoon Command@Command HQ"` (`composition.sqe:371`; doc cited 369, off by 2) | — |
| 25d | OFCRA `Role@SquadName` | **HOLDS** | Two readers split on `@` in primary source: `omtk/fn_rosterBriefing.sqf:74,:84` (`_nbr = (roleDescription _x) find "@"`; role = `select [0,_nbr]`, squad override = `select [_nbr+1]`), `omtk/table_forum.sqf:85-88`. Caveat: convention is reconstructed from reader code — no in-repo authoring doc states it, and class-file `description:` values carry no `@` | — |
| 25e | WOG `Role@Group` | **HOLDS** | Framework: `fn_onMissionSaveEH.sqf:3` `splitString "@"` (+ rejoin `:8/:12`). Corpus: **3,236 of 18,478 `description=` values contain `@`, across 160/171 missions**; the doc's exact example counts verified: `"1: Squad Leader@Team 1"` 21 missions, `"1: Officer@Command Team"` 25, `"1: Zeus@Azus"` 33 | — |

**Claim 25 verdict: HOLDS — genuine independent convergence.** All five legs exist in primary
source with different separator *consumers* (Discord embed, 3DEN export, roster briefing,
save-hook tagger, validator), different extensions (side-padding, `"1: "` index, ` | Med`
qualifier), and different eras. This is not agents echoing one another: the code is there,
five times, written by four communities.

### Sweeps — unsourced claims, reversals, cross-document numbers

| # | Item | Verdict | Evidence | Severity |
|---|---|---|---|---|
| S1 | Known fabrication (§D.2 "single cheapest, highest-value idea") is correctly documented | **HOLDS** | Phrase absent from every artifact (re-grepped); §D.2's subsidiary claim "only occurrences of cheapest/highest-value are `fnf_tooling.md:403` and `:861`" re-derived **exact** | Calibration confirmed |
| S2 | `fnf_v3.md` zero `INFERRED:` over 1198 lines — rigour or unlabelled inference? | **HOLDS (rigour), 1 FAILS item** | 0 markers confirmed (others 17/8/5/3, exactly as README states). Deep-sample of 12 specific claims: **11 exact** (28 Comment objects; `cfgFNFORBAT.hpp` = 974 lines; 267 playable / 54 groups / 12 layers; 27 `fnf_*` settings; `config.sqf:128` value verbatim; `fn_serverInit.sqf:197-199`; `fn_setGroupIDs.sqf:174-175`; 15 restriction scripts in `client/restrictions/`; 9 game modes in `mode_config/`; `fn_lobbyTextGenButton.sqf` exists; tutorial ending-quote verbatim). **1 wrong: the ":4093 7-paragraph tutorial" has 5 paragraphs** (9 `\n`-segments incl. blanks — 7 matches neither counting) | The zero-INFERRED doc is substantively sound; one inflated count propagated into `framework_synthesis.md:1157` |
| S3 | fnf_tooling "internal reversal": triage 40 live / 5 on-save / 7 post-hoc vs "nothing genuinely post-hoc" | **FAILS (the premise, not the doc)** | **No 40/5/7 triage exists in fnf_tooling.md or anywhere in the corpus** (grepped all 16 docs). Its actual §1.5 triage is **21 live / 2 on-save / 0 post-hoc / 4 n/a**, which is *consistent* with its §1.5 conclusion and with synthesis §D.3's "21 of 27". There is no internal reversal. The 40/5/7 figures entered the verification brief the same way the §D.2 phrase entered the record — asserted, uncited, wrong | Important as meta-finding; no doc change needed |
| S4 | Numbers that disagree between documents | **1 disagreement found** | Checked: 157/78 histogram (fnf_v4:707 = synthesis:831 ✓), 1504/+1,045,833/−85,891 (fnf_v4:793 = synthesis:805 ✓), 27/14/~5y (tooling = README = synthesis ✓), 78+33+60=171 (README = synthesis ✓), 21-live (tooling = synthesis ✓), drafts "23 tickets" (T-631…T-653 = 23 ✓). **Disagreement: none between docs — but "seven-paragraph" is wrong in both fnf_v3.md:230 and synthesis:1157 (correlated error, not independent)** | Cosmetic |
| S5 | eden_screenshots corpus | **HOLDS (sampled)** | 75 PNGs exist; dimension claim **exact**: 23 files at 1897×1077 spanning `161621`–`163658`, 52 at 1920×1077 (PNG IHDR re-read). Status-bar m/pix mechanism re-derived: frame `170450` prints `1.02586 m/pix` — inside batch08's claimed ~1.03 band for the ~5 m interval | — |
| S6 | 3den_enhanced_feature_catalogue.md | **HOLDS (sampled), source not staged** | GitHub `73f6868` not on Disk_2; spot-checked `fn_measureDistance.sqf` at that commit via raw.githubusercontent: **14.15 km/h verbatim** (`_dist3D / (14.15 * 1000) * 3600`), `BIS_fnc_markerPath … 50` ✓, `drawLine3D … [1,0,0,1]` ✓. The "zero hits across 1,700 files" LoS grep not re-run (needs full clone) | — |
| S7 | editor_ui docs — "the T-628 bar", "next free id is T-631" | **HOLDS, with a repo finding** | T-628 is real and load-bearing in code (`mission_editor.rs:185,:213,:452…`, `world_assets/*`, `aegis.css:260`), as are T-627/T-629/T-630 (`ratelimit.rs`, `t630_map_assets_exempt.rs`) — but **`.ai/tickets/registry.json` tops out at T-626**: four shipped tickets exist only as code comments. "Next free = T-631" is correct against code, unverifiable against the registry it names | Registry desync — repo hygiene, not a corpus error |
| S8 | Synthesis citations into TBD's own code | **HOLDS** | `editor_ops.rs:120-132` `SlotAttrs{…role,tag,squad}` exact; `:1167-1179` `OrbatSlotDetail{…callsign,rank,index,squad_id}` exact; `mission_editor.rs:36-93` `thread_local!` registry SPA-session cache (at `:48`) ✓; `outliner.rs:44-50` `LayerRow` workflow struct ✓; `contract/validate.rs` zone scan at save boundary (`:113,:119,:128`) ✓; `mission.schema.json:668` respawn enum still offers `"tickets"` ✓ | — |
| S9 | **New defect the corpus missed:** Analyzer R9 locked-vehicle branch is polarity-inverted | **(new finding)** | `:1123` usable branch: `-eq <empty-sentinel> → "Empty"`. `:1144` locked branch: `-ne <empty-sentinel> → "Empty"` — inverted. A locked vehicle **with cargo** is labelled "Empty" and passes R9; a locked **empty** vehicle is flagged. fnf_tooling cites both lines (`:1123`/`:1144`) as equivalent sentinels and never notes the inversion. Same class: R6/D1-shape guards (`if ($TriggerObjs.name)`) fail **open** when a mission has *zero* triggers — the "missing trigger" rule can't fire on the worst input | Minor doc omission; strengthens (not weakens) the "make every rule fail on demand" recommendation |
| S10 | DTAS Part-2 census (fnf_tooling §2.3–2.4) | **HOLDS** | Re-counted: 124 Group / 124 isPlayable / 178 Object / 5 Marker / 2 Trigger / 2 Logic / 1 Layer — all exact; `maxPlayers = 124` at `description.ext:8`; **missing comma verbatim** at `:235` (`13, 14 15, 17`), values[]=25 vs texts[]=26; `minPlayers` 4 (ext:7) vs 2 (sqm:136) | — |

---

## 2. FAILS / PARTIAL diagnoses (with reproduction)

### F-8 — THE MOST CONSEQUENTIAL ERROR: the WOG `/g` regex is not proven broken, and three documents repeat that it is

`wog.md §14.1` (echoed §4b, §15.5, steal-list #3) claims WOG's Med/Eng auto-tagger regex
`".*\| (Med|Eng).*/g"` can never match because "`/g` is a literal part of the pattern — SQF has
no /g flag syntax". **SQF has exactly that syntax.** Bohemia's official documentation
(`Arma 3: Regular Expressions`, `regexMatch`) specifies flags *are* given as a trailing
`/`-prefixed suffix of the pattern string, lowercase, with `g` and `i` as defaults. So the
shipped pattern parses as pattern `.*\| (Med|Eng).*` + flag `g` (making it case-sensitive
global) and **matches tagged descriptions**; the strip branch is reachable and the
"appends a duplicate on every save" corollary is unsupported.

What survives: the verbatim code quote and location (`wog3_3den/functions/fn_onMissionSaveEH.sqf:5`)
are accurate, and the corpus observation is real — zero framework-cased ` | Med`/` | Eng` tags
in 18,478 descriptions across 171 missions (the only pipes are hand-typed `| MED`/`| ENG` in one
Star-Wars-themed mission). Something keeps the tagger's output out of the corpus — mod not
loaded at save-time, feature younger than the archive, or another cause — but **the documented
mechanism is wrong**.

Propagation (all need the same one-line correction):
- `wog.md` §14.1 headline finding + §4b + §15.5 + steal-list
- `frameworks/README.md` "Both tool-analyses found the tool broken in the same way… **Neither community appears to know**"
- `framework_synthesis.md:54-55` (Decision 2 evidence) and `:1194` (T-656 rationale: *"Both validators in the corpus are silently dead — `$MarkObjs` and the `/g` flag"*)

Severity: **major for the record, minor for the build order.** The "make every rule fail on
demand" recommendation and T-656 stand unchanged on the FNF leg alone — `$MarkObjs` is verified
beyond doubt. But the symmetric "two rival communities, same silent failure" story was the
synthesis's second-strongest rhetorical beat, and one of its two legs is gone. Caveat recorded:
verdict rests on official BIKI semantics, not a live engine run (BI wiki 403s direct fetch;
retrieved via search snippets); an engine test would settle it conclusively.

```bash
sed -n '1,15p' /run/media/system/Disk_2/tbd-framework-analysis/wog/extracted/wog3_3den/functions/fn_onMissionSaveEH.sqf
# then read https://community.bistudio.com/wiki/Arma_3:_Regular_Expressions  §Flags
```

### F-14 + S3 — two phantom claims in the verification brief itself (the §D.2 failure shape, live)

Two premises this adversarial pass was asked to check exist in **no artifact**:

1. *"wog.md self-corrects from 182 to 28 USMC group presets"* — wog.md contains no `182` and no
   correction passage; it states 28 cleanly, and 28 re-derives correct from
   `wog3_usmc/Config.bin` (28 group classes; the subtree's *total* class count 4+28+150 = 182 is
   almost certainly where the phantom number was minted).
2. *"fnf_tooling.md first triaged rules 40 live / 5 on-save / 7 post-hoc, then reversed itself"* —
   no such triage exists anywhere in the corpus; the real triage (21 live / 2 on-save /
   0 post-hoc / 4 n/a) is consistent with its own conclusion. There is no reversal.

Both are the exact failure `framework_synthesis.md §D.2` documents: a number or narrative from a
chat/summary enters the record carrying a citation it never had. The record (the artifacts) is
cleaner than the brief about the record. Practical rule restated: **a claim is sourced when it
is in an artifact with a citation; a summary is a pointer to work, not evidence of it.**

### F-10/W3 — the WOG slot census uses an unstated exclusion

`wog.md §15.4`'s 19,627 slots reproduces only after excluding 157 side-less virtual playables
(140 `ace_spectator_virtual`, 17 `HeadlessClient_F`); raw `isPlayable=1` = 19,775, max 329 not
324. Median 137 / p25 37 / min 9 / side split are exact under that exclusion. Defensible
definition ("human combat slots"), but the doc describes its method as counting `isPlayable = 1`
— which does not yield its own number. One sentence would fix it. Severity: cosmetic-to-minor
(the A.4 scale-target recommendation is unaffected).

```bash
grep -hEc '^\s*isPlayable\s*=\s*1\s*;' $SCRATCH/sqm_text/*.sqm | paste -sd+ | bc   # 19775
```

### F-16/15b/15c — v4 git numerology: exact stats, loose labels

- Histogram 157/78 reproduces with word-anchored `^Fixed|^Added` (not strict `Fixed:` — 156/73);
  the range label "(351 commits)" for `v4.0.0..v4.7.0` is off by one at `fnf_v4.md:707` and
  `framework_synthesis.md:831` (350; the 351 belongs to `v3.6.9..v4.7.0`, which fnf_v4:803 and
  synthesis:807 state correctly).
- The two ORBAT-flattening commits exist with **exact** claimed stats, but *"more change than
  the other 349 combined"* is true only for deletions (1.74M vs 1.46M), not total churn
  (49.1%), and `578ba60e` (−943,897) is individually larger than the second of them. The §B.2
  moral — commit counts are not effort — is ironically strengthened.
- `frameworks/README.md:41` "all 118 tags" → 117.

### Aggregate-count wobbles found by sampling (all cosmetic, none load-bearing)

| Doc | Claimed | Re-derived | Note |
|---|---|---|---|
| fnf_v4.md:21,109 | 143 registered functions | **146** function-leaf classes | no counting method yields 143 |
| fnf_v4.md:174/:610 | "15 extra" / "17 entries" marker colours | **18** scope-2 classes | both numerals wrong; the doc's own 18-name lists are right |
| fnf_v4.md:65,601 | 518 `force force` settings | 518 forced lines, **515** literal `force force` | 3 single-`force` lines; "30 commented sections" ≈ 61 header lines |
| fnf_v4.md:872 | "45 typed attributes" | contradicts its own §10.5 (62 live / 46 distinct — the correct figures) | doc-internal inconsistency |
| fnf_v3.md:230 + synthesis:1157 | "7-paragraph tutorial" | **5** paragraphs (9 segments) | correlated propagation, ending quote verbatim-correct |
| ofcra_omtk.md:606 | "identical key set in all 57 files" | 3 files carry stray top-level keys | all 28 roles do appear in all 57 |
| ofcra_omtk.md §5.7.5 | 12× in 32 / 8× in 10 / 5–6× in 3 | 33 / 11 / 4 (+4 files unbucketed: 10×/9×) | asymmetry finding intact |
| ofcra_omtk.md §5.7.4 | (headline) 3 spares, 3 warheads | 0-spare files ×2 omitted; 3-warheads only 38/57 | precise sub-figures (48/7) exact |

### P-3 — "five years" holds only as a lower bound; the landing date is out of reach

The MissionAnalyzer clone is shallow. `fnf_tooling.md` §1.1 discloses this, but §1.3/§3.5's
"five-year-old silent failure" reads as a dated event. What the evidence supports: the typo is
present in the only visible commit (2021-03-31), and no commit has landed since ⇒ the tool has
shipped broken for ≥5.3 years. When it broke is unknowable without a full clone of the upstream.

```bash
cd /run/media/system/Disk_2/tbd-framework-analysis/fnf/FNF-MissionAnalyzer
git -c safe.directory='*' rev-list --count HEAD   # → 1
ls .git/shallow                                    # → exists
git -c safe.directory='*' show -s --format='%h %ad' HEAD  # → 3ee5b17 2021-03-31
grep -n 'MarkObjs' AnalyzeSQM.ps1                  # typo present in tip
```

### P-5 — the accordion *can* render green, on exactly the wrong mission

`fnf_tooling.md` §1.3 states it precisely ("a perfect mission can never go green" — true);
§3.5 loosens it to "can never render green" — false. Truth table of `AnalyzeSQM.ps1:1755-1765`:

| Mission state | Labelled GH/C2 | Unlabelled GH/C2 | Colour |
|---|---|---|---|
| Nothing labelled | ∅ / ∅ | any | **red** (`$NoNamedUnits`) |
| Everything labelled (perfect) | non-∅ | ∅ | **orange** (any `!list` true) |
| Half-done in both categories | non-∅ | non-∅ | **green** |

Reproduction: read `:1760` — green (`goodbg`) is the `else` of
`(!$NonLabeledSpecialUnitsGolfHotel -or !$NonLabeledSpecialUnitsCharlie -or !$LabeledSpecialUnitsGolfHotel -or !$LabeledSpecialUnitsCharlie)`,
i.e. requires all four lists non-empty, including both *unlabelled* lists.

### S2 — the "7-paragraph tutorial" is a 5-paragraph tutorial

`fnf_v3.md:230` and `framework_synthesis.md:1157` both say 7 paragraphs. The Comment's
`description` at `FNF_MissionTemplate.VR/mission.sqm` (Item15, "Custom Boundarys", `id=15771`)
contains **5** content paragraphs separated by blank segments (9 `\n`-segments total). Neither
counting yields 7. The ending quote is verbatim-correct.

```bash
grep -n 'title="Custom Boundarys"' /run/media/system/Disk_2/tbd-framework-analysis/fnf/FNF-v3.6.9/FNF_MissionTemplate.VR/mission.sqm
sed -n '4091,4101p' …/mission.sqm   # count the \n "" \n separators in the description
```

Severity: cosmetic. But it is the exact failure mode the zero-INFERRED flag predicted — a
confident specific number, no marker, slightly wrong, propagated into the synthesis.

### S3 — the "internal reversal" premise is itself an unsourced claim

The verification brief asserts fnf_tooling.md "first triaged rules 40 live / 5 on-save /
7 post-hoc". No such numbers exist in that file or anywhere in the 16-document corpus
(`grep -rn "post-hoc" .ai/artifacts/` → the only rule-timing triage is fnf_tooling §1.3/§1.5).
The real triage — 21 live (P3, R1–R9, D1–D11), 2 on-save (D12, D13), 0 post-hoc, 4 n/a
(P1/P2/P4/P5) — is *consistent* with the §1.5 conclusion "nothing in this rule set is genuinely
post-hoc only", and with synthesis §D.3. **There is no reversal to adjudicate.** This is a live
instance of the §D.2 lesson operating on the review process itself: the brief inherited a
figure no artifact contains. (Also mirrored: memory of this program says *verify the numbers,
not the vibe*.)

---

## 3. Agent re-derivations — method and reproduction

Three parallel verification agents (Fable), read-only on Disk_2, all extraction into the session
scratchpad. Key reproduction bases:

**WOG** — all 171 mission PBOs extracted (`tools/pbo/unpbo.py`), all 171 `mission.sqm` decoded
(86 rapified via `tools/pbo/derap.py`, 85 plaintext; 0 failures — matching README's 86/85 split);
counts via flexible-whitespace grep over the decoded set, slot census via brace-stack parse
attributing `isPlayable=1` to the nearest enclosing `side=`. Beyond the priority claims, sampled:
`zeusTracer.sqf` byte-identical in 47 missions, single MD5 `cf725dbd…` — **exact**;
`ModuleHideTerrainObjects_F` 3,053 in 121 missions — **exact**; `allowDamage` 2,909×`1` exact /
66,169×`0` (doc 66,127, Δ0.06%).

**FNF v4** — `git -c safe.directory='*' -C FNF-full` (log/show/rev-list only); v4.7.0 tree greps;
template `mission.sqm` deraped to scratch for the 77-slot count; attribute/kit counts re-derived
with comment-stripping parsers (block comments distinguish live from dark — the doc's counting
hazard).

**OFCRA** — reproduced the doc's own offline extraction of `omtk-loadouts.exe` (Ocra stub, LZMA
overlay @`0x9a00` → 13,326,664 B / 574 files, both figures matching the doc) with a scratch
`extract_ocra.py`; YAML re-derived with a hand-rolled parser. Beyond priority claims, sampled:
579 explicit `backpack:` nulls with every per-role sub-figure exact; variant diff 2 hunks /
497→500 lines exact; medic standardisation 25/25/10 exact across 57 files.

The scratch scripts live under
`/tmp/claude-1000/-home-Samuel/429c40b7-ef6b-4d3b-8b75-4dac381575e0/scratchpad/{wog_verify,fnfv4_verify,ofcra_verify}/`
(session-lifetime only; the commands above and in §1 suffice to reproduce).

---

## 4. Unverifiable claims, and why

| Claim | Where | Why unverifiable |
|---|---|---|
| When the `$MarkObjs`/`$ReqCoreObjs` bugs landed | fnf_tooling §1.3 | Shallow clone — one visible commit; upstream not staged |
| Batch07's "map region MD5 `25dc42c3ebdd` identical across 13 frames" | eden batch07/README | Crop box not specified precisely enough to reproduce the hash; the *conclusion* (same map view) is consistent with the frames |
| Pixel-forensic constants (chevron bboxes, `#383838 α=0.90`, blend-solved colours) | batches 07/08 | Solved from blends, self-flagged "good enough to reproduce, not authoritative" — no config ground truth exists in screenshots |
| "Zero hits for `lineIntersect*`… across 1,700 files" | 3den catalogue §2.2 | Mod source not staged on Disk_2; re-running the grep needs a full clone. Sampled claims from the same doc verified exactly via GitHub raw at the pinned commit, so credibility is earned, not assumed |
| `wmt_main` module semantics | wog.md | Absent from corpus entirely (per README); doc already labels these `INFERRED:` |
| `/g` behaviour in a *live* engine | F-8 verdict | Verdict rests on official BIKI flag semantics; Arma is not installed in this environment and community.bistudio.com refuses direct fetch (403). A 30-second `regexMatch` test in any Arma 3 debug console settles it conclusively |
| Why zero framework-cased ` \| Med` tags exist corpus-wide | wog.md §14.1's observation | The observation is re-verified true; its cause is now open (mod-not-loaded-at-save, feature age, or another gate) — the corpus alone cannot distinguish |

---

## 5. Bottom line

**Verdict counts over the 25 priority claims:** 16 HOLDS · 7 PARTIAL · 2 FAILS
(#8 the `/g` regex mechanism; #14 as briefed — the "182→28 self-correction" exists in no
artifact, while the 28 itself is correct). Sweep items: the known §D.2 fabrication is
confirmed-and-correctly-documented; the zero-INFERRED `fnf_v3.md` survives a 12-claim deep
sample with 11 exact and 1 inflated count; the "internal reversal" premise is itself a phantom;
one new analyzer bug the corpus missed (R9 locked-branch inversion at `:1144`); the tickets
registry lags shipped code by four IDs (T-627–T-630).

**The single most consequential error: the `/g` claim (#8).** It is the only finding that
crosses documents (wog.md → README → synthesis Decision 2 → T-656 rationale), the only one
presented as a *matched pair* with a verified finding ($MarkObjs) — which lent it borrowed
credibility — and the only one whose correction changes recorded facts rather than digits:
WOG's tagger is *not known to be broken*, and "neither community appears to know" has one
community too many. No build-order change follows (T-655/T-656 stand on the FNF leg), but the
record should be corrected before the phrase is quoted again.

**The pattern across everything checked:** quoted code, line citations, hashes and
mechanically-derived counts reproduce to the digit with striking consistency — the corpus is
real measurement, not confabulation. The failures cluster in exactly two shapes: (a) *aggregate
counts summarised by hand* (off-by-ones, category splits, "7 paragraphs"), and (b) *semantic
interpretation stated with citation-grade confidence but no test* (`/g`). Shape (b) is the
dangerous one, and both instances of it in this program — the §D.2 quote and the `/g`
mechanism — were caught only by re-derivation. The corpus's own advice applies to itself:
generate the numbers from the source, and make every claimed failure fail on demand.
