# T-068.10.5 weapon family census — variant_of verdicts (operator review table)

**Date:** 2026-07-13 · offline census cache · rule: VARIANT iff prefix-parent in family dir
(Variants/ folds up) AND magwell equal AND attachment-slot-type set equal AND base mesh equal
— i.e. the prefab differs only by pre-mounted attachments / camo materials. Fail-safe: any
unresolvable or differing evidence → KEEP. Data: [`t068_10_5_weapon_families.json`](t068_10_5_weapon_families.json).

**Totals:** 71 concrete weapons = **33 keep** (picker list) + **38 variants** (hidden, `variant_of` set). Conservation asserted.

## Engine-truth notes (review these two calls)

1. **M203-integrated M16s KEEP** (`Rifle M16A2 M203`, `carbine M203`, `suppressor M203 …`):
   Reforger bakes the M203 into the weapon prefab — own mesh AND second (40mm) muzzle/magwell.
   The GP-25 by contrast is an ATTACHMENT ITEM (host slots unchanged) → AK GP25 prefabs
   collapse, exactly per the operator's read. Engine models the two UGLs differently.
2. **`Rifle M21 ARTII OliveGreen Sand Stripes Wrapped` KEEP** — wrap changes the mesh
   (fail-safe fires). Its prefix-parent chain is itself variant; if the operator prefers this
   collapsed, it becomes a one-line rule exception at review.

## KEEP list (the collapsed pickers)

| kind | weapon |
|---|---|
| handgun | Handgun M9 |
| handgun | Handgun PM |
| launcher | Flare RSP30 green |
| launcher | Flare RSP30 red |
| launcher | Flare RSP30 white |
| launcher | FlareStarParachute M126A1 red |
| launcher | FlareStarParachute M127A1 white |
| launcher | FlareStarParachute M195 green |
| launcher | Launcher M72A3 |
| launcher | Launcher RPG22 |
| launcher | Launcher RPG7 |
| launcher | Launcher RPG75 |
| primary | MG M249 |
| primary | MG M60 |
| primary | MG PKM |
| primary | MG PKMN |
| primary | MG RPK74 |
| primary | MG RPK74N |
| primary | MG UK59 4x8 |
| primary | Rifle AK74 |
| primary | Rifle AK74N |
| primary | Rifle AKS74U |
| primary | Rifle AKS74UN |
| primary | Rifle M16A2 |
| primary | Rifle M16A2 M203 |
| primary | Rifle M16A2 carbine |
| primary | Rifle M16A2 carbine M203 |
| primary | Rifle M16A2 suppressor M203 OliveGreen Sand Stripes |
| primary | Rifle M21 |
| primary | Rifle M21 ARTII OliveGreen Sand Stripes Wrapped |
| primary | Rifle SVD |
| primary | Rifle VZ58P |
| primary | Rifle VZ58V |

## VARIANTS (child → immediate parent; UI hides all, transitive root reachable)

