# Editor factory — run log

Program: waves 100–126 (77 tickets), authority `docs/platform/EDITOR_FACTORY_START.md`.
Run started 2026-08-02 from 852f17a4 (plan adversarially verified: 20/20 invariants).

**Close-marker numbering:** the wave-close ledger (wave.sh oracle 1) requires marker numbers to
advance by exactly one from `wave 82 CLOSED` (c2dac546). Editor wave L therefore closes as marker
M = L − 17 (100→83 … 126→109), with the editor label in the free text:
`wave M CLOSED — editor wave L: …`. Oracle 2 cannot corroborate markers 83+ (plan rows carry the
100+ labels), so wave gates after 100 pass `TBD_GATE_BASE_CONFIRM=<prev close sha>` after
verifying the sha by hand. This is the documented hatch, not a bypass: membership is confirmed by
the operator-side reading the tooling demands.

**Standing env on every wave.sh / cargo invocation:**
`CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target` (never /tmp) ·
`TBD_WAVE_GENERATION_FLOOR=100` (aim current_wave at this program, not the legacy backlog).

**Shape per wave:** worktrees → ≤3 slice agents (Opus; Fable for `.c` under
apps/mod/tbd-framework/) → BARRIER all report → merge all → `wave.sh gate` → ONE Fable
adversarial verifier on merged main → triage (BLOCKERs fixed in-wave, rest deferred with
diagnosis) → registry flip + `distrobox-host-exec sh -c './scripts/ticket sync'` → close commit.

| Wave | Marker | Tickets | Gate | Verifier | Outcome |
|---|---|---|---|---|---|
| 100 | 83 | T-661 | — | — | in flight |
| 101 | 84 | T-639 T-662 T-663 | — | — | pending |
| 102 | 85 | T-640 T-664 T-665 | — | — | pending |
| 103 | 86 | T-076 T-631 T-641 | — | — | pending |
| 104 | 87 | T-635 T-656 T-666 | — | — | pending |
| 105 | 88 | T-636 T-646 T-683 | — | — | pending |
| 106 | 89 | T-647 T-667 T-691 | — | — | pending |
| 107 | 90 | T-638 T-657 T-659 | — | — | pending |
| 108 | 91 | T-642 T-650 T-658 | — | — | pending |
| 109 | 92 | T-079 T-643 T-660 | — | — | pending |
| 110 | 93 | T-644 T-648 T-668 | — | — | pending |
| 111 | 94 | T-645 T-655 T-693 | — | — | pending |
| 112 | 95 | T-649 T-686 T-692 | — | — | pending |
| 113 | 96 | T-082 T-669 T-694 | — | — | pending |
| 114 | 97 | T-633 T-651 T-695 | — | — | pending |
| 115 | 98 | T-634 T-670 T-688 | — | — | pending |
| 116 | 99 | T-069 T-690 T-696 | — | — | pending |
| 117 | 100 | T-084 T-671 T-672 | — | — | pending |
| 118 | 101 | T-637 T-698 T-699 | — | — | pending |
| 119 | 102 | T-697 T-700 T-703 | — | — | pending |
| 120 | 103 | T-701 T-706 | — | — | pending |
| 121 | 104 | T-702 T-212 T-654 | — | — | pending |
| 122 | 105 | T-673 T-674 T-675 | — | — | pending |
| 123 | 106 | T-676 T-677 T-678 | — | — | pending |
| 124 | 107 | T-679 T-680 T-681 | — | — | pending |
| 125 | 108 | T-682 T-684 T-685 | — | — | pending |
| 126 | 109 | T-689 T-705 | — | — | pending |

## Deferred tickets filed by verifiers

(none yet)

## Incidents

(none yet)
