# Deploy settings loader

The parts of the one reader of `tools/xtask/deploy/deploy.env`: the file's grammar, the deploy
host it names, and the remote folders that default under that host's user.
`tools/xtask/src/core/deploy_environment.rs` declares the three files, holds the loader and the
precedence rule, and re-exports `DeployHost`, `first_ipv4_address` and `DeployHostFolder`.

## Contents

```text
tools/xtask/src/core/deploy_environment/
├── assignment_syntax.rs    the `KEY=VALUE` grammar: comments, `export `, quotes, errors by line
├── deploy_host.rs          `DeployHost` (`user@host` or `host`) and the first IPv4 address of a name
├── deploy_host_folders.rs  `DeployHostFolder`: the checkout, profile, addon and server folders
└── tests/                  unit tests for the grammar, the host and the folder defaults
```

## How it works

- `assignment_syntax`: blank lines and `#` lines are skipped and `export ` is dropped; the key
  matches `[A-Za-z_][A-Za-z0-9_]*`, with spaces allowed around `=`. A value in `"…"` or `'…'` is
  taken verbatim, with no escapes and no `$` expansion, and only spaces or ` # comment` may follow
  it; an unquoted value ends at the first `#` after whitespace, so `KEY= # note` is empty and
  `PASS=a#b` keeps its `#`. A line without `=`, a bad key, an unterminated quote or text after a
  closing quote is an error that names its line and never echoes the value. A repeated key keeps
  its last assignment.
- `deploy_host`: `DeployHost::parse` refuses whitespace, an empty user or host, a second `@` and a
  leading `-`, so ssh never reads the value as an option. `home_directory` is `/home/<user>` and
  `tbd_folder` `/home/<user>/tbd`; a host without a user has neither. `first_ipv4_address` returns
  an IPv4 literal as it is and otherwise takes the first IPv4 answer of the system resolver, which
  may list IPv6 answers first; a name with only IPv6 answers is an error.
- `deploy_host_folders`: `TBD_REMOTE_DIR`, `TBD_PROFILE_DIR`, `TBD_ADDONS_STAGING` and
  `TBD_SERVER_DIR` resolve to their value when set, else to `/home/<user>/tbd/repo`,
  `/home/<user>/tbd/profile`, `/home/<user>/tbd/addons-staging` and
  `/home/<user>/steam/arma-reforger-server`; with no user in `TBD_SSH_HOST` an unset folder is a
  missing setting.

## Boundaries

- Depends on: the loader in `tools/xtask/src/core/deploy_environment.rs`; the standard
  library's resolver for `first_ipv4_address`.
- Used by: the loader, and through its re-exports the `deploy`, `setup`, `debug` and `mod` command
  groups.
- Rules: the grammar's cases, its errors and their line numbers are pinned in
  `tests/assignment_syntax/tests.rs`; the IPv4 pick after IPv6 answers
  (`the_first_ipv4_address_is_picked_after_ipv6_ones`) and the refusals in
  `tests/deploy_host/tests.rs`; an explicit folder beats its default and a host without a user
  leaves the folders required (`tests/deploy_host_folders/tests.rs`). No test needs the network:
  the resolution tests use literals, `::1` and `localhost` from `/etc/hosts`.
