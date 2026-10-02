# Session Tests (`v2/apps/editor/shell/tests`)

Sibling test files for the browser session, mounted from their production module with
`#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`.

**Depended on by:** nothing. Test files are mounted, never imported.

**Boundary:** these run on the native target, so they exercise the decidable half of the session —
the save policy, the writer election, the size arithmetic, the exported grid reference. A test
needing a browser belongs in the headless gate harness instead.
