//! The generators: the contract codegen and the `gen` command group.
//!
//! **Role:** declares the contract codegen ([`schema_types`]), the Spleen font-table generator
//! (`font_table`) and the `gen` group's arguments ([`cli`]) and dispatch ([`dispatch`]).
//! **Position:** private to the crate; the crate root re-exports the entries the xtask binary
//! calls.
//! **Signals & state:** none.
//! **Invariants:** a generator writes only its own output: the codegen the generated folder of
//! `contract_schema_types`, the font table its stdout.

pub(crate) mod cli;
pub(crate) mod dispatch;

mod font_table;

pub(crate) mod schema_types;
