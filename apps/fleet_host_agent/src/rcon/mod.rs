//! BattlEye RCon client for the Arma Reforger dedicated server.
//!
//! Arma Reforger's RCON server (the `rcon` block of the server config, UDP port 19999 by
//! default) speaks the BattlEye RCon protocol over UDP: a login with the password, command
//! packets numbered by a one-byte sequence that wraps after 255, responses that may arrive
//! split into parts in any order, server messages every client acknowledges, and a login that
//! lapses unless the client sends a command packet at least every 45 seconds.
//!
//! - `packet_codec`: the packet layout and its CRC32.
//! - `command_sequence`: command sequence numbers and their wrap-around.
//! - `response_assembly`: reassembly of split responses.
//! - `server_message_window`: duplicate detection for server messages.
//! - `rcon_session`: the task that owns the socket, logs in, retransmits, keeps the login alive
//!   and logs in again after the session lapses.
//! - [`RconClient`]: the handle the agent sends commands through.
//! - [`reforger_commands`]: the game commands this agent sends and the reading of their
//!   responses.

mod command_sequence;
mod packet_codec;
mod rcon_client;
mod rcon_session;
pub mod reforger_commands;
mod response_assembly;
mod server_message_window;

pub use rcon_client::{MAX_COMMAND_BYTES, RconClient, RconError, RconSettings, RconTimings};
