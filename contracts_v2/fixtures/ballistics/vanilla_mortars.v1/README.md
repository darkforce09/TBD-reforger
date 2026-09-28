# Vanilla mortar calibration bundle

The calibration cases the ballistics flight model must reproduce before version 1 of the
`vanilla_mortars` catalog is accepted: the game's own ballistic and wind tables for every charge of
its 7 shells, and the engine oracle's answers for the same shells. The catalog is
`contracts_v2/catalogs/ballistics/vanilla_mortars.v1.catalog.json`. The folder is written whole by
`cargo xtask ballistics trim-export --generation 6A6F008DC5395616` and is never edited by hand.

## Contents

```text
contracts_v2/fixtures/ballistics/vanilla_mortars.v1/
├── calibration.json  31 native tables (476 rows), 31 wind tables, 21111 oracle samples
└── negative/         four bundles the evaluator must refuse, one defect each
```

## How it works

The trim reads the gameplay equipment export of generation 6A6F008DC5395616 (plain path
assets_v2/equipment/gameplay/generations/6A6F008DC5395616/export, gitignored) and the ballistics oracle's
output for it (assets_v2/scratch/ballistics_oracle/6A6F008DC5395616, gitignored). It verifies every export
file against the export manifest's SHA-256 and each oracle file against its `_meta.json` sidecar, then
keeps only what the catalog's charges need: the native table at each charge coefficient, the wind
tables at charge coefficients, and the oracle's forward-angle, simulation and altitude-difference
samples at those coefficients. Every engine number is written as the shortest decimal of the 32-bit
float the engine holds.

A game table row stores range, an uninterpreted second column and time of flight, but no elevation.
The oracle samples the engine's forward lookup on a 1.5625-mil elevation lattice, which holds every
row's elevation, so each row's `elevation_mils_6400` is fixed by a forward sample whose range and time
of flight equal the row's within 0.01 m and 0.001 s, or by a lattice end, which the engine answers
with the time-of-flight sentinel −1 (the first row is the vertical shot at range 0, the last sits at
the last lattice elevation and its range equals the range the engine reports there). Every sample
between two adjacent rows equals their linear interpolation. A row neither rule fixes refuses the
trim, so every native row of every kept table is in the bundle.

## Provenance

