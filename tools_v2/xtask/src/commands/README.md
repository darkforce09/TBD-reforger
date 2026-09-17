# Domain command adapters

The live command modules delegate to domain libraries:

- `ticket`: ticket-engine services plus host-side agent execution and cleanup.
- `wave`: ticket-engine lock compilation, checking, and collision selection.
- `map`: developer-tools asset processing.

Other directory scaffolds describe phase-four targets. Their commands continue to use the existing xtask modules and CLI declarations until that phase.
