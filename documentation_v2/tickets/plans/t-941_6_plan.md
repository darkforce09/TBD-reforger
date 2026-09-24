# T-941.6 — Plan

## Context
TBD_AdminCommands.c:41 handles `#tbd link <code>` after it reaches public chat; Backend/TBD_IdentityLink.c owns the flow.

## Approach
1. Verify on main: read :41 and the IdentityLink handler; record as defect evidence.
2. TBD_AdminCommands.c: consume the command before broadcast; suppress the echo; reply via SendPrivateMessage.
3. TBD_IdentityLink.c: code goes server→backend, never echoed; status replies private.
4. No filtering beyond the command prefix.
5. Perturbation: misspelled private-message method → compile red; restore, touch, green.

## Risks
- Chat hook ordering: the consume must run before the broadcast handler, verified on the checklist.

## Verification
- `cargo xtask mod compile`; `cargo xtask mod world-boot`
- `cargo xtask platform wave gate --slice T-941.6`
- Checklist: other players see nothing; sender gets a private confirmation; `#tbd link status` is private.
