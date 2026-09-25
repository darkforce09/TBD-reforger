**Status:** live

# Eden vehicle crew interactions

How the Arma 3 Eden editor shows a vehicle's crew and moves characters into, out of and between
its seats. Each entry follows the Eden reference format of the
[feature entry schema](/documentation_v2/website/frontend/apps/editor/feature_inventory/feds_schema.md#eden-reference-entries);
placing a vehicle with or without its crew is in [asset browser](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/interactions/asset_browser.md#right-crew-001--vehicle-crew-toggle)
and [entity placement](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/interactions/entity_placement.md#place-crew-001--empty-vehicle-alt-while-placing).

## CREW — Vehicle crew

#### CREW-PANEL-001 — Hover crew panel

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Transforming_Crew#Crew_in_the_editor_interface |
| **Trigger** | Hover vehicle icon |
| **Procedure** | Panel lists all crew roles (Driver/Pilot, Commander, Turret, Passenger); visible crew get extra scene icon |
| **Acceptance** | `- [ ] Crew list beside vehicle icon` |

#### CREW-BOARD-001 — Drag character into vehicle

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Transforming_Crew#Moving_crew_into_a_vehicle |
| **Trigger** | Drag character onto vehicle icon/model |
| **Edge cases** | Full vehicle → character moves to vehicle position only; enemy vehicle warns |
| **Acceptance** | `- [ ] Character becomes crew` |

#### CREW-UNBOARD-001 — Drag crew out

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Transforming_Crew#Removing_crew_from_a_vehicle |
| **Trigger** | Drag crew icon away from vehicle |
| **Acceptance** | `- [ ] Crew detached` |

#### CREW-SEAT-001 — Change seat (RMB)

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Transforming_Crew#Changing_Seats |
| **Trigger** | RMB crew → Change Seat → pick role |
| **Edge cases** | Occupied seat → swap |
| **Acceptance** | `- [ ] Seat reassigned` |
