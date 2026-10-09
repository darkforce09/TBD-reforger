# Check crates

The repository verifications as libraries: the structural, licensing and registry checks of the
repository, the mod script checks and the documentation gates. Each check concludes in a
`verification_core` verdict, and the xtask binary and the CI task catalog run them.

## Contents

```text
tools/checks/
├── documentation_checks/  `documentation_checks`: the link check with its backticked-path and command-citation rules (`verify`)
├── mod_script_checks/  `mod_script_checks`: the UI layout gate and the spawn runs (`verify`, `mod spawn-determinism`, `mod spawn-verify`)
└── repository_checks/  `repository_checks`: workspace laws, the language bans, upstream code leaks and registry aliases (`verify`)
```
