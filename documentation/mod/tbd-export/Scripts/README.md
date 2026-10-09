**Status:** live

# Export addon script documentation

The deeper documents of the export addon's scripts, mirroring `mod/tbd-export/Scripts/`. Only
the [Workbench](/documentation/glossary/n_to_z.md#workbench) module has documents here; the game
module's runtime road export is covered by the map export feature doc.

## Contents

```text
documentation/mod/tbd-export/Scripts/
└── WorkbenchGame/  the map export, the equipment and vehicle exporter and the ballistics oracle documents
```

## Code

- [Export addon scripts](/mod/tbd-export/Scripts/) — the `Game/` module, which runs in a
  playing export world, and the `WorkbenchGame/` module, which runs in the editor.

## Boundaries

- Depends on: the code READMEs under `mod/tbd-export/Scripts/`.
- Used by: the [export addon index](/documentation/mod/tbd-export/README.md).
- Rules: the folder mirrors the code folder names, so a document's place follows its code.
