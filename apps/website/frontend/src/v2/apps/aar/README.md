# After-Action Report (`v2/apps/aar`)

An empty scaffold. The post-match forensics workspace will live here: replaying a completed match
from server telemetry — player positions, vehicle routes, engagements and objective captures — on
a playback clock, with the map read-only underneath it.

**Depended on by:** nothing yet. `v2/apps/mod.rs` declares only `debug` and `editor`, so no code
in this directory is compiled, and `app_routes.rs` carries no replay route.

**Boundary, and the reason it is written down before there is code:** a replay authors nothing. It
needs the map engine's camera, world streaming, symbology and picking with no mission document
open, no undo stack and no armed placement — so `website-map-engine`'s `editing/` must not acquire
editor-only assumptions that would make its tools unusable without an authoring session.