| variant | variant_of | evidence |
|---|---|---|
| Launcher RPG7 PGO7 | Launcher RPG7 | parent=Launcher RPG7; magwell_eq=True; slots_eq=True; mesh_eq=True |
| MG PKMN 1P29 | MG PKMN | parent=MG PKMN; magwell_eq=True; slots_eq=True; mesh_eq=True |
| MG RPK74N 1P29 | MG RPK74N | parent=MG RPK74N; magwell_eq=True; slots_eq=True; mesh_eq=True |
| Rifle AK74 GP25 | Rifle AK74 | parent=Rifle AK74; magwell_eq=True; slots_eq=True; mesh_eq=True |
| Rifle AK74N 1P29 | Rifle AK74N | parent=Rifle AK74N; magwell_eq=True; slots_eq=True; mesh_eq=True |
| Rifle AK74N GP25 | Rifle AK74N | parent=Rifle AK74N; magwell_eq=True; slots_eq=True; mesh_eq=True |
| Rifle AK74N PBS4 | Rifle AK74N | parent=Rifle AK74N; magwell_eq=True; slots_eq=True; mesh_eq=True |
| Rifle AK74N PBS4 1P29 | Rifle AK74N PBS4 | parent=Rifle AK74N PBS4; magwell_eq=True; slots_eq=True; mesh_eq=True |
| Rifle AK74N PBS4 GP25 | Rifle AK74N PBS4 | parent=Rifle AK74N PBS4; magwell_eq=True; slots_eq=True; mesh_eq=True |
| Rifle AKS74UN 1P29 | Rifle AKS74UN | parent=Rifle AKS74UN; magwell_eq=True; slots_eq=True; mesh_eq=True |
| Rifle AKS74UN PBS4 | Rifle AKS74UN | parent=Rifle AKS74UN; magwell_eq=True; slots_eq=True; mesh_eq=True |
| Rifle M16A2 4x20 | Rifle M16A2 | parent=Rifle M16A2; magwell_eq=True; slots_eq=True; mesh_eq=True |
| Rifle M16A2 OliveGreen Sand Stripes | Rifle M16A2 | parent=Rifle M16A2; magwell_eq=True; slots_eq=True; mesh_eq=True |
| Rifle M16A2 OliveGreen Solid | Rifle M16A2 | parent=Rifle M16A2; magwell_eq=True; slots_eq=True; mesh_eq=True |
| Rifle M16A2 Tutorial | Rifle M16A2 | parent=Rifle M16A2; magwell_eq=True; slots_eq=True; mesh_eq=True |
| Rifle M16A2 suppressor 4x20 OliveGreen Sand Stripes | Rifle M16A2 | parent=Rifle M16A2; magwell_eq=True; slots_eq=True; mesh_eq=True |
| Rifle M16A2 4x20 OliveGreen Sand Stripes | Rifle M16A2 4x20 | parent=Rifle M16A2 4x20; magwell_eq=True; slots_eq=True; mesh_eq=True |
| Rifle M16A2 4x20 OliveGreen Solid | Rifle M16A2 4x20 | parent=Rifle M16A2 4x20; magwell_eq=True; slots_eq=True; mesh_eq=True |
| Rifle M16A2 M203 OliveGreen Sand Stripes | Rifle M16A2 M203 | parent=Rifle M16A2 M203; magwell_eq=True; slots_eq=True; mesh_eq=True |
| Rifle M16A2 M203 OliveGreen Solid | Rifle M16A2 M203 | parent=Rifle M16A2 M203; magwell_eq=True; slots_eq=True; mesh_eq=True |
| Rifle M16A2 carbine 4x20 | Rifle M16A2 carbine | parent=Rifle M16A2 carbine; magwell_eq=True; slots_eq=True; mesh_eq=True |
| Rifle M16A2 carbine AP2k | Rifle M16A2 carbine | parent=Rifle M16A2 carbine; magwell_eq=True; slots_eq=True; mesh_eq=True |
| Rifle M16A2 carbine OliveGreen Sand Stripes | Rifle M16A2 carbine | parent=Rifle M16A2 carbine; magwell_eq=True; slots_eq=True; mesh_eq=True |
| Rifle M16A2 carbine OliveGreen Solid | Rifle M16A2 carbine | parent=Rifle M16A2 carbine; magwell_eq=True; slots_eq=True; mesh_eq=True |
| Rifle M16A2 carbine suppressor AP2K OliveGreen Sand Stripes | Rifle M16A2 carbine | parent=Rifle M16A2 carbine; magwell_eq=True; slots_eq=True; mesh_eq=True |
| Rifle M16A2 carbine suppressor OliveGreen Sand Stripes | Rifle M16A2 carbine | parent=Rifle M16A2 carbine; magwell_eq=True; slots_eq=True; mesh_eq=True |
| Rifle M16A2 carbine 4x20 OliveGreen Sand Stripes | Rifle M16A2 carbine 4x20 | parent=Rifle M16A2 carbine 4x20; magwell_eq=True; slots_eq=True; mesh_eq=True |
| Rifle M16A2 carbine 4x20 OliveGreen Solid | Rifle M16A2 carbine 4x20 | parent=Rifle M16A2 carbine 4x20; magwell_eq=True; slots_eq=True; mesh_eq=True |
| Rifle M16A2 carbine AP2k OliveGreen Sand Stripes | Rifle M16A2 carbine AP2k | parent=Rifle M16A2 carbine AP2k; magwell_eq=True; slots_eq=True; mesh_eq=True |
| Rifle M16A2 carbine AP2k OliveGreen Solid | Rifle M16A2 carbine AP2k | parent=Rifle M16A2 carbine AP2k; magwell_eq=True; slots_eq=True; mesh_eq=True |
| Rifle M16A2 carbine M203 OliveGreen Sand Stripes | Rifle M16A2 carbine M203 | parent=Rifle M16A2 carbine M203; magwell_eq=True; slots_eq=True; mesh_eq=True |
| Rifle M16A2 carbine M203 OliveGreen Solid | Rifle M16A2 carbine M203 | parent=Rifle M16A2 carbine M203; magwell_eq=True; slots_eq=True; mesh_eq=True |
| Rifle M16A2 carbine M203 Tutorial | Rifle M16A2 carbine M203 | parent=Rifle M16A2 carbine M203; magwell_eq=True; slots_eq=True; mesh_eq=True |
| Rifle M21 ARTII | Rifle M21 | parent=Rifle M21; magwell_eq=True; slots_eq=True; mesh_eq=True |
| Rifle M21 OliveGreen Sand Stripes | Rifle M21 | parent=Rifle M21; magwell_eq=True; slots_eq=True; mesh_eq=True |
| Rifle M21 ARTII OliveGreen Sand Stripes | Rifle M21 ARTII | parent=Rifle M21 ARTII; magwell_eq=True; slots_eq=True; mesh_eq=True |
| Rifle M21 OliveGreen Sand Stripes Wrapped | Rifle M21 OliveGreen Sand Stripes | parent=Rifle M21 OliveGreen Sand Stripes; magwell_eq=True; slots_eq=True; mesh_eq=True |
| Rifle SVD PSO | Rifle SVD | parent=Rifle SVD; magwell_eq=True; slots_eq=True; mesh_eq=True |

