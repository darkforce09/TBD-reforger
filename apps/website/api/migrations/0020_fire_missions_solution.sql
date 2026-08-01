-- T-587 — `fire_missions` can finally store the solution it was created to record.
--
-- Surfaced by T-285, which wired the two orphaned `/fire-missions` endpoints into the Mortar
-- Calculator and had to work around this table to do it.
--
-- ── WHAT THE TABLE COULD NOT HOLD ───────────────────────────────────────────────────────────
--
-- `services/mortar.rs::solve_fire_mission` computes SEVEN numbers plus the four inputs it was
-- given. `fire_missions` (0001_initial_schema.sql:225-236) had columns for three of the seven
-- and none of the four:
--
--   computed        distance_m ✓   azimuth_deg ✓   elevation_mils ✓
--                   azimuth_mils ✗  charge ✗        time_of_flight_s ✗
--   given           fp_x ✗  fp_y ✗  tgt_x ✗  tgt_y ✗
--
-- The three missing computed fields are not decoration on a mortar. **`charge` is the ring the
-- crew physically sets on the round** — an elevation without it is not a fire order, it is half
-- of one, and the other half is unrecoverable. **`azimuth_mils` is what the sight is actually
-- dialled in**; the stored `azimuth_deg numeric(5,1)` is the human-readable echo, not the
-- setting. **`time_of_flight_s`** is what the observer counts down to splash. All three are
-- computed on every solve and all three were discarded on the way to the INSERT.
--
-- The visible symptom, and the one the ticket names: a freshly-saved solution shows full TOF —
-- the `POST /fire-missions` response carries the live `FireSolution` — while the SAME row read
-- back after a reload shows `—`. The asymmetry is the schema's, not the UI's. The UI is being
-- honest; it has nothing to render.
--
-- ── WHY THE COORDINATES ARE THE SHARPER HALF ────────────────────────────────────────────────
--
-- The table stores no numeric coordinates at all. T-285 kept the operator's four numbers alive
-- by encoding them, losslessly, into the two free-text columns the table does have:
-- `frontend/src/mortar.rs::fmt_grid` writes `"1000, 2000"` into `fp_grid` and `parse_grid`
-- reads it back. That encoding IS the persistence of the inputs, which is why it carries a
-- round-trip test. It works. It is also a client-side private format sitting in a shared
-- column: any other writer of `fire_missions` — a second client, a game-server bridge, an
-- operator's `psql` — stores a grid reference that the calculator restores as "no
-- coordinates", and there is no column that would have held the truth.
--
-- ── THE COLUMNS, AND WHY EACH EARNS ITS PLACE ───────────────────────────────────────────────
--
-- Exactly the four inputs `solve_fire_mission` takes plus the three outputs it produces that
-- had nowhere to go. Nothing else: this is a flat-earth simplified-projectile model with no
-- wind, no altitude delta, no ammunition type and no propellant temperature, so columns for
-- those would be columns the calculator can never fill. A NULL that is always NULL is worse
-- than no column — it reads as "not measured this time".
--
--   fp_x, fp_y, tgt_x, tgt_y  double precision — the four `solve_fire_mission` arguments, in
--                             flat game-world metres. Re-issuing or re-checking a mission means
--                             re-solving it, and re-solving needs these. `double precision`
--                             rather than numeric: they ARE `f64` on both sides of the wire
--                             (`SolveInput`, `fmt_grid`), and a lossy column under a lossless
--                             text encoding would be a downgrade.
--   azimuth_mils              bigint — the sight setting. `bigint` to match the shipped
--                             `distance_m` / `elevation_mils`, and `FireSolution::azimuth_mils`
--                             is `i64`, so no cast is needed on read (unlike `azimuth_deg`,
--                             whose `numeric(5,1)` forces a `::float8` in every query).
--   charge                    bigint — the propellant ring index into the weapon's charge table.
--                             Small, but see above on casts; consistency beats two bytes.
--   time_of_flight_s          double precision — seconds to splash, `f64` in `FireSolution`.
--
-- `fp_grid` / `target_grid` are NOT dropped and NOT relaxed. They are `NOT NULL` today, both
-- halves of the shipped API contract (`SaveFireInput` requires them), and they carry the
-- operator's own text — which for a hand-typed six-figure reference is information no numeric
-- column holds. What retires is the DEPENDENCE on the encoding: `restore()` now reads the
-- numeric columns and falls back to `parse_grid` only for rows written before this migration.
--
-- ── NULLABILITY: NULLABLE, NO DEFAULTS ──────────────────────────────────────────────────────
--
-- Every column added here is nullable with no default, and the reason is 0014's reason
-- (`0014_nullable_match_player_stat_counters.sql`) applied to a mortar: a stored `0` claims to
-- be a measurement. `time_of_flight_s DEFAULT 0` renders `0.0 s` — a plausible, wrong,
-- unfalsifiable number that no reader can tell from a real zero-second flight. `charge DEFAULT
-- 0` names charge zero, a real ring on every tube in `charges_for`. `fp_x DEFAULT 0` is grid
-- origin, a legitimate coordinate; it is the same argument `handlers/field_tools.rs` makes for
-- why the four coordinates must be *present* rather than non-empty in the request body.
--
-- So NULL here means exactly one thing, and it is true: **this row predates T-587**. Reading
-- code must handle it — `models::FireMission` types all seven as `Option`, and the calculator
-- renders `—` for a missing TOF exactly as it did before this migration existed.
--
-- ── BACKFILL: THE COORDINATES, FROM THE ENCODING THAT WAS ALREADY HOLDING THEM ──────────────
--
-- The grid strings are the CURRENT persistence format, so this migration reads them rather
-- than stranding them. The accept test below is `parse_grid`'s, deliberately character for
-- character: optional whitespace, a signed decimal, a comma, a signed decimal. Agreeing with
-- the shipped reader is the whole criterion — a backfill that accepted MORE than `parse_grid`
-- would invent coordinates for rows the calculator has always shown as unrestorable, and one
-- that accepted less would drop rows it has always restored.
--
-- The two pairs are backfilled independently, each guarded on its own column, because each is
-- true on its own. A row where only one grid is this encoding gets one real pair and one NULL
-- pair, which is what happened.
--
-- Rows whose grids are anything else — `'012345'`, a real six-figure military reference, a
-- label — keep NULL coordinates. That is not a gap; it is the honest answer, and it is the
-- same answer the calculator gives that row today.
--
-- **`azimuth_mils`, `charge` and `time_of_flight_s` are NOT backfilled, and must not be.**
-- They were never stored, and the only way to produce them for a historical row is to re-run
-- the ballistics — which would mean a second copy of `charges_for`'s muzzle-velocity table
-- transcribed into SQL. That is the exact drift this codebase has fixed twice (T-347, and the
-- `WEAPONS` duplication T-285 flagged), it would be frozen at today's constants while the row
-- was solved under whatever they were then, and it would produce a confident number for a
-- mission nobody re-checked. A computed backfill here would be this program's signature defect
-- written into the schema: a value reported over an input it never examined. NULL is correct.

