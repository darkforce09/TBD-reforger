# Deploy settings source

The loader of `deploy/deploy.env` and its parts: the file's grammar, the deploy host it names, the
remote folders that default under that host's user, and the errors a load or a setting reports.

## Contents

```text
tools/foundation/deploy_settings/src/
├── assignment_syntax.rs    the `KEY=VALUE` grammar: comments, `export `, quotes, errors by line
├── deploy_environment.rs   `DeployEnvironment`: loads the file under one precedence rule; the setting keys; the ssh transport
├── deploy_host.rs          `DeployHost` (`user@host` or `host`) and the first IPv4 address of a name
├── deploy_host_folders.rs  `DeployHostFolder`: the checkout, profile, addon and server folders
├── error.rs                `Error` and `Result` (the file could not be loaded), `SettingError` and `SettingOrigin`
├── lib.rs                  the crate root: module header, `mod` lines and the re-exports
├── prelude.rs              the loader, the path, the host, the folders and `SettingError` for glob import
└── tests/                  unit tests for the loader, the grammar, the host and the folder defaults
```

## How it works

- `deploy_environment`: `deploy_environment_path` takes the file from `DEPLOY_ENV` when that is set
  (made absolute against the working directory), else `deploy/deploy.env` under the checkout.
  `load_required` refuses a missing file, `load_if_present` allows one, and both refuse an
  unreadable file and a line that breaks the grammar with `<path>:<line>`. A repeated key keeps its
  last assignment; the file decides every key it assigns, an empty assignment counting as unset,
  and the process environment fills only the keys the file never assigns.
- `assignment_syntax`: blank lines and `#` lines are skipped and `export ` is dropped; the key
  matches `[A-Za-z_][A-Za-z0-9_]*`, with spaces allowed around `=`. A value in `"…"` or `'…'` is
  taken verbatim, with no escapes and no `$` expansion, and only spaces or ` # comment` may follow
  it; an unquoted value ends at the first `#` after whitespace, so `KEY= # note` is empty and
  `PASS=a#b` keeps its `#`. A line without `=`, a bad key, an unterminated quote or text after a
  closing quote is an error that names its line and never echoes the value.
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
- `error`: the texts are the operator's instructions (`Missing <path> — copy from
  deploy/deploy.env.example`, `<path>:<line>: <KEY>: <problem>`, `<KEY> is not set: add it to
  <path>`), fixed per variant.

## Boundaries

- Depends on: `repository_layout` (`DEPLOY_ENV`, `DEPLOY_ENV_EXAMPLE`), `process_runner`
  (`SshBase`), `thiserror`, and the standard library's resolver for `first_ipv4_address`.
- Used by: the crate root's re-exports, read by the `xtask` command groups that reach a host.
- Rules: the grammar's cases, its errors and their line numbers are pinned in
  `tests/assignment_syntax_tests.rs`; the IPv4 pick after IPv6 answers
  (`the_first_ipv4_address_is_picked_after_ipv6_ones`) and the refusals in
  `tests/deploy_host_tests.rs`; an explicit folder beats its default and a host without a user
  leaves the folders required (`tests/deploy_host_folders_tests.rs`); the precedence rule, the
  load errors and the committed example in `tests/deploy_environment_tests.rs`. No test needs the
  network: the resolution tests use literals, `::1` and `localhost` from `/etc/hosts`.
