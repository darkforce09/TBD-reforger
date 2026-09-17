# Map command adapters

`mod.rs` resolves the active repository root and forwards existing blueprint, ingestion, parity, and world-LOS command arguments to `developer-tools`. The adapter preserves `Result<u8>` exit handling and imports no map-engine types.

Other map export commands retain their current routing until the later dispatcher decomposition.
