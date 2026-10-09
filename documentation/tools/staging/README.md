**Status:** live

# Staging tool crates documentation

The documents on the staging crates in `tools/staging/`: the member load's plan crate and its
generator, which drive the staging API for the `staging_load` receipt, and the
acknowledgement-dropping relay, which loses one fleet executor answer for the `staging_fleet`
receipt. Developers and AI agents read them below the crates' code READMEs, for the flows that
cross crates, the reasons and the open work.

## Contents

```text
documentation/tools/staging/
└── staging_verification_engines.md  the member load (`staging-load`) and `acknowledgement-dropping-relay`, end to end
```

## How it works

The document follows the [feature doc template](/documentation/standards/templates/feature_doc.md)
and covers both engines end to end. The code READMEs are exact about each crate and are linked,
not repeated:

| Executable | What it does | Code |
|---|---|---|
| `staging-load` | runs the member load a plan describes and prints its report, for the `staging_procedures` load procedure | [`staging_load_generator`](/tools/staging/staging_load_generator/README.md), over [`staging_load_plan`](/tools/staging/staging_load_plan/README.md) |
| `acknowledgement-dropping-relay` | on the staging host, relays one game server host agent's calls and, when armed, withholds one claim or result answer | [`acknowledgement_dropping_relay`](/tools/staging/acknowledgement_dropping_relay/README.md) |

Both executables are one-line `developer_tools` binaries
([executables README](/tools/developer_tools/src/bin/README.md)); the procedures that run them
against the staging host are [runbooks](/documentation/runbooks/staging_verification/README.md).

## Code

- [Staging tool crates](/tools/staging/) — the three crates the document covers.

## Boundaries

- Depends on: the crates' code, the xtask staging procedures that call them and the staging design
  note, which every claim is checked against; the feature doc template.
- Used by: the READMEs of `tools/staging/` and its crates, which link the document under Related
  documentation; the [tooling documentation](/documentation/tools/README.md) index.
- Rules: the document describes the committed code, and a disagreement goes under Known
  discrepancies with both places.
