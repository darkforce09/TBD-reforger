// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/machine-credential.schema.json — regenerate with: cargo xtask ci schema-codegen

//! Types generated from `contracts_v2/definitions/machine-credential.schema.json`, one module per schema definition.

pub mod error;
mod executor_kind;
pub use executor_kind::*;
mod issued_machine_credential;
pub use issued_machine_credential::*;
mod machine_credential;
pub use machine_credential::*;
mod machine_credential_issue;
pub use machine_credential_issue::*;
mod machine_credential_list;
pub use machine_credential_list::*;
