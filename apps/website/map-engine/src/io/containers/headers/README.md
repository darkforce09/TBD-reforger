# Container header facade

One flat import point for the four binary containers' headers, magics and versions, the shared
header trait and `peek_version`. It declares nothing of its own and hosts the containers' unit
tests.

## Contents

```text
apps/website/map-engine/src/io/containers/headers/
├── mod.rs  the module tree; re-exports the header trait, the four headers, magics and versions
└── tests/  unit tests: sizes, wire layouts, round trips, level spans and every refusal path
```

## Boundaries

- Depends on: `crate::io::containers::header`, `tbdc`, `tbde`, `tbdb` and `tbds`, whose items it
  re-exports; the tests also use `crate::io::pod::instance` and
  `crate::io::archives::codec::BinaryError`.
- Used by: nothing outside the folder; every reader and writer imports the defining module
  (`crate::io::containers::tbdc` and the rest).
- Rules: a re-export only; the tests pin the container contract: every header is 32 bytes and its
  fields fill it, magic is checked before version, a short, misnamed, wrongly versioned or
  wrongly sized buffer is an error rather than a panic, an unaligned buffer reads through
  `ContainerHeader::read`, a count or level past the address space or shift width is refused, and
  a `TBDS` version 1 file fails the version 2 header but still answers `peek_version`
  (`tests/cases_1.rs`).
