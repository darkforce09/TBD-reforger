// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/game-runtime-session.schema.json — regenerate with: cargo xtask ci schema-codegen

//! Types generated from `contracts_v2/definitions/game-runtime-session.schema.json`, one module per schema definition.

pub mod error;
mod runtime_fence_refusal;
pub use runtime_fence_refusal::*;
mod runtime_heartbeat;
pub use runtime_heartbeat::*;
mod runtime_session_end;
pub use runtime_session_end::*;
mod runtime_session_start;
pub use runtime_session_start::*;
mod started_runtime_session;
pub use started_runtime_session::*;
