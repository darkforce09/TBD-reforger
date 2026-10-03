# Ballistics agreement bench tests

Sibling test files for the ballistics agreement bench, mounted from their production module with
`#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`.

## Contents

```text
apps/frontend/src/workspaces/debug/ballistics_agreement/tests/
├── agreement_report.rs  the case-to-inputs mapping, the solved reading and the f64 bit patterns
└── bench_query.rs       the URL parameters, the choice of catalog version and the state attribute
```

## Boundaries

- Depends on: the committed catalog
  `contracts/catalogs/ballistics/vanilla_mortars.v1.catalog.json`; an unreadable catalog fails
  the tests.
- Used by: nothing. Test files are mounted, never imported.
- Rules: these run on the native target; the browser half is covered by the gate
  `gate ballistics-agreement`.
