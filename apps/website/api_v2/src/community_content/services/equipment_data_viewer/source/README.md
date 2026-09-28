# Equipment export source readers

Reads what a published equipment export generation holds: its manifest and pointer, its resource
list, its source documents with every native value kept as its original JSON token, and the label
each resource is shown under.

## Contents

```text
apps/website/api_v2/src/community_content/services/equipment_data_viewer/source/
├── document.rs         `Snapshot`, `Node`, `Fact`: a source document, and JSON pointer steps
├── gameplay.rs         a gameplay generation's field definitions, resources and snapshots
├── manifest.rs         `Manifest`, `PublishedPointer`, supported kinds, digests, contained paths
├── mod.rs              the module tree
└── resource_labels.rs  `resource_label`: a resource's title from its owning item's name
```

## How it works

`manifest.rs` is the integrity layer every other reader goes through. `supported` accepts two
generation kinds, `diagnostic` at schema version 2 and `gameplay` at version 1. `safe_child`
resolves a relative path inside a root and refuses an empty path, a backslash, any component that
is not a plain name (an absolute root, `.` or `..`) and a symbolic link on the way; `read` reads
a regular file of at most 128 MiB through it, `inventory` lists a folder's files and refuses
anything that is not a regular file, and `digest` is the lowercase hexadecimal SHA-256 the
manifest records.

A diagnostic generation lists its resources in `generation.json`, each with a record file and a
source file; a gameplay generation lists them in `resource_index.json`, one compact file per
resource that serves as both, with the retained fields defined once in `field_definitions.json`.
`gameplay.rs` reads either shape into the same `ResourceEntry` and `Snapshot`, so the index writer
and the queries treat both kinds alike. A `Snapshot` is a resource's native instances (`Node`),
each with its class, identity, view (`effective` or `ancestor`), children and properties; a
`Fact` keeps its value as raw JSON beside its metadata, so a number keeps the exact text the
export wrote.

## Boundaries

- Depends on: `serde_json` (with raw values), `sha2` and `hex`, and the filesystem.
- Used by: `importing/` (pointer, manifest, copy checks), `index/` (entries, snapshots, labels),
  `queries/` (documents and pointers), `service_state.rs` (dataset loading), and the equipment data
  viewer's download handler, which resolves the named document through `safe_child`.
- Rules: every file read under a data or source directory goes through `safe_child`, so no
  manifest entry or query parameter reaches outside it; a native value is never re-encoded.
