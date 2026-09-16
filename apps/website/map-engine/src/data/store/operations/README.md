# operations

Headless editor queries and mutations over an explicit `MissionDocCore`.

Entity modules cover identity, selection, ORBAT, vehicles, comments, connections, zones, and clipboard operations. Attribute, cargo, composition, reassignment, tactical graphic, and transform modules preserve the existing field-level transactions. Placement and rotation modules contain deterministic geometry.

Session state that outlives a single call but is never persisted lives here, in thread-locals no browser API can reach: the installed cargo defaults, the loadout buffer and the seed each Apply draws from, and the tactical-graphics draw machine — the selection, the in-flight draw, and the in-flight vertex drag.

The frontend retains signals, the active-layer preference, dialogs, ID counters, and post-edit refreshes. It supplies plain values or callbacks for confirmation, layer choice, and the crew toggle. Each callback runs at the point in the operation where its decision is needed; no UI type enters this crate.