## KEEP-by-diff rows (prefix parent exists but evidence differs — fail-safe)

- **Rifle M16A2 carbine** — parent=Rifle M16A2; magwell_eq=True; slots_eq=True; mesh_eq=False; KEEP: evidence differs (fail-safe)
- **MG PKMN** — parent=MG PKM; magwell_eq=True; slots_eq=False; mesh_eq=True; KEEP: evidence differs (fail-safe)
- **Rifle M16A2 carbine M203** — parent=Rifle M16A2 carbine; magwell_eq=True; slots_eq=False; mesh_eq=True; KEEP: evidence differs (fail-safe)
- **Rifle M16A2 M203** — parent=Rifle M16A2; magwell_eq=True; slots_eq=False; mesh_eq=True; KEEP: evidence differs (fail-safe)
- **MG RPK74N** — parent=MG RPK74; magwell_eq=True; slots_eq=False; mesh_eq=True; KEEP: evidence differs (fail-safe)
- **Rifle AK74N** — parent=Rifle AK74; magwell_eq=True; slots_eq=False; mesh_eq=True; KEEP: evidence differs (fail-safe)
- **Rifle AKS74UN** — parent=Rifle AKS74U; magwell_eq=True; slots_eq=False; mesh_eq=True; KEEP: evidence differs (fail-safe)
- **Rifle M16A2 suppressor M203 OliveGreen Sand Stripes** — parent=Rifle M16A2; magwell_eq=True; slots_eq=False; mesh_eq=True; KEEP: evidence differs (fail-safe)
- **Rifle M21 ARTII OliveGreen Sand Stripes Wrapped** — parent=Rifle M21 ARTII OliveGreen Sand Stripes; magwell_eq=True; slots_eq=False; mesh_eq=True; KEEP: evidence differs (fail-safe)

V1 (mesh reliability): every weapon resolved a mesh from its ancestry chain (0 NO-MESH rows).
V2 (RPG7/PGO7): `Launcher RPG7 PGO7 → Launcher RPG7` VARIANT — all three checks equal.
