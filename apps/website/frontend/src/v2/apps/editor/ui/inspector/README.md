# Inspectors (`v2/apps/editor/ui/inspector`)

The surfaces that read one subject out of the document and write it back: a slot's transform and
identity, a zone's shape and rules, the placed vehicles, the mission-wide environment bag
(weather, radio nets, tasks, audio, spawn waves, win conditions), and the validation drawer that
reports what the compiler found.

**Depended on by:** the docks and the settings dialog, which mount these sections.

**Boundary:** a commit is one undo step and reaches the document through the map engine's hosted
commands, never by mutating a row in place. Every field re-reads the document on each version
bump, so an undo taken with a panel open refreshes the fields rather than stranding them.
