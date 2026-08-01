# Adversarial verification — internal consistency and inferential soundness

**Written 2026-08-01.** Scope: do the 16 planning documents agree with each other, and do the
synthesis's conclusions follow from the evidence its own corpus states? Sibling passes own
claims-vs-primary-source and coverage-vs-asks; nothing here re-reads Disk_2 or the TBD tree.

**Method.** `framework_synthesis.md` read in full; `fnf_tooling.md`, `wog.md` (§4–§15), `fnf_v4.md`
(§7–§15), `fnf_v3.md` (§7–§8, §11–§14), `ofcra_omtk.md` (§4, §7–§8, §13–§14 + targeted greps) read
against every claim the synthesis attributes to them; both READMEs, the program plan, the ticket
drafts, and eden batches 05/06/08 read in full. Registry claims in §D.6/§D.4 checked against
`.ai/tickets/registry.json` directly. **Findings are documented, not fixed.**

---

## Executive summary — the five reasoning failures that most affect the decisions

**F1 — Community-count inflation runs through every headline convergence.**
`frameworks/README.md:28` states the ground rule: *"'Two FNF frameworks' is one repo at two eras,
not two projects."* The synthesis's own line 7 says "All three communities are Arma 3." Yet Decision
4 and §A.4 claim **"five artefacts, four communities"** — the five artefacts are fnf_v3, fnf_v4,
the Analyzer (all FNF), OFCRA, WOG: **three communities**, and there is no counting under which four
is reachable without splitting FNF against the README's explicit instruction. Every "4/4" row in the
Appendix counts FNF twice. Two rows are worse than double-counted (see F5 and the convergence
audit): the medical row counts an *absence* as agreement, and the attack/defend row contradicts
`ofcra_omtk.md §7`'s own text. The convergences are mostly real at N=3; the arithmetic that sells
them is not.

