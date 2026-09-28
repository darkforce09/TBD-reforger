# Server registry

The [server control](/documentation_v2/glossary/n_to_z.md#server-control) page's server registry:
the configured game servers as read, which one the card shows, and the side sheet that registers a
server, changes its registration, and takes it out of service or puts it back. A registered server
is what the page issues [machine credentials](/documentation_v2/glossary/g_to_m.md#machine-credential)
to, sends [fleet commands](/documentation_v2/glossary/a_to_f.md#fleet-command) and deploys
[missions](/documentation_v2/glossary/g_to_m.md#mission) to.

## Contents

```text
apps/website/frontend/src/v2/pages/administration/server_control/server_registry/
├── mod.rs                   the module tree; `ServerRegistry`, the list read, the selection and the four writes
├── registration_sheet.rs    the sheet: the registration form, the refusal, the service section
├── registration_wording.rs  the name, address and port checks, the bodies, the modpack labels, the row placement
└── tests/                   unit tests for the checks, the bodies against the captured requests, the wiring
```

## How it works

The page builds one `ServerRegistry` and calls `load`, which reads `GET /api/v1/servers` once and
selects the first active server, else the first (`pick_default_id`). The picker, the card and the
sheet read its rows; the card is rebuilt only when the selection changes, and reads its own row
from the registry, so a write updates the card's header and telemetry band while its command
console, deployments panel and credential sheet keep their state. Every accepted write changes
only the row it concerns: `register` appends the answered row and selects it, `save_change` and
`reactivate` replace the row with the answered one, and `deactivate`, answered with 204 and no
body, marks the row inactive, the one field the [API](/documentation_v2/glossary/a_to_f.md#api)
changes. Each then closes the sheet and toasts; a refusal leaves the rows alone and shows the
API's sentence in the sheet.

"Add server" at the foot of the picker, or in the detail pane while no server is configured, opens
the sheet empty; "Edit" on the card opens it on that server. Each opening reads `GET
/api/v1/modpacks` for the required-modpack choice. `registration_wording` checks the form as the
API does: a name trimmed and not blank; an address that parses as a literal IPv4 or IPv6 address,
sent in its canonical form, never a hostname or an address with a `/mask` (which the database would
store and silently drop), and told apart when it carries its port; a game port from 1 to 65535.
Its messages state the API's rules. `server_change` diffs the form against the row it was seeded
from and names only the fields that differ: an address is compared as a parsed address, clearing
the required modpack sends `null`, and an untouched form sends nothing, because the API refuses a
change that names no field. While the modpacks are unread the choice cannot change, so an edit
never clears a requirement it could not show.

The sheet's service section states whether the server is active. "Deactivate" asks first: a
deactivated server's host agent and game runtime are refused, it takes no fleet command,
deployment or new credential, and members stop seeing it; nothing is deleted, and "Reactivate"
(a change with `is_active: true`) restores it with the same credentials. Every request runs in the
browser build only; a native build reads the list as failed.

## Boundaries

- Depends on: `crate::v2::core::api` (`ServerRowDto`, `ServerRegistration`, `ServerChange`,
  `ModpackDto`, `api_error_message`, and the endpoint module `server_registry`),
  `crate::v2::core::auth` (`AuthStore`) and `crate::v2::core::ui` (`Sheet`, `MaterialIcon`,
  `Toasts`); over HTTP, the `/api/v1/servers` routes of the
  [server infrastructure](/documentation_v2/glossary/n_to_z.md#server-infrastructure) domain and the
  modpack list of the community content domain.
- Used by: `page.rs` and `server_cards.rs` in
  `apps/website/frontend/src/v2/pages/administration/server_control/`, which build the registry,
  render the sheet, and open it from "Add server" and the card's "Edit"; `server_control_source` in
  `apps/website/frontend/src/v2/core/test_support/pins.rs`.
- Rules: a registration or change the API would refuse is never sent
  (`names_are_trimmed_and_required`, `addresses_are_checked_as_the_backend_checks_them`,
  `game_ports_are_bounded_as_the_backend_bounds_them`); the form sends the captured registration
  (`a_filled_form_sends_the_captured_registration`) and a change names only what differs
  (`a_change_names_only_what_differs`); an answered row takes the place of its own
  (`answered_rows_take_their_place`); the writes go through the typed endpoints
  (`writes_go_through_the_typed_endpoints`), all in `tests/server_registry.rs`.

## Related documentation

- [Server control page](/documentation_v2/website/frontend/pages/administration/server_control/server_control_page.md)
  — the registry's behaviour and what the registry routes mean server-side.
- [Give the staging server its credentials and a mission](/documentation_v2/runbooks/game_server_staging/machine_credentials_and_mission_deployment.md)
  — registering the staging server from this sheet.
