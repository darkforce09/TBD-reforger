# Systems/Zones

Spatial containment, play area boundary enforcement, 2D/3D geometry math, and editor triggers.

### Roles & Responsibilities
- `TBD_ZoneRegistry.c`: Static authority registry caching prepared mission boundary and trigger zones.
- `TBD_Zone.c`: Prepared zone instance with precalculated bounding boxes and resolved enforcement rules.
- `TBD_ZoneGeometry.c`: Pure 2D XZ polygon math (ray-casting point-in-polygon tests and distance calculations).
- `TBD_ZoneVolume.c`: 3D height containment logic (AGL minimum/maximum checks and presence counts).
- `TBD_PlayAreaComponent.c` & `TBD_PlayAreaVehicleAxis.c`: Authority components monitoring player/vehicle positions, issuing warnings and penalties for boundary violations.
- `TBD_TriggerRuntime.c`: Server runtime evaluating custom editor trigger conditions and firing activated effects.

### Call Flow & Contracts
Authority-ticked containment monitoring against `TBD_ZoneRegistry`. Boundary violations transmit client warning RPCs via `SCR_PlayerController` and enforce health penalties.
