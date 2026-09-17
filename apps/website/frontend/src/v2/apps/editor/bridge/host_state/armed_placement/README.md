# Armed Placement (`v2/apps/editor/bridge/host_state/armed_placement`)

The in-flight placement: what the operator picked up from a palette and has not yet dropped, the
zone draw that shares the same arm, and the map release that commits it.

**Depended on by:** the right dock's palette, which arms it, and the canvas pointer closures,
which release it.

**Boundary:** the discriminant lives on the armed value, never on a reading of which palette tab
is open — the tab can change between pick-up and commit. Nothing here is undoable, and a release
that commits nothing leaves no trace.
