//! HTTP handlers for the server fleet: the intel reads, the registry writes, the live status
//! stream, and the two halves of the RCON console (request parsing, then delivery).

pub mod rcon_command_parser;
pub mod rcon_console;
pub mod server_intel;
pub mod server_registry;
pub mod server_status_stream;
