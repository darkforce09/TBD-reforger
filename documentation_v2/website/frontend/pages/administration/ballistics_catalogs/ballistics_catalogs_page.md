**Status:** live

# Ballistics catalogs page

The `/admin/ballistics-catalogs` page, titled "Ballistics Catalogs", one of the
[administration](/documentation_v2/glossary/a_to_f.md#administration) pages: administrators upload a
ballistics catalog version together with the calibration bundle it is validated against, read the
validation report, and see every stored version. A stored version never changes; the page only
adds versions, and every catalog the mortar calculator solves with, vanilla included, arrives
through it.

## Where it lives

- Code: [`apps/website/frontend/src/v2/pages/administration/ballistics_catalogs/`](/apps/website/frontend/src/v2/pages/administration/ballistics_catalogs/):
  `page.rs` holds the route component `BallisticsCatalogsPage` and the two-column layout;
  `upload_form.rs` the two file pickers and the multipart upload; `validation_report.rs` the
  outcome and the failed cases; `version_list.rs` the stored versions; `view_model.rs` the upload
  readiness, the outcome classes and the version rows. The folder's
  [README](/apps/website/frontend/src/v2/pages/administration/ballistics_catalogs/README.md)
  describes each file.
- Entry: the route, its tier and its layout are in the README's
  [Routes](/apps/website/frontend/src/v2/pages/administration/ballistics_catalogs/README.md#routes).
- Related: the [mortar calculator page](/documentation_v2/website/frontend/pages/field_tools/mortar/mortar_calculator_page.md),
  which reads the published catalogs; the
  [game ballistics engine](/documentation_v2/website/map-engine/data/scenario/ballistics/game_ballistics_engine.md),
  whose calibration judges an upload.

## Behaviour

The body renders inside `AdminGate`. The README's
[How it works](/apps/website/frontend/src/v2/pages/administration/ballistics_catalogs/README.md#how-it-works)
quotes every headline and state text.

### Uploading a version

1. Under "Upload a catalog version" the administrator picks the "Catalog document" and the
   "Calibration bundle", both `.json` files, typically the pair
   `cargo xtask ballistics trim-export` writes under `contracts_v2/`. Picking a file clears the
   previous outcome, so the report always describes the files on the form.
2. "Validate and store" stays disabled until both parts are picked and both are `.json`; it sends
   them as one multipart request with the parts `catalog` and `calibration`, and reads
   "Validating…" while the API runs the calibration.
3. The outcome shows below the form: "Accepted — all N calibration cases passed; the version is
   stored.", or "Refused — F of N calibration cases failed; nothing was stored." followed by every
   failed case and what differs, or "Already stored — " for a version or bytes already on file, or
   "Upload refused (status) — " with the API's sentence for a malformed, oversized or wrongly typed
   upload. A session that expired after the refresh asks to sign in again; an upload that never
   reached the API asks to check the connection.
4. The failure table is never truncated: each row names a case the catalog must be fixed for.

### Reading the stored versions

1. "Stored versions" lists every version, catalogs ordered by identifier and each catalog's
   versions newest first, the newest marked latest, with the short SHA-256 (the full value in the
   tooltip) and the upload time in UTC.
2. An accepted upload refreshes the list, as does "Refresh". A failed fetch shows its reason and
   "Retry", never an empty list; with nothing stored the list says to upload the first version.

## Data

The README's [Data](/apps/website/frontend/src/v2/pages/administration/ballistics_catalogs/README.md#data)
lists each call with its DTO. Server-side:

- `POST /api/v1/ballistics-catalogs` (`apps/website/api_v2/src/operations/handlers/ballistics_catalogs/upload.rs`):
  administrator only, checked before the body is read. Refusals in the order they are checked: a
  body that is not `multipart/form-data` 415; a body over 17 MiB + 64 KiB, a `catalog` part over
  1 MiB or a `calibration` part over 16 MiB 413 (`request_too_large`); a part of another content
  type 415; a missing or repeated part 400; a part that does not decode 400; a version or bytes
  already stored 409 (`catalog_version_exists`, `catalog_bytes_exist`); any calibration failure
  422 with the whole report under `details`. An accepted pair answers 201 with the report
  (`accepted`, `cases`, `failures`, `forward_samples_not_judged`); the version and its audit line
  (`ballistics_catalog.uploaded`) are stored in one transaction, so a refused upload stores
  nothing. A database trigger refuses every later update or delete of a stored version.
- `GET /api/v1/ballistics-catalogs` (`apps/website/api_v2/src/operations/handlers/ballistics_catalogs/reads.rs`):
  public; one summary per stored version.

## Design

- Padded inside the navigation frame under the header: the upload form and, below it, the
  validation report on the left; the stored versions on the right.
- No design set exists for this page; the built layout is the reference.

## Open work

None.

## Decisions

- Catalogs arrive only through this upload, never at boot: every version, vanilla included, is
  judged by the same calibration before any page can solve with it.
- An upload must carry its calibration bundle: the API runs the model against the game's tables
  and the engine oracle and refuses anything over 1 mil or 0.1 s, so a catalog that does not
  reproduce the game is never stored.
- Versions are immutable: saved fire missions pin a catalog id and version, and a correction is a
  new version rather than an edit.
- The report lists every failed case: the administrator fixes the catalog, not a summary.
