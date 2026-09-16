# operations

Headless editor queries and mutations over an explicit `MissionDocCore`.

Entity modules cover identity, selection, ORBAT, vehicles, comments, connections, zones, and clipboard operations. Attribute, cargo, composition, reassignment, tactical graphic, and transform modules preserve the existing field-level transactions. Placement and rotation modules contain deterministic geometry.

The frontend retains signals, active-layer preference, palette and drag state, dialogs, loadout buffers, ID counters, and post-edit refreshes. It supplies plain values or callbacks for confirmation, layer choice, and cargo defaults. Callbacks run at the same points in the operation as before extraction; no UI type enters this crate.
