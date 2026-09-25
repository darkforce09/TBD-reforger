**Status:** live

# Eden transformation interactions

How the Arma 3 Eden editor moves, lifts, rotates and snaps entities: the basic drag gestures in
the view, and the transformation widget with its variants and reference axes. Each entry follows
the Eden reference format of the [feature entry schema](/documentation_v2/website/frontend/apps/editor/feature_inventory/feds_schema.md#eden-reference-entries).

## XFORM — Basic transform

#### XFORM-MOVE-001 — LMB drag move

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Entity_Transforming#Position |
| **Shortcut** | LMB drag |
| **Acceptance** | `- [ ] Drag moves entity` |

#### XFORM-ALT-001 — Alt drag altitude

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Entity_Transforming#Altitude |
| **Shortcut** | Alt + drag |
| **Eden parity** | N/A (3D) for the [Mission Creator](/documentation_v2/glossary.md#mission-creator), whose map view is top-down |

#### XFORM-SHIFT-001 — Shift drag rotate

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Entity_Transforming#Direction |
| **Shortcut** | Shift + drag |
| **Acceptance** | `- [ ] Rotates to cursor` |

#### XFORM-VERT-001 — Vertical mode

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Entity_Transforming#Vertical_Mode |
| **UI Surface** | Toolbar |
| **Acceptance** | `- [ ] Sea level / underground` |

#### XFORM-SNAP-001 — Surface snap

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Entity_Transforming#Surface_Snapping |
| **UI Surface** | Toolbar |
| **Acceptance** | `- [ ] Snaps to terraces` |

## WIDGET — Transformation widget

#### WIDGET-CYCLE-001 — Cycle widget (Space)

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Transformation_Widget#Variants |
| **Shortcut** | Space |
| **Acceptance** | `- [ ] Cycles translation/rotation/area widgets` |

#### WIDGET-TRANS-001 — Translation widget

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Transformation_Widget#Translation_Widget |
| **Acceptance** | `- [ ] Axis arrows move entity` |

#### WIDGET-ROT-001 — Rotation widget

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Transformation_Widget#Rotation_Widget |
| **Acceptance** | `- [ ] Rotates on axis` |

#### WIDGET-AREA-SCALE-001 — Area scaling widget

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Transformation_Widget#Area_Scaling_Widget |
| **Acceptance** | `- [ ] Resizes trigger area` |

#### WIDGET-AREA-001 — Area widget

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Transformation_Widget#Area_Widget |
| **Acceptance** | `- [ ] Orients area marker` |

#### WIDGET-COORD-001 — Global vs local axes

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Transformation_Widget#Changing_the_Reference_System |
| **Acceptance** | `- [ ] Local rotates with entity` |
