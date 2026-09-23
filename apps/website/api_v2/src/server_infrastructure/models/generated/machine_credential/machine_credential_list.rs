// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/machine-credential.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::MachineCredential;

///GET /api/v1/servers/:id/credentials response. Revocation: DELETE /api/v1/servers/:id/credentials/:credentialId?reason=... answers the revoked MachineCredential.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct MachineCredentialList {
    pub items: ::std::vec::Vec<MachineCredential>,
}
