//! Mission compiler — shared between the Axum backend (`/compiled` + event ORBAT derivation) and
//! the wasm client. Ported from the backend `services/mission_payload.rs` + `contract/kit_aliases.rs`
//! (T-145 Phase 2); the backend now re-exports these from here. Gated behind the `mission` feature
//! (serde/serde_json) so the DEM-only wasm/backend builds don't pull it. The mod-document flatten
//! (`flatten_to_mod_document`) lands here next, once decoupled from the backend `Mission` model.

pub mod flatten;
pub mod kit;
pub mod orbat;
