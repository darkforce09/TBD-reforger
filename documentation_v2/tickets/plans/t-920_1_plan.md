# T-920.1 — main_goal rename + tiered gates: plan

## Context

user_story exists on 46/1195 tickets and its content is goal-shaped, not persona
prose; 99 tickets carry their id as their title and 340 titles exceed 10 words; body
fields are renderable but nothing forces them filled. Operator decisions 2026-08-15.

## Approach

Serde-alias rename user_story→main_goal through TicketFile, the typed model, ops,
check, sync and the 46 on-disk carriers (write_back migration). Tier rules in check
(idea/queued corpus-wide, ready-tier on ready-class) + pre-write refusals in
ops::mark_ready and ops::ship. Title gate in ops post-image for changed tickets.
Two shrink-only debt pins measured at land, T-917.3 ratchet pattern.

## Risks

- Rename breaking a Value-path consumer that greps "user_story" — sweep sync.rs,
  cmds.rs, board; alias covers parse, emit must be main_goal everywhere.
- Ready-tier rule redding the live ready set — the live ready tickets must be filled
  or demoted in this same slice, honestly.

## Verification

- Acceptance list on T-920.1: migration counts, roundtrip N/N, lock diff empty,
  refusal fixtures, pins == measured, strict OK, workspace build green.
