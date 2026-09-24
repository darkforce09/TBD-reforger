# Fleet command console

The "Fleet commands" section of the [server control](/documentation_v2/glossary.md#server-control)
card: an administrator requests a [fleet command](/documentation_v2/glossary.md#fleet-command) for
the selected server, follows it until an executor reports how it ended, reads the server's command
history and cancels a command no executor has claimed.

## Contents

```text
apps/website/frontend/src/v2/pages/administration/server_control/fleet_commands/
├── command_history.rs   the followed command's panel and the server's history with its cancel
├── command_requests.rs  Start, Stop, Restart, List players, the broadcast form and the kick form
├── command_wording.rs   actions, states, arguments and outcomes in words; request checks; refusals
├── mod.rs               the module tree; `CommandConsole` and its read, request, follow and cancel
└── tests/               unit tests for the announcements, the request checks and the follow loop
```

## How it works

The server card builds one `CommandConsole` per server it shows, which reads the history at once.
The console holds the history, the receipt it is following, the `busy` flag that keeps a second
click from sending a second request, the last refusal and a follow generation.

```text
request ─► POST, answered 202 with a receipt ─► toast "<Action> accepted — …"
        └─► follow: every FOLLOW_INTERVAL_MS (2 s) read the receipt
              ├─ queued, claimed, executing ─► keep following
              ├─ terminal ─► announce once (announce_receipt), read the history again, stop
              └─ read failed FOLLOW_READ_FAILURES (5) times in a row ─► "Stopped following …", stop
```

A newer request, a cancellation of the followed command and the card going away each retire the
follow through its generation. Only `succeeded` is announced as a success; `indeterminate` is
announced as an unknown outcome that nothing repeats, and a state this build does not know is
announced as unknown rather than followed forever. The request controls offer only the six actions
an operator may request, never the two a
[mission deployment](/documentation_v2/glossary.md#mission-deployment) issues, and
`validated_broadcast` and `validated_kick` check a request as the
[API](/documentation_v2/glossary.md#api) does before it is sent: a broadcast of 1 to 256 bytes
without line breaks or control characters, and a kick with an Arma identity and an optional reason
of at most 128 bytes each and a runtime session id. The kick form offers the players of the newest
successful player listing and the session that confirmed the server's newest confirmed deployment,
and fills neither on its own. Only a queued command offers "Cancel". Every request runs in the
browser build only.

## Boundaries

- Depends on: `crate::v2::core::api` (`ApiRefusal`, `api_error_message`, the receipt DTOs
  `FleetCommandReceipt`, `FleetCommandList` and `FleetCommandRequest`, and the endpoint module
  `fleet_commands`), `crate::v2::core::auth` (`AuthStore`), `crate::v2::core::ui` (`Toasts`,
  `badge_class`, `MaterialIcon`), `crate::v2::core::utils` (`utc_label`), and `executor_label` from
  `apps/website/frontend/src/v2/pages/administration/server_control/machine_credentials/`; over
  HTTP, the fleet command routes of the
  [server infrastructure](/documentation_v2/glossary.md#server-infrastructure) domain.
- Used by: `server_cards.rs` in
  `apps/website/frontend/src/v2/pages/administration/server_control/`, which builds the console
  and renders `command_requests` and `command_history`; the deployments panel in
  `apps/website/frontend/src/v2/pages/administration/server_control/mission_deployments/`, which
  words a deployment's command with `command_wording::state_label` and announces through
  `OutcomeAnnouncer`; `server_control_source` in
  `apps/website/frontend/src/v2/core/test_support/pins.rs`.
- Rules: every state earns exactly its announcement (`every_state_earns_exactly_its_announcement`)
  and `indeterminate` is said to be unknown (`an_indeterminate_outcome_is_said_to_be_unknown`); a
  request is checked as the API checks it (`requests_are_checked_as_the_backend_checks_them`); the
  console offers exactly the operator actions (`the_console_offers_exactly_the_operator_actions`);
  an acceptance is followed to its outcome (`an_acceptance_is_followed_to_its_outcome`), all in
  `tests/fleet_commands.rs`.

## Related documentation

- [Server control page](/documentation_v2/website/frontend/pages/administration/server_control/server_control_page.md)
  — the console's behaviour and what each command route means server-side.
- [Fleet command ledger evidence](/documentation_v2/website/api_v2/verification_evidence/fleet_command_ledger.md)
  — the ledger's states, claims and expiry.
