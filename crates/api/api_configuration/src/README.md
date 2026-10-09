# API configuration source

The source of `api_configuration`: the configuration module, the process lifecycle module, and
the crate root that exports them with the crate's error.

## Contents

```text
crates/api/api_configuration/src/
├── configuration/      `Config`, `ConfigError` and `ProxyNet`: the settings read at boot and their checks
├── error.rs            `Error`, which a `ConfigError` converts into, and `Result`
├── lib.rs              the crate root: module header, `mod` lines and the re-export of `Error`
├── prelude.rs          `Config`, `ConfigError`, `ShutdownSignal` and `process_shutdown` for glob import
└── process_lifecycle/  `ShutdownSignal` and `process_shutdown`, the flag the server raises to stop
```

## How it works

Each folder is a public module with its own README. The two modules do not import each other.

## Boundaries

- Depends on: `api_identifiers`, `dotenvy`, `repository_root`, `thiserror` and `tokio::sync`.
- Used by: the crate root and, through it, the API's database crate and application.
- Rules: no module names a domain or an API crate above this one.
