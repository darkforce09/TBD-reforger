# Operator eye-pass checklist — September 2026 factory run

One line per shipped job, appended at each wave close. Run on `cargo xtask mk leptos` (release) at
1920×1080 unless the line says otherwise; mod lines need a headless boot or a live server as stated.
"Anything that feels wrong is a finding" — report it and it becomes a ticket, never an ad-hoc fix.

| Wave | Ticket | What to check | How | Result |
|---|---|---|---|---|
| 248 | T-311 | Leaderboard paging is stable across tied scores | Seed the golden (`apps/website/api/seeds/content_golden.sql`), open Leaderboards → Missions, page with limit 2: ranks 1–6 read Dev Operator, Rhodes, Vance, Okafor, Brandt, Kessler; no name appears on two pages. Restart the dev API first (the running one predates the slice). | |
| 248 | T-940.5 | DB pool knobs come from env | Restart the dev API with `TBD_DB_POOL_MAX_CONNECTIONS=abc`: it refuses to boot naming the variable. With `=3`, `SELECT count(*) FROM pg_stat_activity WHERE datname='tbd_reforger'` never exceeds 3 under load. | |
| 248 | T-940.6 | Audit console is live, three new row kinds | Restart the dev API; open the admin audit console; create an event in a second tab → `event.create` row within 1 s. Soft-delete a mission → `mission.delete`. Clear an occupied slot → `event.slot_kick`. `SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE query ILIKE 'LISTEN%'` → feed keeps flowing ≤ 2 s. | |