ALTER TABLE public.fire_missions
    ADD COLUMN fp_x double precision,
    ADD COLUMN fp_y double precision,
    ADD COLUMN tgt_x double precision,
    ADD COLUMN tgt_y double precision,
    ADD COLUMN azimuth_mils bigint,
    ADD COLUMN charge bigint,
    ADD COLUMN time_of_flight_s double precision;

-- `parse_grid`, as a POSIX regex. `^\s*-?\d+(\.\d+)?\s*,\s*-?\d+(\.\d+)?\s*$` — `f64`'s Display
-- never emits an exponent, so `fmt_grid` output is always plain decimal. `btrim` before the cast
-- so the whitespace the regex tolerates cannot reach `float8in`.
UPDATE public.fire_missions
   SET fp_x = btrim(split_part(fp_grid, ',', 1))::double precision,
       fp_y = btrim(split_part(fp_grid, ',', 2))::double precision
 WHERE fp_grid ~ '^\s*-?\d+(\.\d+)?\s*,\s*-?\d+(\.\d+)?\s*$';

UPDATE public.fire_missions
   SET tgt_x = btrim(split_part(target_grid, ',', 1))::double precision,
       tgt_y = btrim(split_part(target_grid, ',', 2))::double precision
 WHERE target_grid ~ '^\s*-?\d+(\.\d+)?\s*,\s*-?\d+(\.\d+)?\s*$';
