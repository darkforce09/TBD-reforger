//! Outbound HTTP behaviour shared by every client this service speaks through (Discord, the
//! announcement webhook): the retry policy that sits between a `RequestBuilder` and the network.

pub mod retry_on_429;
