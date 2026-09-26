# Machine credential sheet

The side sheet the selected server's "Credentials" button opens: every
[machine credential](/documentation_v2/glossary/g_to_m.md#machine-credential) of that server without its
secret, the form that issues one and shows its secret once, and the revocation that asks for a
reason.

## Contents

```text
apps/website/frontend/src/v2/pages/administration/server_control/machine_credentials/
├── credential_sheet.rs  the sheet: the one-time secret, the issue form, the list and the revoke form
├── credential_text.rs   program names, the issue and standing lines, the label and reason checks
├── mod.rs               the module tree; `CredentialPanel`, its requests; re-exports `executor_label`
└── tests/               unit tests for the list wording, its order and the label and reason bounds
```

## How it works

The server card builds one `CredentialPanel` for the server it shows, so one server's credentials
or freshly issued secret never show under another's name. `open_sheet` opens the sheet and reads
the list; an issue or a revocation the [API](/documentation_v2/glossary/a_to_f.md#api) accepts reads it
again. The list puts live credentials first and the newest first within each, and states for each
its program, its label, who issued it and when, and either its last use or who revoked it, when
and why ("you" for the viewer).

The secret of an issued credential lives in one signal and nowhere else: the sheet shows it once
under a warning, with a copy control that goes through `crate::v2::core::utils::clipboard` and
reports a copy only once the browser confirms it, and the signal is cleared when the administrator
says it is stored or the sheet closes. It is never written to storage or a log. `credential_text.rs`
checks a request as the API bounds it before it is sent: a trimmed label of 1 to 128 bytes, and a
trimmed revocation reason of 1 to 512 bytes, which the audit trail keeps. The issue form offers the
two program kinds, `mod_runtime` (the [game runtime](/documentation_v2/glossary/g_to_m.md#game-runtime))
and `host_agent` (the [fleet host agent](/documentation_v2/glossary/a_to_f.md#fleet-host-agent)); a kind
this build does not know shows as the API spells it. Every request runs in the browser build only.

## Boundaries

- Depends on: `crate::v2::core::api` (`MachineCredential`, `MachineCredentialList`,
  `MachineCredentialIssue`, `IssuedMachineCredential`, `ExecutorKind`, `api_error_message`, and the
  endpoint module `machine_credentials`), `crate::v2::core::auth` (`AuthStore`),
  `crate::v2::core::ui` (`Sheet`, `MaterialIcon`, the toast queue) and `crate::v2::core::utils`
  (`clipboard`, `utc_timestamp`); over HTTP, the credential routes of the
  [server infrastructure](/documentation_v2/glossary/n_to_z.md#server-infrastructure) domain.
- Used by: `server_cards.rs` in
  `apps/website/frontend/src/v2/pages/administration/server_control/`, which builds the panel, opens
  the sheet and renders it; `executor_label`, which the
  [fleet command](/documentation_v2/glossary/a_to_f.md#fleet-command) console in
  `apps/website/frontend/src/v2/pages/administration/server_control/fleet_commands/` names executors
  with; `server_control_source` in `apps/website/frontend/src/v2/core/test_support/pins.rs`.
- Rules: the secret is shown once and never stored; labels and reasons are bounded as the API
  bounds them (`labels_and_reasons_are_bounded_as_the_backend_bounds_them`), the issue form offers
  exactly the API's program kinds (`the_issue_form_offers_the_backend_program_kinds`), and live
  credentials list first and newest first (`live_credentials_list_first_and_newest_first`), all in
  `tests/machine_credentials.rs`.

## Related documentation

- [Server control page](/documentation_v2/website/frontend/pages/administration/server_control/server_control_page.md)
  — the sheet's behaviour and what the credential routes mean server-side.
- [Machine credentials evidence](/documentation_v2/website/api_v2/verification_evidence/machine_credentials.md)
  — how the API issues, verifies and revokes credentials.