- Game build: 1.8.0.13.
- Export generation: 6A6F008DC5395616.
- Catalog SHA-256 (the bundle's `catalog_sha256`): `24a68cc5e22b5d3dc62ec80d82d0e004916e1d300ac0a964d1660847e8f9fbde`.
- Calibration bundle SHA-256: `12be201bbdd22f1bfa72b3aead176cc53969abeca532e33ee8e7216b2fc2d3d0`.
- Oracle run: plugin revision tbd-ballistics-oracle/2, first run started 2026-09-28T11:45:17Z; `output_sha256` `3842052cdce9ef3c0f217318ef786f00d2d37b3dd93fbfde3af30d1c3af862bd` is the SHA-256 of the
  `sha256sum` listing of the two output files below, in name order.
  - forward_angles.json: `cadf9f9b9fc801e61c16e3b89b5e03c4a311d7e04cec46a461e1752f46c333e4`, equal to its sidecar.
  - simulation.json: `591d3147890c4a14523344b04a125f60fe7785054e9b730ac5432a752bf6d16d`, equal to its sidecar.
- Gravity: PhysicsWorld.GetGravity reported 9.8100004196167 m/s²; the catalog's `gravity_m_s2` and the bundle's
  `gravity_reported_m_s2` store 9.81 m/s², the shortest decimal of that 32-bit float.

| Document | GUID | Resource | SHA-256 of the exported file |
|---|---|---|---|
| catalog | 38BAE094333E31BF | {38BAE094333E31BF}Prefabs/Weapons/Ammo/Ammo_Shell_81mm_HE_M821.et | `71502c291fe872030140d428f757fc547658f439ba3f9f8c32eaed13cfd11c18` |
| catalog | 6113990D163E5249 | {6113990D163E5249}Prefabs/Items/Equipment/BallisticTable/BallisticTable_US.et | `3b3541a3ae68bbbbf44c22918c9e1bc482de405d31c2ddfdea667eea5eb2fd58` |
| catalog | 8094D99689ABE241 | {8094D99689ABE241}Prefabs/Weapons/Mortars/M252/Mortar_M252.et | `5c0d59d73121803fc35563062cf37d87efebe13cfe3717986ee65922b9b819f2` |
| catalog | 98EC9C526AFBA282 | {98EC9C526AFBA282}Prefabs/Weapons/Ammo/Ammo_Shell_82mm_HE_O832DU.et | `54eb1e81f8cccf0abef14abeed3abdc025a6c7c8bf35233e4afa750c49ce84dc` |
| catalog | A544A2C131DE2C64 | {A544A2C131DE2C64}Prefabs/Weapons/Ammo/Ammo_Shell_82mm_Smoke_D832DU.et | `ca20f04101dd0639199b2fddf0338568e2844ea187cd58e307274655c34050cd` |
| catalog | B41607EAB58A4252 | {B41607EAB58A4252}Prefabs/Items/Equipment/BallisticTable/BallisticTable_USSR.et | `520239878b230b2087aa9e91d08f58c36ce8d896a964a04c1069753e5e0d2025` |
| catalog | C8A906FB198D1A33 | {C8A906FB198D1A33}Prefabs/Weapons/Ammo/Ammo_Shell_82mm_Illum_S832S.et | `1fd96514bee7f9764cab9fb048c93f4ad1a28e7049f08f5873ec60eb23b6d13e` |
| catalog | D1FFE458E8AC4BDB | {D1FFE458E8AC4BDB}Prefabs/Weapons/Mortars/2B14/Mortar_2B14.et | `65abc244ba65e37d20d394b2839f112d80502a0889389bdb3c74760f06a5cb6f` |
| catalog | DD2065AE34D8DFA9 | {DD2065AE34D8DFA9}Prefabs/Weapons/Ammo/Ammo_Shell_81mm_Illum_M853A1.et | `08a3f86ae918e8f5fdcc74dd08120e6853c93f41ad554ca4a520fc3800d4dbf6` |
| catalog | DD6844AB03FDA84F | {DD6844AB03FDA84F}Prefabs/Weapons/Ammo/Ammo_Shell_81mm_Practice_M879.et | `2a5047f10db93bb5a1e69ad7c365d329cce25fc82419bc092499c590cc0519cd` |
| catalog | F7807293E94D3C88 | {F7807293E94D3C88}Prefabs/Weapons/Ammo/Ammo_Shell_81mm_Smoke_M819.et | `c04a41f9248b03b663ee196fa060751d01bbb81791422371dd1d5f926c316ada` |
| calibration | 1776217937D7D5AB | {1776217937D7D5AB}Configs/Weapons/Ammo/M853A1_Shell_81mm_Illumination.conf | `d38900952ba81992eb81557bc34a2e052091e61293eccbbdc5f40786734274ca` |
| calibration | 1FDD66CD968BA3F9 | {1FDD66CD968BA3F9}Configs/Weapons/Ammo/WindData/WindData_Shell_82mm_HE_O832DU.conf | `b832c1f4325e5b51dcc78aaccc7b50aa05846897e814da27f9bfe436d1c21545` |
| calibration | 2CFC2755F68357FC | {2CFC2755F68357FC}Configs/Weapons/Ammo/M819_Shell_81mm_Smoke.conf | `0ba72983ff89d50a89312c98fa0c4dcc6ec99d3880c3336ca8d58bf8ed455d1f` |
| calibration | 3B8528B7CD63EE4D | {3B8528B7CD63EE4D}Configs/Weapons/Ammo/O832DU_Shell_82mm_HE.conf | `5fe35492684ca445c563caeb51523e3c9f44b6811c938cfaa41d0cc021335b64` |
| calibration | 57EE2625B53A5763 | {57EE2625B53A5763}Configs/Weapons/Ammo/WindData/WindData_Shell_81mm_Illum_M853A1.conf | `01b97ce10d841380eab9b5b02bd027e2d528888a9d323e77ccd1daac53b8be9d` |
| calibration | 5BAA973A894BDF0F | {5BAA973A894BDF0F}Configs/Weapons/Ammo/WindData/WindData_Shell_81mm_Practice_M879.conf | `59c552f9b233c27b6e84a51127ae6f946848913202f87dd0fe8c6d6b49be0267` |
| calibration | 61369B7723DC63DA | {61369B7723DC63DA}Configs/Weapons/Ammo/D832DU_Shell_82mm_Smoke.conf | `679c246fa8040ab5a48d7dc7c4326f5334d1f349dab0d44fcac8d5953417e525` |
| calibration | 86E1596833C1569C | {86E1596833C1569C}Configs/Weapons/Ammo/S832S_Shell_82mm_Illumination.conf | `1cca95e90548e9f7803dc1e006dbf0b50d62d3bb5468849336aae6067210fb0e` |
| calibration | 8EEEFC36AD250257 | {8EEEFC36AD250257}Configs/Weapons/Ammo/WindData/WindData_Shell_82mm_Smoke_D832DU.conf | `c5a3fb5af719690aa60c992dba4f990bf7f83230a71a538161cbec1423fe4a87` |
| calibration | 97AAA5A969EF4CF9 | {97AAA5A969EF4CF9}Configs/Weapons/Ammo/WindData/WindData_Shell_81mm_HE_M821.conf | `af01d5341c1336a5fb4950da27e4abad893be45d48b5339d6d403591cc4cc960` |
| calibration | 9924A554000C4555 | {9924A554000C4555}Configs/Weapons/Ammo/M821_Shell_81mm_HE.conf | `6ebd11536b9a1de626412f30f75c7cb6931459a686cd475951c8bf032b5fdf2d` |
| calibration | B0EC54D545BC2C72 | {B0EC54D545BC2C72}Configs/Weapons/Ammo/M879_Shell_81mm_Practice.conf | `cb4cae68a7594c97be3e87dac600c16d8b4fe48e28331da357544356958c0941` |
| calibration | F42488BA4AEA74F6 | {F42488BA4AEA74F6}Configs/Weapons/Ammo/WindData/WindData_Shell_81mm_Smoke_M819.conf | `8b36b7bc4921d1f6675994881546be97be65d3ca9659b5f7c1ef8b71df7b912d` |
| calibration | F74EFC9B5E7EA9ED | {F74EFC9B5E7EA9ED}Configs/Weapons/Ammo/WindData/WindData_Shell_82mm_Illum_S832S.conf | `373617d11407aef40e1aa1cb289dec8d3d878cc9d02eae615fd6537f0858c28f` |

## Native row elevations

- Fixed by equality with one forward sample: 414.
- Fixed at a lattice end: 62.
- Every native row is matched; none is interpolated or left out.

## Format

- Encoding: UTF-8 JSON indented by two spaces, one table row, wind row, resource or oracle sample per
  line, with a final newline.
- Schema: `contracts_v2/definitions/ballistics-calibration.schema.json`; the catalog it pins follows
  `contracts_v2/definitions/ballistics-catalog.schema.json`.
- Oracle samples carry the charge's `rings` first in `inputs`, then the engine call's arguments under
  the oracle's names; `outputs` are the oracle's results unchanged.
- Changing a file: rerun the trim after a new export or oracle run, then `cargo xtask schema validate`.

## Producers and consumers

- Producer: `cargo xtask ballistics trim-export` (`tools_v2/xtask/src/commands/ballistics/`).
- Consumers: `cargo xtask schema validate`, whose ballistics section checks both documents and their
  provenance and coverage; the map engine's calibration tests; the upload of the catalog pair through
  `POST /api/v1/ballistics-catalogs`.
