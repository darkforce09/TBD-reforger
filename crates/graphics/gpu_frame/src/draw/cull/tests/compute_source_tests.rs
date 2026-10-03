//! Role: compute cull source tests.
//! Position: `draw/cull/tests` in the GPU frame crate.
//! Signals & state: the source text of `compute.rs`, read at compile time.
//! Invariants: `compute.rs` is WebAssembly-only, so its per-lane parameter binding is pinned by
//! scrubbing its source text on the native test target; the test reads the file by name, so the
//! name `compute.rs` is load-bearing.

/// `true` when every culled lane binds its **own** `cull_params` uniform.
fn cull_params_is_per_lane() -> bool {
    let src = include_str!("../compute.rs");
    let prod = src.split("#[cfg(test)]").next().unwrap_or(src);

    prod.contains("queue.write_buffer(&slot.params_buf, 0, &params);")
        && prod.contains("resource: slot.params_buf.as_entire_binding(),")
        && !prod.contains("self.params_buf")
}

#[test]
fn per_lane_cull_params_not_shared() {
    assert!(
        cull_params_is_per_lane(),
        "every cull lane must own its cull_params uniform: one shared buffer plus one \
             staged write per lane makes all lanes read the last lane's src_count"
    );
}
