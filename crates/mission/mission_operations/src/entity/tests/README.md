# tests

Native regression cases for the entity authoring operations: the arm gate, the zone and trigger
draw machine, layer authoring over a live document, the two armed pointer-drags, trigger edits,
the selection projection, and what a canvas release commits for each kind of armed placement.
Each file is the sibling of the production module it proves, wired by
`#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`.
