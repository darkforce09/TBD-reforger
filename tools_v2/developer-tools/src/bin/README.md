# Executable entrypoints

The stable executable names are `enf`, `gate`, `mcpd`, `world`, `map`, and `capture`. Each file delegates to its subsystem CLI implementation and stays below 250 lines. Argument parsing, help, error handling, and runtime cleanup remain in the owning subsystem.
