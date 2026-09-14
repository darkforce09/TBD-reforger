# Hub 4: Field Tools (`src/v2/pages/field_tools`)

Standalone tactical aids and engineering debug inspectors — pages that stand on their own rather
than hanging off a mission or an operation.

## Pages
1. **`mortar/` (`/tools/mortar`)**: the mortar calculator. The **ballistics are solved on the
   server** (`POST /fire-missions/solve`, or `POST /fire-missions` when the solution is also to be
   saved against an operation); this folder sends the geometry and renders what comes back.

## Still to arrive
The two engine-dependent debug inspectors — the building viewer (`/debug/building-viewer`) and the
world line-of-sight testbed (`/debug/world-los`) — mount the render engine and the occluder, so
they move here once the map engine does. Until then they live in the legacy tree.
