# tests

Native regression cases for the document operations that keep session state of their own: the
installed cargo defaults, the loadout buffer and its Apply seed, and the tactical-graphics draw
machine. Each file is the sibling of the production module it proves, wired by
`#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`.
