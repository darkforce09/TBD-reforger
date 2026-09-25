# Staging compose path audit

The body of `cargo xtask verify staging-compose-paths`: the entry point, the audit that returns
every verdict in print order, the comment stripper, and the parsing of the `-f` path out of the
dry-run and live compose lines. The parent file
`tools_v2/xtask/src/verifications/deployment/staging_compose_paths.rs` holds the pinned paths
and keys.

## Contents

```text
tools_v2/xtask/src/verifications/deployment/staging_compose_paths/
└── source_audit.rs  the entry point, the audit, the comment stripper and the compose line pins
```

## Boundaries

- Depends on: the parent's constants; `verification_core` (`Pattern`, `gate::ban_str`,
  `Verdict`, `NotRun`); the `regex` crate for the `-f` argument.
- Used by: the parent module, which re-exports `verify_staging_compose_paths`; its tests call
  `audit`, `strip_comments`, `f_path`, `f_regex` and `source_basename`.
- Rules:
  - `audit` returns every failure of one run, not only the first;
  - the stripper removes `//` and `#` comments outside quotes before any pin, so a comment
    naming the right path is not presence; a backslash before a closing single quote keeps it
    in quote mode for the rest of the file, a known limitation pinned by
    `a_backslash_before_a_closing_single_quote_swallows_the_rest`;
  - a missing deploy source prints `FAIL: missing <path>` with no summary line and exits 1
    (`a_missing_deploy_source_does_not_read_as_pass`).
