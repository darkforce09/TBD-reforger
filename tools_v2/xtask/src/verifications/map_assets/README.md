# Map asset verification adapters

`mod.rs` forwards object golden, label, terrain-manifest, and BLAS-manifest checks to `developer_tools::map_verification`. The implementation and engine-dependent tests belong to that crate; `xtask` owns only command routing and repository-path discovery.
