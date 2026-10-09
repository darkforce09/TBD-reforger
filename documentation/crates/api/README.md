**Status:** live

# API crate documentation

The documentation of the [API](/documentation/glossary/a_to_f.md#api) crates under `crates/api/`,
kept under the API server, the top crate that assembles them: the cross-domain overview, the
environment variable reference, the design decisions and the verification evidence. Developers
and AI agents read it below the crates' code READMEs, before changing the API or deploying it.

## Contents

```text
documentation/crates/api/
└── api_server/  the API: overview, environment variables, decisions and verification evidence
```

## Code

- [API crates](/crates/api/README.md) — the library crates, one per kernel concern or domain,
  and their category rules.
- [API server](/crates/api/api_server/README.md) — the router, the composition root and the
  `api-server` and `import-item-registry` binaries.

## Boundaries

- Depends on: the code of `crates/api/`, which every claim is checked against.
- Used by: the API crates' code READMEs; the
  [library crate documentation](/documentation/crates/README.md) index.
- Rules: the documents describe the committed code; the per-domain detail stays in each crate's
  own README.
