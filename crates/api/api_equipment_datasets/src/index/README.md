# Equipment dataset navigation index

Builds the SQLite index one imported generation is browsed through: its resources, native
instances, fields, field occurrences, capabilities and references. The index holds positions and
identities only; every native value stays in the generation's source documents, and the queries
read it from there.

## Contents

```text
crates/api/api_equipment_datasets/src/index/
├── mod.rs              the module tree and `INDEX_VERSION`, the index layout version
├── resource_writer.rs  one resource's nodes, field occurrences, edges, capabilities and references
├── schema.sql          the index tables, from resources and nodes to references and the summary
└── writer.rs           `build`: checks the generation, fills the index, stores its overview
```

## How it works

`writer::build` accepts only a complete, error-free generation of a supported kind whose resource
inventory is exactly the manifest's file list. A gameplay generation also needs a selection report
of policy version 1 with no unreviewed field, and its record, node and fact counts must equal what
the index holds. The writer applies `schema.sql` to a new `catalog.sqlite` in staging, writes each
resource through `resource_writer`, then adds the secondary indexes and the per-field counts
(effective, ancestor and per-resource). It refuses the index when the occurrence count differs
from the facts it read, when a gameplay reference points at no indexed resource, when a foreign
key dangles, or when SQLite's integrity check fails. The overview it stores in `summary` (resource,
node, fact and field counts, equipment, vehicles and dependencies, bytes and capabilities) is what
the dataset status route answers.

## Boundaries

- Depends on: the source readers in `source/` (documents, gameplay definitions, resource labels),
  the service's progress reporting, and `sqlx` with SQLite.
- Used by: `importing/generation_import.rs`, which builds the index in staging and seals it.
- Rules: an index is rebuilt from the source documents, never edited in place; its layout version
  is `INDEX_VERSION`, part of the path the service loads it from, so a layout change
  raises that version.
