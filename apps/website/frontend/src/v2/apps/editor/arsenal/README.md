# Arsenal Domain (`v2/apps/editor/arsenal`)

The loadout editor's decisions and serialization: the loadout rows and the compatibility graph
behind them, the asset catalog the pickers read, the 3D paper doll preview, and the commit path
that persists a pick on the slot as one undo step.

**Depended on by:** the attributes modal's Arsenal tab, and `ui/arsenal/`, which draws the panels
this module mounts.

**Boundary:** the pure half — rows, compatibility, option building, validation, doll regions,
weight — stays decidable without a browser and is tested natively. A pick reaches the document
through the map engine's hosted commands, never by writing a row in place.
