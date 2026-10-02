# Ballistics catalogs page

The `/admin/ballistics-catalogs` page: administrators upload a ballistics catalog version — the
weapon and shell values the mortar calculator solves with — together with the calibration bundle
it is validated against, read the validation report the upload answers with, and see every stored
catalog version. A stored version never changes; the page only adds versions.

## Contents

```text
apps/frontend/src/v2/pages/administration/ballistics_catalogs/
├── mod.rs                the module tree; re-exports `BallisticsCatalogsPage`
├── page.rs               `BallisticsCatalogsPage`: the gate and the two-column layout
├── tests/                unit tests for the view model and the route, breadcrumb and sidebar registration
├── upload_form.rs        the two file pickers, the send control and the multipart upload
├── validation_report.rs  the upload outcome's headline and the table of failed calibration cases
├── version_list.rs       the stored versions: fetch, loading, failed and empty states, refresh
└── view_model.rs         upload readiness, outcome classification and headlines, version rows
```

## How it works

`BallisticsCatalogsPage` renders `BallisticsCatalogsInner` inside `AdminGate`. The left column
holds the upload form and, below it, the validation report; the right column holds the stored
versions.

The form keeps an `UploadDesk`: the picked catalog and calibration bundle (their names and sizes
for display, and the browser file handles), an in-flight flag and the latest outcome. Picking a
file clears the outcome, so a report always describes the files on the form. `upload_blocker`
names why the upload cannot be sent — a part is missing, or a part is not a `.json` file — and the
send control stays disabled until it returns nothing. "Validate and store" sends both files as one
multipart `POST` with the parts `catalog` and `calibration`, through
`api_post_form_keeping_refusal`, which keeps the refusal's `details`.

`upload_outcome` classifies the answer:

| Answer | Outcome | Tone | Report |
|---|---|---|---|
| `201` with `accepted` and no failures | `Accepted` | success | "Accepted — all N calibration cases passed; the version is stored." |
| `422` whose `details` carries a `failures` array | `CalibrationRefused` | failure | "Refused — F of N calibration cases failed; nothing was stored.", then every failed case and what differs |
| `409` | `Duplicate` | warning | "Already stored — " and the backend's sentence |
| `400`, `413`, `415`, a `422` without failures, any other status | `Rejected` | failure | "Upload refused (status) — " and the backend's sentence |
| `401` after the refresh-and-retry | `SessionExpired` | warning | sign in again |
| never reached the backend, or unreadable | `Unreachable` | warning | check the connection |

The report panel carries the outcome's name as `data-upload-outcome` (`accepted`,
`calibration-refused`, `duplicate`, `rejected`, `session-expired`, `unreachable`). The failure
table is never truncated: each row names a case the catalog has to be fixed for.

The version list fetches the list on mount and whenever its reload counter changes: an accepted
upload and the Refresh control both bump it. `version_rows` orders the catalogs by identifier and
each catalog's versions newest first, marks the newest version of each catalog "latest", shortens
the SHA-256 to 12 characters (the full value is the cell's tooltip) and labels the upload time in
UTC. A failed fetch shows its reason and a Retry control, never an empty list. Every request runs
in the browser build only; a native build renders the failed state.

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/admin/ballistics-catalogs` | `BallisticsCatalogsPage` | route tier `admin`; the body renders inside `AdminGate`, for the `admin` [role](/documentation/glossary/n_to_z.md#role) only | padded inside the navigation frame; breadcrumb Administration / Ballistics Catalogs; sidebar entry "Ballistics Catalogs" |

## Data

- `GET /api/v1/ballistics-catalogs`: `BallisticsCatalogList` of `BallisticsCatalogSummary`, from
  `core::api::dto::ballistics_catalogs`.
- `POST /api/v1/ballistics-catalogs` (administrator, multipart parts `catalog` and
  `calibration`): `CatalogUploadReport` on `201`; on `422` the same report under the error's
  `details`, read by `report_from_details`, which ignores extra keys such as a `code`.

## Public surface

- `BallisticsCatalogsPage`: the route component `apps/frontend/src/app_routes.rs` mounts.

## Boundaries

- Depends on: `crate::v2::core` (the API client with `api_get` and
  `api_post_form_keeping_refusal`, `ApiRefusal`, the ballistics catalog DTOs, `AuthStore`,
  `AdminGate`, `PageHeader`, `MaterialIcon`, `cn`, `utc_label`); over HTTP, the operations domain
  of the API.
- Used by: the route table in `apps/frontend/src/app_routes.rs` and `router.rs`; the
  Administration section of `apps/frontend/src/v2/pages/navigation/nav_config.rs`.

## Tests

`tests/ballistics_catalogs.rs` (module `v2::pages::administration::ballistics_catalogs::tests`)
covers the route and part names, upload readiness, file size labels, every outcome class with its
headline, tone and key, the tolerant read of a `422` report, the version row order, latest marks,
short SHA and count label, and the route's tier, breadcrumb, sidebar entry and mount.

## Related documentation

- [Ballistics catalogs page documentation](/documentation/apps/frontend/pages/administration/ballistics_catalogs/README.md) —
  the page's feature documentation.
