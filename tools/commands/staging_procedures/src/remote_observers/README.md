# Staging observers

Every read the harness makes of the staging host, and the ssh transport that runs it. Each
observer builds a read-only `RemoteCommand` (a shell command line with every value single-quoted,
and optional stdin) and parses its answer.

## Contents

```text
tools/commands/staging_procedures/src/remote_observers/
├── console_log_reader.rs     an instance's newest `console.log` (path line, then the tail)
├── database_reader.rs        committed SELECTs in a read-only `psql` session, `:'name'` bindings
├── discord_member_reader.rs  the bot's member read and its `discord-member-read {json}` line
├── host_shell.rs             the live transport: one ssh per command, the password in `SSHPASS`
├── metrics_reader.rs         `/metrics` with the bearer read on the host and piped to curl
├── mod.rs                    the module tree
├── remote_command.rs         `RemoteCommand`, `CommandPurpose`, `HostCommandRunner`, shell quoting
├── unit_journal_reader.rs    a unit's `journalctl --user` lines since a Unix time
└── unit_state_reader.rs      `systemctl --user show`: state, PID, start, memory, CPU
```

## How it works

The database reader runs `docker exec -i -e 'PGOPTIONS=-c default_transaction_read_only=on'
<container> psql -X -A -t -q -v ON_ERROR_STOP=1 -U tbd -d tbd_reforger -v name=value …` with the
statement on stdin. The statement is a committed constant that starts with `SELECT` or `WITH` and
holds no `;`; each value is bound with `-v` and used as `:'name'`, so no value is ever part of the
statement text. The metrics reader's script reads `OBSERVABILITY_TOKEN` from the API env file on
the host and pipes `Authorization: Bearer …` to `curl -H @-`; the token is in no argument.

`HostShell` builds `ssh_argv` from `process_runner::secure_shell_transport` with the command line as
the only remote argument and spawns it with a 120 s timeout for a read and 3600 s for a change.

## Boundaries

- Depends on: `process_runner::secure_shell_transport`; `process_runner`; on the host, bash,
  docker, `systemctl`, `journalctl`, curl and the `staging-fixtures` tool.
- Used by: the procedure runner's probes, `support_commands/`, `environment_identity/`.
- Rules: every observer builds a read; no command line or script carries a secret; psql reads run
  behind the read-only guard.
