# Fleet command console

The "Fleet commands" section of the [server control](/documentation/glossary/n_to_z.md#server-control)
card: an administrator requests a [fleet command](/documentation/glossary/a_to_f.md#fleet-command) for
the selected server, sends one line to its [RCON](/documentation/glossary/n_to_z.md#rcon) console,
follows each command until an executor reports how it ended, reads the server's command history and
cancels a command no executor has claimed.

## Contents

```text
apps/website/frontend/src/v2/pages/administration/server_control/fleet_commands/
├── command_history.rs       the followed command's panel and the server's history with its cancel
├── command_requests.rs      Start, Stop, Restart, List players, the broadcast form and the kick form
├── command_wording.rs       actions, states, outcomes and replies in words; request checks; refusals
├── console_command_form.rs  the console box: one line, its byte count, its check and "Send"
├── mod.rs                   the module tree; `CommandConsole`: read, request, follow and cancel
└── tests/                   unit tests for the announcements, the request checks and the follow loop
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
announced as unknown rather than followed forever. The request controls offer only the seven actions
an operator may request, never the two a
[mission deployment](/documentation/glossary/g_to_m.md#mission-deployment) issues, and
`validated_broadcast`, `validated_kick` and `validated_console_line` check a request as the
[API](/documentation/glossary/a_to_f.md#api) does before it is sent: a broadcast of 1 to 256 bytes
without line breaks or control characters; a kick with an Arma identity and an optional reason
of at most 128 bytes each and a runtime session id; and a console line with no control character
and no line or paragraph separator in what was typed, 1 to 256 bytes once trimmed, and no leading
`@`. The kick form offers the players of the newest successful player listing and the session that
confirmed the server's newest confirmed deployment, and fills neither on its own.

The console box sits under the process-control buttons, since the
[fleet host agent](/documentation/glossary/a_to_f.md#fleet-host-agent) carries out a
`console_command` over RCON as it does the player list. "Send" and Enter in the field both submit
it; nothing is sent while another request is in flight, and a sent line leaves the field, because
the host agent transmits a line once and nothing repeats it. A succeeded console command's reply
(`ConsoleCommandOutcome`: `response` and `response_truncated`) is summarised by
`console_reply_summary` in the outcome line and the toast, and shown whole, as text with its line
breaks, under the "Your command" panel and under its history row. A line the server never answered
fails with the host agent's reason, "no RCON response; the command may or may not have run".

Only a queued command offers "Cancel". Every request runs in the browser build only.

## Boundaries

- Depends on: `crate::v2::core::api` (`ApiRefusal`, `api_error_message`, the receipt DTOs
  `FleetCommandReceipt`, `FleetCommandList`, `ConsoleCommandOutcome` and `FleetCommandRequest`, and
  the endpoint module `fleet_commands`), `crate::v2::core::auth` (`AuthStore`), `crate::v2::core::ui`
  (`Toasts`, `badge_class`, `MaterialIcon`), `crate::v2::core::utils` (`utc_label`), and
  `executor_label` from
  `apps/website/frontend/src/v2/pages/administration/server_control/machine_credentials/`; over
  HTTP, the fleet command routes of the
  [server infrastructure](/documentation/glossary/n_to_z.md#server-infrastructure) domain.
- Used by: `server_cards.rs` in
  `apps/website/frontend/src/v2/pages/administration/server_control/`, which builds the console
  and renders `command_requests` and `command_history`; the deployments panel in
  `apps/website/frontend/src/v2/pages/administration/server_control/mission_deployments/`, which
  words a deployment's command with `command_wording::state_label` and announces through
  `OutcomeAnnouncer`; `server_control_source` in
  `apps/website/frontend/src/v2/core/test_support/pins.rs`.
- Rules: every state earns exactly its announcement (`every_state_earns_exactly_its_announcement`)
  and `indeterminate` is said to be unknown (`an_indeterminate_outcome_is_said_to_be_unknown`); a
  request is checked as the API checks it (`requests_are_checked_as_the_backend_checks_them`,
  `a_console_line_is_checked_as_the_backend_checks_it`); a console reply reads as the server sent
  it (`a_console_reply_reads_as_the_server_sent_it`) and is shown under the panel and the history
  (`a_console_reply_is_shown_under_the_panel_and_the_history_row`); the console offers exactly the
  seven operator actions and sends only a checked console line
  (`the_console_offers_exactly_the_operator_actions`); an acceptance is followed to its outcome
  (`an_acceptance_is_followed_to_its_outcome`), all in `tests/fleet_commands.rs`.

## Related documentation

- [Server control page](/documentation/website/frontend/pages/administration/server_control/server_control_page.md)
  — the console's behaviour and what each command route means server-side.
- [Fleet command ledger evidence](/documentation/website/api_v2/verification_evidence/fleet_command_ledger.md)
  — the ledger's states, claims and expiry.
