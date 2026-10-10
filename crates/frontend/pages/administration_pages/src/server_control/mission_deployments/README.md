# Mission deployments panel

The "Mission deployments" section of the
[server control](/documentation/glossary/n_to_z.md#server-control) card: an administrator requests a
[mission deployment](/documentation/glossary/g_to_m.md#mission-deployment) of an approved
[artifact](/documentation/glossary/a_to_f.md#artifact) to the selected server, optionally bound to an
[event](/documentation/glossary/a_to_f.md#event) mission's seats, follows it until a runtime session
confirms it, reads the server's deployments and cancels one whose command no executor has claimed.

## Contents

```text
crates/frontend/pages/administration_pages/src/server_control/mission_deployments/
├── deployment_list.rs     the followed deployment, the list newest first and an opened row's detail
├── deployment_refusal.rs  `DeploymentRefusal`: each refusal code read with its details and worded
├── deployment_request.rs  the request form, its pickers and the choices it reads when it opens
├── deployment_wording.rs  states, transitions and detail rows in words; the choices a request offers
├── mod.rs                 the module tree; `DeploymentPanel` and its read, request, follow and cancel
└── tests/                 unit tests for the detail, the announcements, the choices and the refusals
```

## How it works

The server card builds one `DeploymentPanel` per server it shows, which reads the deployment list
at once. The request form reads its choices only when it first opens: the mission library, of
which it offers the live missions whose latest approval names an artifact, and the upcoming
events, of which it keeps those scheduled on this server and reads each one's hub for its event
missions that run the chosen mission. Choosing no event mission deploys without binding seats.

```text
request ─► POST, answered 202 ─► toast "Deployment of <mission> recorded — …"
        └─► follow: every 2 s (FOLLOW_INTERVAL_MS) read the deployment
              ├─ requested ─► keep following
              ├─ confirmed, failed or cancelled ─► announce once, read the list again, stop
              └─ read failed 5 times in a row (FOLLOW_READ_FAILURES) ─► "Stopped following …", stop
```

A 202 records a deployment and nothing more: only `confirmed`, reported when a runtime session
loads the artifact, is announced as success. A newer follow or the card going away retires the
current one. `DeploymentRefusal` reads every refusal code the [API](/documentation/glossary/a_to_f.md#api)
names, with its details, and words what to change before asking again, listing each unbound seat
and unseated [slot](/documentation/glossary/n_to_z.md#slot) of an
[ORBAT](/documentation/glossary/n_to_z.md#orbat) mismatch; a code this build does not know falls back to
the API's sentence. Only a deployment in flight offers "Cancel this deployment".
`latest_confirmed_session` hands the [fleet command](/documentation/glossary/a_to_f.md#fleet-command)
console's kick form the runtime session that confirmed the newest confirmed deployment. Every
request runs in the browser build only.

## Boundaries

- Depends on: `frontend_transport` (`MissionDeployment`, `MissionDeploymentPage`,
  `DeploymentRequest`, `Paginated`, `MissionCard`, `EventListItem`, `EventHub`, `Error`,
  `Error::message_or`, and the endpoint module `mission_deployments`), `frontend_session`
  (`AuthStore`), `frontend_ui` (`Toasts`, `MaterialIcon`, `badge_class`),
  `frontend_ui` (`utc_label`); `command_wording` from
  `crates/frontend/pages/administration_pages/src/server_control/fleet_commands/` (command
  states and `OutcomeAnnouncer`); `short_digest` from
  `crates/frontend/features/mission_review_record/src/`; over HTTP, the deployment
  routes of the [missions](/documentation/glossary/g_to_m.md#missions) domain and the event reads of
  the [operations](/documentation/glossary/n_to_z.md#operations) domain.
- Used by: `server_cards.rs` in `crates/frontend/pages/administration_pages/src/server_control/`,
  which builds the panel, renders `deployment_request` and `deployment_list`, and passes
  `latest_confirmed_session` to the kick form.
- Rules: only approved live missions are offered (`only_approved_live_missions_are_offered`) and a
  request names the approved artifact (`a_request_names_the_approved_artifact`); event missions come
  from events on this server (`event_missions_come_from_operations_on_this_server`); a deployment is
  announced only once it has ended (`a_deployment_is_announced_only_once_it_has_ended`), all in
  `tests/mission_deployments.rs`; every refusal says what to change; requests are followed and
  choices read on demand.

## Related documentation

- [Server control page](/documentation/crates/frontend/pages/administration_pages/server_control/server_control_page.md)
  — the panel's behaviour and what the deployment routes mean server-side.
- [Mission artifacts design note](/documentation/crates/api/api_server/design_notes/mission_artifacts.md)
  — the design of artifacts, their reviews and deployments.
