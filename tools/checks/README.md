# Check crates

The repository verifications as libraries: the structural, licensing and registry checks of the
repository, the mod script checks and the documentation gates. Each check concludes in a
`verification_core` verdict, and the xtask binary and the CI task catalog run them.

## Contents

```text
tools/checks/
├── documentation_checks/  `documentation_checks`: README coverage with its Contents check, Markdown placement with its size limit, and the link check with its backticked-path and command-citation rules (`verify`, `ci verify-documentation`)
├── mod_script_checks/  `mod_script_checks`: the Enfusion comment card, the identity-comment, destroy-target and mission size pins, the UI layout gate and the spawn runs (`verify`, `mod spawn-determinism`, `mod spawn-verify`)
└── repository_checks/  `repository_checks`: workspace laws, route tags, ORBAT coherency, the language bans and file length, upstream code leaks, registry aliases, and the tooling rules over every tool crate (`verify`)
```
