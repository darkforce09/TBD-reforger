# Session/PostGame

Match resolution overlays, victory announcements, and post-game debriefing scoreboards.

### Roles & Responsibilities
- `UI/TBD_EndScreen.c`: Immediate end-of-round overlay banner announcing the victorious faction and reason for match end.
- `UI/TBD_DebriefScreen.c`: Detailed scoreboard overlay displaying player roles, kills, deaths, and team outcomes.

### Call Flow & Contracts
Stage transition to `POST_GAME` or `DEBRIEF` in `TBD_FrameworkManager` -> triggers overlay opening -> screens read cached match data from `API/TBD_ResultsReporter` and display stats without closing engine menus.
