# Fleet scenario sheet

The side sheet that keeps the [fleet scenario](/documentation/glossary/a_to_f.md#fleet-scenario)
registry: for each terrain, the [mission header](/documentation/glossary/g_to_m.md#mission-header) the
fleet boots when a [mission deployment](/documentation/glossary/g_to_m.md#mission-deployment) restarts a
server on that terrain. It belongs to the whole fleet, so the server picker's heading opens it, not
a server's card.

## Contents

```text
crates/frontend/pages/administration_pages/src/server_control/fleet_scenarios/
├── mod.rs               the module tree; `ScenarioRegistry`, its read and its register and remove
├── scenario_sheet.rs    the sheet: the registration form, a row per terrain, the remove confirmation
├── scenario_wording.rs  the registration checks and the "Updated by" line
└── tests/               unit tests for the checks against the captured registry and the endpoints
```

## How it works

The [server control](/documentation/glossary/n_to_z.md#server-control) page builds one
`ScenarioRegistry` and renders `scenario_sheet` over the screen. `open_sheet` opens it and reads the
registry whole, and `put` and `remove` read it again after every change the
[API](/documentation/glossary/a_to_f.md#api) accepts, since a terrain with no registered scenario
refuses every deployment. The registry holds the sheet's open flag, the registry as read, the `busy`
flag and the last refusal.

`scenario_registration` checks a registration exactly as the API and the contract do before it is
sent: a terrain key of lowercase letters, digits and underscores that starts with a letter, at
most 64 bytes; a scenario id of sixteen uppercase hex digits in braces, then a path of letters,
digits, `_`, `.`, `/` or `-` ending in `.conf`; a trimmed display name of 1 to 128 bytes. The form
keeps its fields until the API stores the mapping, and "Edit" on a row loads that row into it.
Removing a terrain asks first and only stops offering it to new deployments. `updated_line` names
who last changed a mapping, "you" for the viewer. Every request runs in the browser build only.

## Boundaries

- Depends on: `frontend_transport` (`FleetScenario`, `FleetScenarioList`,
  `FleetScenarioUpdate`, `Error::message_or`, and the endpoint module `fleet_scenarios`),
  `frontend_session` (`AuthStore`), `frontend_ui` (`Sheet`, `MaterialIcon`,
  `Toasts`) and `frontend_ui` (`utc_label`); over HTTP, the `/api/v1/fleet/scenarios`
  routes of the [server infrastructure](/documentation/glossary/n_to_z.md#server-infrastructure) domain.
- Used by: `page.rs` in `crates/frontend/pages/administration_pages/src/server_control/`,
  which builds the registry, renders the sheet and opens it from the picker's "Fleet scenarios"
  button.
- Rules: a registration the API would refuse is never sent
  (`terrain_keys_are_checked_as_the_backend_checks_them`,
  `scenario_ids_follow_the_contract_pattern`, `display_names_are_trimmed_and_bounded`), every
  captured mapping passes the checks (`every_captured_mapping_passes_the_checks`), all in
  `tests/fleet_scenarios.rs`; changes go through the typed endpoints.

## Related documentation

- [Server control page](/documentation/crates/frontend/pages/administration_pages/server_control/server_control_page.md)
  — the sheet's behaviour and what the registry routes mean server-side.
