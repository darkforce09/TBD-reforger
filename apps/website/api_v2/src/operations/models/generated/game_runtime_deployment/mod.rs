// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/game-runtime-deployment.schema.json — regenerate with: cargo xtask ci schema-codegen

//! Types generated from `contracts_v2/definitions/game-runtime-deployment.schema.json`, one module per schema definition.

mod deployment_allowed;
pub mod error;
pub use deployment_allowed::*;
mod deployment_decision;
pub use deployment_decision::*;
mod deployment_denied;
pub use deployment_denied::*;
mod deployment_request;
pub use deployment_request::*;
mod ended_life;
pub use ended_life::*;
