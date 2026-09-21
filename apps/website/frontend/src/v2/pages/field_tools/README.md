# Hub 4: Field Tools (`src/v2/pages/field_tools`)

Standalone web-based tactical aids — utility pages that stand on their own rather
than hanging off a mission or an operation.

## Pages
1. **`mortar/` (`/tools/mortar`)**: the mortar calculator. The **ballistics are solved on the
   server** (`POST /fire-missions/solve`, or `POST /fire-missions` when the solution is also to be
   saved against an operation); this folder sends the geometry and renders what comes back.

*(Note: Engine-dependent 3D debug inspectors such as the building viewer and world line-of-sight testbeds are standalone applications and live under `src/v2/apps/debug/` rather than web pages).*
