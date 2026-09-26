**Status:** live

# Draft persistence

The decisions behind the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s local
drafts of a [mission](/documentation_v2/glossary/g_to_m.md#mission): which key a draft lives under,
whether a stored blob is worth keeping, how a record on disk merges with the one about to replace
it, whether the local draft and the server's version differ, how a server version replaces the
document, and the snapshot pair that makes a replacement reversible. The map engine makes these
decisions; the Mission Creator stores the bytes in IndexedDB and fetches the server's version.

## Where it lives

- Code: [`apps/website/map-engine/src/editing/persist/`](/apps/website/map-engine/src/editing/persist/README.md),
  part of the [editing layer](/documentation_v2/website/map-engine/editing_layer.md), over the
  document in [`data/store/`](/apps/website/map-engine/src/data/store/README.md).
- Entry: the Mission Creator's shell. The boot's server reconciliation calls `is_uuid`,
  `classify_local_draft`, `adopt_payload` and the snapshot capture
  ([`shell/hydrate/`](/apps/website/frontend/src/v2/apps/editor/shell/hydrate/README.md));
  the draft writer calls the key scoping, `restores_to_authored_content`, `merge_before_write`
  and the read retry ([`shell/persist/`](/apps/website/frontend/src/v2/apps/editor/shell/persist/README.md));
  the review restore adopts a reviewed version
  (`apps/website/frontend/src/v2/apps/editor/mission_editor/canvas_mount/review_restore.rs`).
- Related features: the [editing layer](/documentation_v2/website/map-engine/editing_layer.md),
  whose post-change tail schedules each draft save, and the Mission Creator's
  [feature inventory](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md)
  for the save, restore and conflict features.

## Behaviour

### Opening a mission

1. The Mission Creator restores any local draft of the mission from IndexedDB, then reconciles
   with the server.
2. An id that is not a UUID (`is_uuid`) names no server row: the document stays local-only.
3. The shell fetches the mission. A mission with no saved version adopts an empty document on a
   first open, or stamps only the row fields (`apply_row_meta_only`) over a local draft.
4. A saved version with no local draft is adopted with `Adopt::Init`: no undo step and no
   snapshot, since nothing is lost.
5. A saved version with a local draft is classified by `classify_local_draft`, cheapest test
   first:
   - `Empty`: the draft holds no authored content; the server version is adopted as in step 4;
   - `MatchesServer`: the draft has the same [slot](/documentation_v2/glossary/n_to_z.md#slot) count and
     compiles to the same `editor`, `loadouts`, `objectives`, `vehicles` and `markers` as the
     server version (hydrated into a throwaway document); nothing is asked, and the mission is
     marked clean;
   - `Diverged`: the Mission Creator shows its conflict dialog with both slot counts and save
     times.
6. "Keep local copy" keeps the draft and marks it unsaved. "Load server version" drops the stale
   pre-restore snapshot, captures the whole draft as the pre-adopt snapshot, then adopts the server
   version with `Adopt::Undoable`, one undo step, and says so: "Loaded the server version. Your
   local copy (N objects) was backed up — press Ctrl/Cmd+Z to put it back."

Terrain, environment and row fields never raise the prompt: only the authored keys compare.

### Adopting a payload

1. `adopt_payload` hydrates the payload into the live document, filing unlayered slots under
   `DEFAULT_LAYER_ID` (`layer-1`), writes the mission row (`RowMeta`, preferring the payload's
   non-blank title), then runs the host's post-change tail once.
2. `Adopt::Init` writes under the init origin, so it is not an undo step. `Adopt::Undoable`
   requires a blank row, so the adopt stays one undo step.
3. An undoable adopt is partial: roots outside the undo scope keep the adopted rows after an undo.
   The snapshot pair exists for that reason.
4. The hydrate clears the authored maps but not the connection map, so the replaced document's
   connections remain after an adopt.

### The snapshot pair

1. `capture_document_snapshot` encodes the whole document before anything replaces it, and
   refuses an empty encode.
2. The pre-adopt slot holds the draft a server adopt displaced; the pre-restore slot holds the
   document a restore displaced. A restore writes the displaced document into the counterpart
   slot, so the pair swings both ways, and reading a slot never consumes it.
3. The Mission Creator keeps both in memory and in IndexedDB under the signed-in account, and
   exposes them as `window.__missionBackup` (`has`, `restore`, `hasUndoRestore`, `undoRestore`)
   for the mission open in the editor. A successful save expires both.

### Saving a draft

1. The post-change tail schedules a debounced draft write, keyed by the bare mission id; a
   snapshot's key adds `::pre-adopt` or `::pre-restore`, which the draft write never reaches.
2. Every key is scoped to its owner: `scoped_key(owner, logical)` prefixes `u<len>:<owner>|`, so
   no owner string can collide two accounts, and a signed-out author writes under `anon`.
3. A blob is kept only when it replays to authored content (`restores_to_authored_content`): a
   stream with no client blocks, or one that fails to replay, is not a backup. The per-edit writer
   probes the live document instead, in the same synchronous window as the encode.
4. Before the write, `merge_before_write` reads the stored record late and merges it into the
   outgoing bytes by CRDT union under the init origin, never as an undo step, and never for another
   mission's record. A failed read is retried after 80, 160 and 320 ms.

### Known discrepancies

- An adopt is meant to replace the document, but it keeps the replaced document's connections:
  the hydrate clears every authored map but `connections`
  (`apps/website/map-engine/src/data/store/rows/hydrate.rs:30-48`). The persist README states it
  as behaviour; T-1050 files it as a bug.

## Data

- IndexedDB: the Mission Creator's draft records and snapshot records, keyed as above; the stored
  value is the document's `encode_state` blob. The storage code is in
  [`shell/persist/`](/apps/website/frontend/src/v2/apps/editor/shell/persist/README.md).
- `GET /api/v1/missions/{id}`: the mission row and its current version, whose `json_payload` is
  the payload `classify_local_draft` and `adopt_payload` read; the call and its fallbacks are in
  the [hydrate README](/apps/website/frontend/src/v2/apps/editor/shell/hydrate/README.md).
- `slots_digest`: an order-independent fingerprint that compares a cold and a warm document by
  content rather than by encoded bytes.

## Design

- The decisions are in the map engine and the transport is in the Mission Creator: the engine
  names no storage or network API, so every decision has a native test
  (`apps/website/map-engine/src/editing/persist/tests/`), and the
  [engine boundary rules](/documentation_v2/standards/engine_boundary_rules.md#where-state-lives)
  place serialise and hydrate decisions on the engine side.
- The prompt appears only on a real difference in authored content: a save followed by a reopen
  compares equal and shows no unsaved mark.

## Open work

- [T-1050 — Fix mission re-hydrate keeping stale connections](/.ai/tickets/T-1050.toml) (idea, no
  plan): the hydrate clears the connection map, so an adopt leaves no connection of the replaced
  document behind.
- [T-1051 — Check whether minted vehicle and object ids can collide](/.ai/tickets/T-1051.toml)
  (idea, no plan): after a reopen, the host's id counter restarts at 0 while the adopted document
  holds ids; a new vehicle or object either cannot overwrite one, or the mint checks every id map.

## Decisions

- Classify by what the documents contain, not by bytes or timestamps: two encodings of the same
  mission differ in bytes, and a draft the adopt would reproduce never prompts.
- Ask on every real difference: "Keep local copy" is one click and "Load server version" is
  reversible twice over (undo and the snapshot pair), while a document silently replaced by one it
  never derived from is neither.
- A boot adopt is not an undo step: the first undo would otherwise restore an empty document.
- Merge before every write, by CRDT union: a record written by another tab or an earlier write is
  folded in, never overwritten, and a record from another mission is never applied.
- Scope every key to its owner: one account's drafts are never read as another's on a shared
  browser.
