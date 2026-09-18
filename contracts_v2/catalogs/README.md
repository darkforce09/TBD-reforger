# Live Workbench Catalogs (`contracts_v2/catalogs/`)

Production data exported from the Enfusion Workbench and ingested by the platform. Not test data.

---

## 1. Contents

```text
catalogs/
├── README.md
├── registry-items.workbench.json       <-- Item catalog: every spawnable arsenal item
└── registry-compat.workbench.json      <-- Weapon-to-attachment compatibility edges
```

---

## 2. Why These Are Not Fixtures

These two files are the arsenal. `cargo xtask db seed` imports them into Postgres, the API serves them as the Virtual Arsenal catalog and the compatibility matrix, and the Scenario Creator's loadout editor is limited to what they contain. A test fixture is data chosen to exercise a code path; these are the live content of the game.

The distinction is operational, not cosmetic. If an ingest ever fell back to a sample file, the arsenal would fill with sample data and every test would still pass, because the tests assert against the samples. Keeping the live export in its own directory makes that substitution impossible to make by accident.

Their sampled counterparts — `fixtures/registry/registry-items.sample.json` and `registry-compat.sample.json` — stay in `fixtures/` and are what the ingest normalisation tests run against.

---

## 3. Production Path

```text
Enfusion Workbench
  └─ TBD_RegistryItemsExportPlugin          writes both files to the profile directory
       └─ committed here, replacing the previous export wholesale
            └─ cargo xtask db seed  →  Postgres arsenal tables
                 └─ GET /api/v1/missions/registry  →  Scenario Creator arsenal
```

Never hand-edited: an entry that does not correspond to a real Enfusion prefab is a mission that fails to spawn its loadout. Correcting the catalog means correcting the game content and re-exporting.
