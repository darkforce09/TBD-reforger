# Hub 2: Operations (`src/v2/pages/operations`)

The lifecycle of a live operation: putting it on the calendar, briefing it, slotting into it, and
what the record looks like afterwards.

## Pages
1. **`schedule/` (`/events`)**: the operation list beside the hub of the one in focus.
2. **`event_detail/` (`/events/:id`)**: the operation dossier — hero, mission dossiers, faction
   cards and the inline slotting selector.
3. **`orbat_selection/` (`/events/:id/missions/:emid/orbat`)**: one mission's slotting, on a page
   of its own for direct links.
4. **`deployments/` (`/deployments`)**: the caller's own service record — active orders, combat
   history and leave of absence.
5. **`leaderboards/` (`/leaderboards`)**: the five global ladders and the operator dossier behind
   every row.

The schedule's detail column and the operation dossier render the same hub body, and the
standalone slotting page mounts the dossier's own selector — one implementation each, two places
each appears.
