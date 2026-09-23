// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/fleet-command.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::FleetCommandReceipt;

///One fleet command as operators observe it. POST /api/v1/servers/:id/commands (administrator) answers 202 with the receipt of an accepted command; GET /api/v1/servers/:id/commands (FleetCommandList, newest first, ?limit=1..100&offset=) and GET /api/v1/servers/:id/commands/:commandId read receipts; POST .../:commandId/cancel cancels a queued command (409 COMMAND_NOT_CANCELLABLE otherwise). queued -> claimed -> executing -> succeeded|failed; queued -> cancelled|expired; a lapsed claim returns to queued; a lapse during the effect makes a non-idempotent command indeterminate, which nothing repeats.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(transparent)]
pub struct FleetCommandContract(pub FleetCommandReceipt);
impl ::std::ops::Deref for FleetCommandContract {
    type Target = FleetCommandReceipt;
    fn deref(&self) -> &FleetCommandReceipt {
        &self.0
    }
}
impl ::std::convert::From<FleetCommandContract> for FleetCommandReceipt {
    fn from(value: FleetCommandContract) -> Self {
        value.0
    }
}
impl ::std::convert::From<FleetCommandReceipt> for FleetCommandContract {
    fn from(value: FleetCommandReceipt) -> Self {
        Self(value)
    }
}