**F2 — A summary-contradicts-its-own-table error propagated into two synthesis sections, the exact
class the fabrication record warns about.** `fnf_tooling.md §1.3` ships a *verified test table*:
R1's two fix-it examples (`FNF_JohnDoe_…`, `FNF_Johnny_…`) **match** R1's regex; only R2's two HTML
examples fail R2. `fnf_tooling.md §3.1` then summarises this as "its own fix-it examples for R1 and
R2 do not satisfy R1 and R2" — contradicting the table eleven hundred lines above it. The synthesis
inherits the wrong branch twice: Decision 3 ("its regexes for mission name and lobby line are the
two rules whose own fix-it examples fail those same regexes — verified test table") and §C.4
("The Analyzer's own fix-it examples fail its own rules"). The citation points at the table that
refutes the sentence. §A.14's phrasing ("two example strings", i.e. R2 alone) is the correct one.
The "generate, don't validate" conclusion survives on R2 + R1's `[A-z]` defect, but half the
flagship anecdote is false as stated.

**F3 — The seven-defect argument (Part B) is better than the 157:78 histogram it replaced, and it
still carries two of the histogram's three defects, plus a new one.** §B.2's demolition of the
histogram is sound (no control group, counts ≠ effort, self-assigned taxonomy — all verified). But:
(a) *Self-assigned taxonomy returns.* "Every one of these is an editor-affordance failure, not a
data-model failure" is the synthesis author's own binary, and it misclassifies its own items — the
fix the synthesis prescribes for defect #2 (stable ids) and #3 (one entity, two framings) is in both
cases a **data-model change**, per §B.4 and Decision 1 themselves.
(b) *Selection replaces the denominator problem.* "All seven cluster on exactly one mechanism — the
sync graph" is false on the synthesis's own descriptions: #4 (dead knobs — attribute-name mismatch,
unread checkbox), #5 (undeclared attribute variables) and #6 (`CfgPatches units[]` packaging) are
attribute/packaging plumbing, not sync-graph failures. Three of seven genuinely sit on sync links
(#1, #3, #7). And the seven are curated from `fnf_v4.md §14`'s **fourteen** frictions — friction #1
(documentation loss), #5 (marker-polygon fragility), #8, #10–#13 are non-sync and were excluded, so
"the defects cluster on sync" is partly an artefact of picking the sync-adjacent ones.
(c) *The rebutted fallacy reappears in the rebuttal.* §B.2 point 2 argues the ORBAT flattening was
"the single largest piece of work in the v4 era… two commits carrying more change than the other 349
combined" from `+865,608 / −1,739,275` lines — line-churn-as-effort, one paragraph after ruling
commit-counts-as-effort inadmissible, over **machine-regenerated kit compositions** (85 × ~12–15k
lines, per §A.5's own description). The arithmetic is right (verified against `fnf_v4.md §15.7`:
543,432+322,176 / 1,547,382+191,893 / 128+63); the inference form is the one just rejected. Note
also the histogram rows sum to 333 of 351 commits; 18 unclassified commits go unremarked in both
documents. The **conditional conclusion** of §B.6 (relationships only behind visible/typed/validated
edges) survives on defects #1/#3/#7 alone — but it is presented as resting on a seven-defect
single-mechanism cluster that does not exist as described.

**F4 — INFERRED discipline holds in the body and fails at headline altitude.** The synthesis's
citation legend promises "Carried forward, never laundered," and in the body it mostly is (§A.7,
§A.8, §A.9, K10, U1's note, C.5 row 10 all carry markers — verified). But Decision 1's executive
text asserts `WMT_Task_Point` semantics as fact: "Take the missing parameters: **attacker count,
defender count, starting owner, advantage percentage**… **min/max height so a zone is a volume, not
a disc**." Every one of those glosses is the INFERRED reading — `wog.md §7` is explicit that
"Parameter **semantics are INFERRED:** from names and value distributions" and `wmt_main` is absent
from the corpus. A reader of the five decisions alone (the likely operator path) receives inferred
semantics as measured fact; the correction arrives 390 lines later. Same shape twice more:
"neither community appears to know" (Decision 2, from README) is unknowable from these artifacts —
no forum, tracker, or member statement was read for either community (`wog.md §14` says so
outright); and §A.11 presents the corpus's **zero** instances of tagger output as *corroborating*
"duplicates on every save," when zero output actually evidences the opposite (the tagger never runs
on shipped saves — `wog.md §14.1`'s own INFERRED explanation: makers predate the feature or don't
save with the addon loaded).

**F5 — The synthesis violates its own stated measurement rules where the number is quotable.**
Its front-matter caveat: any corpus-wide WOG statistic is over a **mixed** population (78 native +
33 OFCRA + 60 third-party) "unless it says WOG-native." Then §A.8, §C.3-B7 and Decision-adjacent
text repeat "**74 of 171** missions disable your default… **if 43% of missions disable your
default, the default is wrong**." The 93 non-WOG missions mostly have no reason to touch a `wog3_*`
global (`wog.md §15.1`: the `wog3_` string appears in exactly 74 missions; `§15.6.2`: the setting is
part of the standard WOG-native 3-line preamble). On the honest denominator the rate is ~74/78 ≈
**95% of WOG-native missions** — the conclusion gets *stronger*, but the quoted number violates the
methodology two hundred lines above it, in the exact way the caveat was written to prevent. Same
class: "the last authoring action is always *generate a summary*" (§A.3, Appendix "4/4") does not
match the cited workflows — FNF v4's generator is step 13 of 16 (export is 14), OFCRA's
`table_forum.sqf` is an in-game pause-menu export to the website's mission manager
(`ofcra_omtk.md:201, §11.2`), not an authoring step at all, and WOG's counter is a menu tool used
when naming. And §D.3's "the four primitives cover **all 21** live-evaluable rules" over-counts:
`fnf_tooling.md §3.2`'s own coverage table assigns 19 rules (R1–R9, D1–D10) to V1–V4; P3, D11, D12,
D13 appear in no primitive row.

**Calibration notes, both directions.** (1) The one confirmed fabrication is handled *well*: §D.2's
verification claim — that "cheapest"/"highest-value" occur in the five analyses only at
`fnf_tooling.md:403` and `:861` — was re-run and is **exactly correct**. (2) A second instance of
the fabrication class was found in this pass's own tasking: the claim that `fnf_tooling.md` "triages
validation rules 40 live / 5 on-save / 7 post-hoc" and thereby contradicts its own 'nothing is
genuinely post-hoc' conclusion. **No such triage exists in any artifact.** The document's actual
markers are 21 live / 2–3 on-save / 0 post-hoc / 4 n-a across 27 checks (grep: 26 `**live**`, 3
`**on-save**`, zero post-hoc assignments), which is *consistent* with its §1.5 conclusion — the
alleged contradiction is a phantom that arrived via a brief, exactly like the fabricated quote. The
synthesis inherits the **correct** conclusion here.

---

## Contradictions

| # | Claim A | Doc | Claim B | Doc | Adjudicated? | Is the adjudication sound? |
|---|---|---|---|---|---|---|
| 1 | Eden semantic layers ranked #1; "copy the idea wholesale" | `fnf_v3.md §13.1` | FNF deleted the entire mode architecture; replacement is "the single largest expressive gain" | `fnf_v4.md §15.2b, §15.5.3` | Yes — `framework_synthesis.md §D.1` | **Substantively sound, headline overstated.** v4's outcome evidence beats v3's recommendation, and §D.1(c) correctly salvages conditional inclusion. Two defects: (i) §15.5.3's "largest gain" praises composable objectives *replacing fixed modes* — it does not directly indict layer semantics; the two axes are conflated. (ii) Decision 5's row "Its own authors deleted it" is contradicted by §D.1(a)'s **own table**: v4 still uses layer *names* as hard contracts (`Info`/`Selectors`/`Units` blacklisted from re-export, empty-layer GC) — mode-as-layer died; layer-name-as-contract did not. The conclusion (K7: keep TBD layers workflow-only) still stands. |
| 2 | F1=Units, F2=Groups | `batch05 §F-tabs` (explicitly marked inference from icons) | F1=Objects, F2=Compositions (tooltips captured) | `batch06` frames `_165952`, `_165959` | Yes — `eden_screenshots/README.md` | **Sound.** Direct tooltip capture beats icon inference; batch05 self-flagged; the `Place vehicles with crew` cross-check is real (batch06 verifies it exists only on F1). One overstatement: README says "F3–F6: both batches agree" — batch06 never activated F6 ("not hovered… never activated in this batch"), so F6=Markers is agreement between two *inferences*, weaker than F3–F5's tooltip proof. |
| 3 | "Contours are labelled with heights" (operator recollection) | dispatch premise | Every annotation is a spot height; ~50 read values are not multiples of any interval | `batch08 §Height labels` | Yes — `eden_screenshots/README.md` + `editor_ui_program_plan.md` Finding 1 | **Sound and well-argued** — the not-multiples-of-interval test is decisive; the refutation propagated correctly into T-641 (spot heights, not contour labels). |
| 4 | R1's fix-it examples **match** R1 (verified table) | `fnf_tooling.md §1.3` | "Fix-it examples for R1 and R2 do not satisfy R1 and R2" | `fnf_tooling.md §3.1`, inherited by `framework_synthesis.md` Decision 3 + §C.4 | **No** | The table is the evidence; the summary is wrong; the synthesis cites the table while repeating the summary. Only R2 self-fails. See F2. |
| 5 | Triage 21 live / 2–3 on-save / **0 post-hoc** | `fnf_tooling.md §1.3, §1.5` | Alleged "40 live / 5 on-save / 7 post-hoc" triage contradicting "nothing genuinely post-hoc" | **no artifact** (brief-level claim) | n/a | **Phantom.** The 40/5/7 split appears nowhere in the corpus. `fnf_tooling.md` is internally consistent on this point and the synthesis (§D.3) inherits the right conclusion. Recorded as a second instance of the chat-summary-becomes-fact class. |
| 6 | Tagger "append branch duplicates on every save" | `wog.md §14.1` (code path) | Zero instances of tagger output in 171 missions | `wog.md §14.1` corpus scan | Partially — wog.md itself reconciles via INFERRED ("makers predate the feature or do not save with the addon loaded") | wog.md is fine. **The synthesis's use is not**: §A.11 presents zero-output as "corroborated by" the duplication bug. Zero output corroborates *the tool never running*, not the duplicate-on-save behaviour; if it duplicated on every save the corpus should contain stacked tags. Direction of evidence inverted. |
| 7 | "No ticket system exists anywhere" / tickets "4/4 absent" | `framework_synthesis.md` §A.8, Decision 5 | v3's Sustained Assault mode is *described in-source* as ticket attrition ("each side starts with a set number of tickets… reaches 0 the opposing side wins", `sekrit.sqf:96-99`); implementation unread (binarized) | `fnf_v3.md §8` | **No** | Unadjudicated omission. Weak counter-evidence (text, not read code; SA is a gated side-mode), but the synthesis's absolute "anywhere" is contradicted by its own source, and the do-not-build row should carry the caveat. C.4's recommendation (delete/annotate TBD's dead `tickets` enum) is unaffected — it stands on TBD-internal grounds. |
| 8 | "All four are attack/defend at heart" | `framework_synthesis.md §A.7` conv. 2 | OFCRA's one mode has "no sudden death, no early win, **no attacker/defender asymmetry** beyond what the maker encodes in objectives" | `ofcra_omtk.md §7` | **No** | The OFCRA evidence cited (per-side DSL field, DIFF capzones) shows *per-side objectives*, which is Convergence 1's claim, not attack/defend genre. OFCRA is symmetric points-at-deadline by its own analysis. 4/4 should be at most 3/4 (and FNF counted once). |
| 9 | "3 of 15 Kit Mission Files remain" | `fnf_v4.md §14.10` | "Four of the seven kit-authoring workspaces were deleted" (3 survive) | `fnf_v4.md §15.6.9` (synthesis §A.5 uses this one) | **No** | Internal fnf_v4 inconsistency (15 vs 7), never reconciled. Same doc also says "45 typed attributes" (§15.4) vs "46 distinct names / 62 live instances" (§10.5, the grep-derived figure the synthesis correctly uses). Minor, but both live in sections Part B leans on. |
| 10 | "All three communities are Arma 3" | `framework_synthesis.md:7` | "Five artefacts, **four communities**" | `framework_synthesis.md:73, :215` (Decision 4, §A.4) | **No** | Internal self-contradiction in the synthesis, on its single "strongest convergence" claim. README (:28) sides with three. |
| 11 | Batch05 tooltip-verified toolbar: x220–239 = `Area Scaling Widget (4)`, x240–259 = `Area Widget (5)`; footer = Delete/…/lock/hide (inferred) | `batch05` | Batch06 infers x216–229 "Scale widget (object scaling, recent 2.x)", x242–256 "unidentified"; batch08 infers "Scale" + "transform pivot". Batch06 tooltips prove footer = New Layer / **Move to Root** / **Toggle Layer Transformation** / Toggle Layer Visibility (batch05's disable/lock guesses wrong) | `batch06`, `batch08` | **No** — README's corrections section records neither direction | The supersession README applied to F-tabs (tooltip beats inference) applies symmetrically here and is unrecorded: batches 06/08 are wrong on two widget buttons batch05 had proven, and batch05 is wrong on two footer buttons batch06 proved. No downstream ticket rests on the wrong values, but the "read this README before trusting any batch" contract is incomplete. |
| 12 | Chip row "BLUFOR / OPFOR / Independent / Civilian / **Props**, plus a sixth **Custom** slot… only under **Groups**" | `editor_ui_ticket_drafts.md` T-646 | Chips are BLUFOR / OPFOR / Independent / Civilian / **Empty** / **Logic**; sixth chip is "F2 only"; F2 is **Compositions**, not Groups | `batch06` (+ README correction) | **No** | T-646 carries pre-screenshot terminology (likely from `gap_analysis.md`) that the batch evidence superseded; "Groups" is the name README explicitly corrected. Structural fact (6th chip only on F2) survives; labels don't. |
| 13 | Corpus-statistics caveat: mixed population unless "WOG-native" | `framework_synthesis.md:25-28`, `wog.md §15.1` | "74 of 171 = 43% of missions disable your default" used as the design lesson | `framework_synthesis.md §A.8`, §C.3-B7 (inherited from `wog.md §14.2`) | **No** | Self-inconsistency; honest denominator (~74/78 native ≈ 95%) strengthens the conclusion but changes the quoted number. See F5. |
| 14 | 20 m @ ~6.20 m/pix — "batch 07 and batch 08 **independently**" | `eden_screenshots/README.md` table | Batch08's ~6.20 row is frame `170028` — a **batch07 frame** re-measured; its m/pix is *derived* (status field reads a plain distance there) and interval "(inferred)" | `batch08 §Contour interval` | Partially (README carries "±1 step" prose) | Two methods on the **same frame** is method-independence, not data-independence; the README table drops batch08's derived/inferred markers. The ladder *behaviour* remains solid (three genuinely different frames at 1.30/3.41 + the ring-count cross-check). |

---

## Synthesis traceability — the five decisions and the ranked list

### Decision 1 — objectives as typed, placed, per-side entities with one attribute spine
**Traced:** `wog.md §7` (166 instances / 73 missions, full parameter+value tables — verified,
matches to the counts), `wog.md §15.5` (165/166 `Condition=true` — verified), `fnf_v4.md §7`
(7 modules, per-side value pairs — verified), `fnf_v4.md §14.4/§15.6.5-6` (pairing defects —
verified), steal-list #6 ("one object with per-side framing" — verified verbatim).
**Verdict: the best-supported of the five, with two caveats.** The parameter *names, values and
adoption* are hard corpus data and genuinely strong. (i) The *semantics* of every parameter are
INFERRED (`wmt_main` absent) — carried in §A.7/U1, laundered in the Decision-1 headline (F4).
(ii) "Arrived at the same answer **independently**" is plausible but unexamined: both frameworks
express objectives as Eden modules because *Eden modules are the only placed-logic affordance Arma
offers* — platform-forced convergence is the alternative hypothesis the synthesis never addresses,
and its own §A.4 evidence (WOG marks INFERRED that even `Role@Group` is Arma's own lobby separator)
shows the ecosystem seeds conventions. Strength claim: slightly over; substance: solid.

### Decision 2 — live validation as a ticket group; every rule ships with a fail-on-demand test
**Traced:** "4/4 no CI" quotes verified verbatim in all four analyses; Analyzer D1/D2 dead-rule
mechanics verified (`$MarkObjs` typo, array-of-arrays guard, `fnf_tooling.md §1.3 C`); WOG tagger
`/g` verified (`wog.md §14.1`); TBD server-side-400-only claim consistent with §C.0 (tree-level
verification is outside this pass's lens).
**Verdict: conclusion robust — evidence overstated in three places.** "Four of four" is three
communities (F1). "Neither community appears to know" is unsupportable from artifacts (F4). The
zero-output "corroboration" inverts evidence direction (row 6). None of this endangers the
decision: TBD-internal grounds (the maker can author what the server 400s) carry it alone, and the
fail-on-demand requirement is the corpus's genuinely best-earned lesson (two independent silently
dead validators — the one convergence in the corpus that is *both* real and 2-independent).

### Decision 3 — generate, don't validate
**Traced:** v3 lobby generator verified (`fnf_v3.md §13.7`, §3 step 12); v4 Generate Lobby
Description verified (§11, §3 step 13); WOG counter + 94% verified (`wog.md §12.1, §13.2`); OFCRA
`table_forum.sqf` verified to exist (§11.2) — as a pause-menu export to the website's mission
manager.
**Verdict: directionally supported; the two decorating claims degrade under checking.**
(i) "Last authoring action, 4/4": v4's is step 13 of 16, OFCRA's is not an authoring step, WOG's is
a naming aid — the honest claim is "all four *derive* a summary from ground truth," which is enough.
(ii) The Analyzer negative case is half-false (F2: R1's examples pass). (iii) "94%-consistent
convention out of a 10-line function" overclaims causality — `wog.md §12.1` credits the counter
*plus* the 3den extension pre-seeding the filename format as every new mission's default name
("Tooling and convention co-designed"); the seeded default is at least co-causal. The design
principle itself is fine and R2 alone proves the negative case.

### Decision 4 — role/squad/rank/traits as separate typed fields; derive display strings
**Traced:** the `@`-packing mechanism verified in all five artefacts (parser locations check out:
`fn_onMissionSaveEH.sqf:3`, `fn_rosterBriefing.sqf:83-89`, `AnalyzeSQM.ps1:448-449`, etc.); the
second-order mangling table verified (v4 trailing-space side padding `init3DEN.sqf:154`; WOG `1: `
prefix and ` | Med` append; OFCRA rank from Eden's field).
**Verdict: right conclusion, inflated arithmetic, unexamined independence.** "Five artefacts, four
communities" is wrong (F1; three communities; three of the five artefacts are FNF's). The five
codebases *do* all contain the convention — but `wog.md §4a` itself flags (INFERRED) that the `@`
split may be **Arma's own MP-lobby role/squad separator**, i.e. an ecosystem-inherited convention,
not five independent inventions; the synthesis never engages this, and "all four diagnose it
identically" partly reflects that all five analyses were written to the same 14-section
questionnaire by the same program. Materially harmless: TBD already has the typed fields, so the
decision costs nothing even at N=1.

### Decision 5 — the four deliberate do-not-builds
**Traced and assessed row by row:**
- *No play-time arsenal* — quotes verified in all four; N=3 communities; also both FNF eras ship a
  bounded pre-start selector, which §A.5 honestly covers. **Sound.**
- *No ticket system* — contradicted at the margin by `fnf_v3.md §8`'s SA description (row 7);
  "4/4" is 3 communities with one in-family exception recorded and unread. The TBD action item
  (dead `"tickets"` enum) is sound regardless. **Sound conclusion, overstated premise.**
- *No semantic editor layers* — adjudication §D.1 substantively sound; the table row's "its own
  authors deleted it" overstates (row 1). **Keep K7; fix the sentence.**
- *No positional/derived identity* — rests on one framework's defect + TBD-internal architecture.
  Honest N=1; the reasoning is TBD-first, not convergence-first. **Sound.**

### The ranked build list (§C.5)
Ranks 1–9 (B1, B2, B3, U4, U5, U2, U7, U6, B7) are all small/medium items whose justification is
substantially TBD-internal (panel-first ordering, data already in the doc, existing schema hooks);
they survive every finding above. Specific notes: **B1**'s "four primitives cover all 21" is
arithmetically 19 (F5) — immaterial to rank, material to T-656's acceptance wording. **B7** rests
on the mis-denominated 43% (F5) — the honest ~95%-of-native figure makes it *stronger*. **U1 (rank
10)** and **B5 (rank 13)** correctly carry the INFERRED-semantics caveat at point of rank — the
uncertainty is priced in exactly where it matters. **B9 last** with "weakest evidence in the set"
stated — honest. The list's *ordering logic* (value ÷ effort, cheap-and-internal first, mod-gated
later) does not depend on any inflated convergence count; the two items that would move if the
convergence audit were taken seriously (B2's "4/4", B4's uniform-section "2/4 independently") move
by zero or one rank. **The ranking survives.**

---

## Convergence audit — every N/4 claim, checked for independence

Baseline: FNF v3, FNF v4 and the Analyzer/DTAS are one community (README:28; DTAS is adopted 2013
third-party code, so its *authoring* choices are not even FNF's). Max independent N = **3**.

| Appendix claim | Stated | Actual independent support | Notes |
|---|---|---|---|
| Role+group packed into description | "5 artefacts / 4 communities" | 5 codebases / **3 communities**, independence unestablished | wog.md itself INFERRED-flags the `@` as possibly engine convention; common F2/F3-era ancestry never checked. Mechanism presence in all five codebases is verified and real. |
| One life, no tickets | 4/4 | **3 communities**, one recorded in-family exception (v3 SA), OFCRA offers lobby respawn for training | Convergence real for the *default match mode*; overstated as absolute. |
| No player-facing arsenal | 4/4 | **3** | Real; FNF's bounded selectors honestly footnoted in §A.5. |
| No CI / linter / schema / tests | 4/4 | **3** | Real and verbatim-verified; FNF counted twice adds nothing (same repo hygiene, same org, same `.github`). |
| Medical taken from the maker | 4/4 | **2 enforce** (FNF 518 forced settings; WOG forced kit). OFCRA's §8: "ships no medical system… defers entirely to ACE" — an **absence counted as agreement**; nothing in omtk takes anything from the maker | The claimed 4/4 contains one double-count and one non-observation. Honest form: 2 of 3 communities actively standardise; the third simply has no medical layer. |
| Last step generates summary to clipboard | 4/4 | **3 communities derive summaries**; "last authoring action" false for 2 of 4 workflows (v4 step 13/16, OFCRA post-hoc website export) | See Decision 3. |
| Briefing mostly derived | 3/4 + WOG hand-rolls same template 80× | **2 derive** (FNF once + OFCRA); WOG counter-instance honestly used as convergence-by-another-route — this row is *well* handled | Good example of the honest form the other rows should take. |
| Config text-file → typed attributes, "with a direction" | 2 crossed, 1 stayed | Direction rests on **one community's arc** (FNF) + WOG's INFERRED end-state (marker carried) + OFCRA-as-friction cross-checked by the 33 in-corpus missions | The 33-mission cross-check is genuinely independent and is the best evidence move in the synthesis. |
| Objectives = typed placed entities | 2/4 independent | 2 communities; platform-forcing unexamined (F/Decision 1) | |
| Attack/defend at heart | 4/4 | **2–3**: v3 code guard ✓, v4 value pairs ✓, WOG default text + 52/42 ✓; **OFCRA contradicted by its own §7** (row 8) | Overcounted by ≥1 even before the FNF double-count. |
| Triggers abandoned for regions | "2/4 outright, 1 mixed, 1 ambiguity" | As stated (2 = v4 + WOG; v4 and v3 differ, so FNF's double-counting here is at least *informative*) | Honest row. |
| Uniform-recognition section | 2/4 "invented independently" | 2 communities, but 33 OFCRA missions live in WOG's own corpus — cross-pollination is live and unaddressed; "independently" asserted | Plausible, unproven. |
| Per-side briefings | 2/4 + 1 names absence | Accurate | |
| Bounded safe-start loadout choice | 2/4 "(both FNF eras)" | **1 community** — and the synthesis says so in-line | Honest. |
| Declarative params used, escape hatch not | "1 framework, 165/166" | Honest N=1 with strong internal data | |
| Editor comments | "1 community, 2 eras — *not* a convergence" | Honest | The model row. |
| Both validators silently dead, same shape | 2 independent tools | **Genuinely 2 independent communities** — the strongest true convergence in the corpus | Real. |

**Summary:** no "4/4" row is four independent observations; two rows (medical, attack/defend) count
non-evidence; the Appendix's own honest rows (comments, selectors, WOG-counter) prove the authors
knew the correct form. The convergence *substance* survives at N=2–3 nearly everywhere; the
*rhetoric* of unanimity does not.

## INFERRED laundering — where uncertainty was dropped

| Where | What happened | Severity |
|---|---|---|
| Decision 1 headline | `WMT_Task_Point` semantics ("attacker count… zone is a volume") stated as fact; INFERRED carried only in §A.7/U1/C.5 | **High** — it is the executive summary of the #13-ranked large build (B5) and U1 |
| §U1 "why" column | "The volume is what excludes aircraft and includes basements" asserted, then the same row says semantics are INFERRED | Medium — self-inconsistent within one table cell |
| Decision 2 / §A.11 / README | "neither community appears to know" — no artifact can support it (wog.md: no tracker/forum read; Analyzer repo dormant) | Medium — colour, but repeated three times as fact |
| §A.11 | Zero corpus output presented as corroborating "duplicates on every save" — inverts what zero output evidences; wog.md's own INFERRED reconciliation dropped | Medium |
| §A.10 | "29 call sites read params never declared, **so** LR radios/nav gear silently resolve to absent" — the consequence clause is inside wog-style INFERRED in `fnf_tooling.md §2.4`; restated unmarked (mechanically near-entailed, mitigating) | Low |
| §B.3 table #6 | CfgPatches gap consequence is INFERRED in `fnf_v4.md §10.14`; §A.14 carries the marker, the B.3 table does not | Low |
| eden README interval table | batch08's "derived m/pix" + "(inferred)" markers on the 20 m row dropped from the table (±1-step caveat survives in prose) | Low |
| Carried correctly (for the record) | WOG die-once-spectate (§A.8), OFCRA `inArea` ambiguity (§A.9), `loc_Fuelstation` (K10), v4 init-field-only hidden attrs (§A.7), wog description.ext migration reason (§A.2), C.5 row 10 | — the body discipline is genuinely good; the failure mode is specifically *headline altitude* |

## Fragile conclusions — single-source, uncorroborated

1. **All semantic content of U1/B5's capture model** — one document (`wog.md §7`), reconstructed
   from usage because the defining addon (`wmt_main`) is absent from the corpus; flagged, but
   nothing can corroborate it short of running WOG's server. The parameter *set* is safe; the
   *behaviour* is a design guess.
2. **`MinHeight=-5` = "basements in, aircraft out"** — a single inference on an invariant value
   (invariant values are also consistent with "nobody ever touched the default").
3. **WOG counter → 94% convention causality** (B2's flagship anecdote) — one doc; co-caused by the
   seeded default mission name per the same doc.
4. **The 43% / "default is wrong" lesson behind B7** — one doc, wrong denominator (F5).
5. **v3's "three hand-maintained copies of a 516-line settings file, divergent in seven values"**
   (§A.1's v3 cost cell) — a one-clause claim from the v3 *mods sub-agent* with no file:line, inside
   the one analysis the README flags for unenforced sub-agent labelling and zero INFERRED markers.
   Several other A-table cells (armor ×282 counts, briefing-table spawning) share this provenance;
   they are consistent and detailed, but they are exactly the "second-hand from workers" content the
   README warned about, and the synthesis leans on them without repeating the flag outside §D.1/D.2.
6. **OFCRA loadout-inheritance payoff for U3** — one document; the 579× explicit-null count and
   eleven-rule enumeration were not independently checkable in this pass (source is a scratch-disk
   YAML corpus).
7. **"~5 years" and "a tool people trusted"** for the Analyzer's dead rules — the code defects are
   rock-solid (quoted source); the age (shallow clone, tip only) and the community-trust claim are
   single-inference garnish.
8. **B.5 #1's `CONN-SYNC-001` warning** — depends on `gap_analysis.md`, outside this corpus;
   unverified here.
9. **Eden F2=Compositions** — effectively single-capture (one tooltip frame) but with two honest
   corroborations (checkbox behaviour, `ui_anatomy.md`); the *least* fragile of the single-source
   set. Registry facts in §D.6 (max id T-626, T-627–630 absent, T-074 cancelled, T-072/073/075/077/
   078 statuses) were **re-verified against `registry.json` this pass: all correct**.

## Selection bias and over-generalisation

- **The corpus is one genre, chosen by affinity.** `frameworks/README.md:10` says it plainly: "The
  operator plays in / follows all three." All three are Arma 3, large-slot, one-life,
  ACE+TFAR-stack, event-based milsim communities. Every "do not build" (arsenal, tickets, respawn
  authoring) is a property of *that genre*, not of mission frameworks: a fourth framework drawn from
  mainstream Arma (KOTH, Invade & Annex, Antistasi/Liberation-style persistence) would show tickets
  or wave respawn, play-time arsenals as standard furniture, non-attack/defend economies, and
  persistent-campaign state — flipping three of Decision 5's four rows at N=4. That is acceptable
  *because TBD is deliberately the same genre* — but the synthesis states it as corpus fact
  ("the clearest 'do not build' in the corpus") rather than genre fact, and only §"What this
  synthesis does not cover" gestures at the boundary.
- **Platform-forced convergence is never controlled for.** Eden modules as the only placed-logic
  affordance (Decision 1's "independent arrival"), one free-text description field (Decision 4's
  packing), clipboard as the only tool output channel (§A.11 convergence 3 — "4/4 deliver to
  clipboard" is 100% an Arma-affordance artefact, not a design preference), marker polygons as the
  only free geometry. Several "convergences" are the platform's fingerprint, and a web-native
  fourth data point would dissolve them. The synthesis draws correct *lessons* from them anyway
  (typed zones, persistent panel) — the lessons happen to be right for reasons the convergence
  framing doesn't supply.
- **Analyst convergence ≠ artifact convergence.** All five analyses were written to the same
  14-section schema by the same program in one day; "all four diagnose it identically" (§A.4) is
  partly the questionnaire diagnosing it identically four times. The raw mechanisms are real;
  the shared *framing* of them is not independent evidence.
- **The FNF double-count is load-bearing for the rhetoric.** "4/4" appears seven times in the
  Appendix; every instance is 3 communities, and the README's own provenance rule (:28) forbids the
  count the synthesis uses. Where the synthesis needed a fourth independent point it had a genuinely
  better move available and used it exactly once: the 33 OFCRA missions inside WOG's corpus (§A.2)
  — real cross-community corroboration. That standard, applied once, shows what the other claims
  lack.
- **Where over-generalisation does *not* happen:** Part B is scoped to "what the arc validates in
  TBD's architecture" and §B.6's verdict is explicitly conditional; §A.10 down-ranks parameters for
  genre mismatch; B9/B8 are ranked low with the weakness named; runtime/social enforcement is
  excluded wholesale. The synthesis's *scoping* instincts are consistently better than its
  *counting*.

## Circularity — checked, mostly clean

- The synthesis **does not** cite `editor_ui_program_plan.md` or `3den_enhanced_feature_catalogue.md`
  as evidence anywhere (grep-verified; the only drafts reference is Part D's header, which treats
  them as *targets* of feedback, correctly noting they predate the research). The feared
  plan→synthesis→plan loop does not exist.
- `frameworks/README.md` ↔ analyses: the README's cross-analysis findings (both-validators-dead,
  mixed corpus) are derivative aggregations of the analyses, and §A.11 cites the README as "the
  most important single fact in the corpus" — derivative citation, not circular; adds no
  independent weight and should not be read as a third source.
- The eden README ↔ batch docs: corrections flow one way; clean.
- Residual one-way staleness rather than circularity: drafts T-646 carries pre-correction
  terminology (contradiction row 12).

## Verdict on the known-calibration item

§D.2's handling of the fabricated quotation is **accurate and complete**: the phrase appears in no
artifact; the synthesis's two claimed near-miss locations (`fnf_tooling.md:403`, `:861`) are exactly
right; the recommendation it decorated survives on its real citations. The practical rule §D.2
derives ("a claim is sourced when it is in an artifact with a citation; an agent's summary is a
pointer to work, not evidence of it") is the correct generalisation — and this pass found the same
error class twice more operating *at the reasoning level*: the §3.1-over-§1.3 summary inversion
(F2, inside an artifact) and the phantom 40/5/7 triage (in a brief, outside all artifacts). The
class is alive; the corpus's defence against it (check the table, not the sentence above it) worked
each time it was applied.
