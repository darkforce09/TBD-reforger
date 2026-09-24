# Vehicle Database

## Status

`doc-complete` — populated vehicle dossiers (T-263) with image failure fallback (T-939.4 recovery).

## Summary

- **Route:** `/vehicles`; the page uses `AuthGate`.
- **Live source:** `apps/website/frontend/src/pages/public/vehicles.rs`.
- **Purpose:** search faction-grouped vehicles and inspect their recorded armor, amphibious and threat information.

## Behavior

`GET /vehicle-database` loads the vehicle list as `{data:[...]}`. A `GlassSplit` shows an 18rem master column and the selected dossier. The first row is selected initially. Search matches vehicle name, armor type and faction; faction headings follow first-seen order and disappear when no rows match.

The dossier displays stored values without inventing capacity or tactical directives. Absent or failed profile imagery uses the existing vehicle icon placeholder. Each selected dossier starts a fresh image attempt, preserving valid artwork and image-area dimensions.

Loading and fetch failures have separate text states. An empty database displays “No vehicles in the database.” in the detail area.

## Verification

Verify populated faction groups, search and selection against the API fixture. Exercise failed imagery, absent imagery, a valid image and selection from a failed image to a valid one. Dossier values and the image-area dimensions must remain intact throughout.
