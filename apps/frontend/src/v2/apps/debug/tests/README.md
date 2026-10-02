# Testbench Tests (`v2/apps/debug/tests`)

Sibling test files for the diagnostics benches, mounted from their production module with
`#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`.

**Depended on by:** nothing. Test files are mounted, never imported.

**Boundary:** these run on the native target, so they cover the benches' pure geometry — the
interior plan lanes, the section cuts, the probe ray. What needs a canvas belongs in the headless
gate harness instead.
