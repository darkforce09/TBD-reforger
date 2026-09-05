# Audit 2026-09 — claims verified FALSE (no ticket minted)

Verified against main @ 072988d57 on 2026-09-04. Each row: what the audit said, what the code does, anchor.

| # | Section | Audit claim | Reality | Anchor |
|---|---|---|---|---|
| 1 | S5 | Event registration has no capacity check | ALREADY FIXED by T-227 (shipped): registration counts orbat_slots and returns 409 when capacity is 0 | apps/website/api/src/handlers/events/events.rs:1768-1772 (count), :1794-1798 (409), :1781-1793 (comment) |
| 2 | S6 | Safestart TickCountdown loops forever | FALSE: `next <= 0` calls GoLive; `!m_bArmed` guard exits; stage-drift lifts the countdown — three exits | apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SafestartManager.c:389-417 (:391-395 guard, :397-405 drift lift) |
| 3 | S5 | Wiki parser enforces limits that break content | FALSE as worded: wiki.rs has no limits at all; the real gap is missing features (H3+, links, images, tables, checklists, revisions) — ticketed by the S4–S6 planner, not here | apps/website/api/src/handlers/content/wiki.rs |

Not false, but not confirmed either: S3 "10+ new buffers per chunk crossing" at world_host.rs:454-525 is
UNVERIFIED — T-938.2 measures it first and ships only a counter if the number is below 3 per crossing.
