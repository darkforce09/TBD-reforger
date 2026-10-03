# Ballistics command tests

Unit tests of `cargo xtask ballistics trim-export`, run over a synthetic gameplay export and oracle
output that each test writes into its own folder under the system temporary directory, so they
need neither the gitignored export nor a Workbench run.

## Contents

```text
tools/commands/ballistics_oracle_tooling/src/tests/
└── trim_export/  the trim tests and the synthetic export they run over
```

## How it works

`trim_export/synthetic_export.rs` writes one mortar, one range card and one shell with two
charges, their table and wind configurations with manifest hashes, and the oracle's forward-angle
and simulation output with sidecars. Its table holds a vertical first row, rows spaced ever finer
toward the vertical and a maximum-range last row; its forward samples, on a lattice that holds
every row, are the linear interpolation of the rows, as the engine answers. `trim_export/mod.rs` holds the
tests, all named `ballistics_trim_export_*`: two runs write identical bytes; the documents validate
against the committed schemas and pin each other by hash; every row is matched by a forward sample
or a lattice end; and an unmatched row, a row between the forward lattice's points, a missing
export and oracle output that differs from its sidecar are refused without writing anything.

Run them with `cargo test -p ballistics_oracle_tooling`.
