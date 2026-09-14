# Server Control & RCON (`/admin/server`)

*Replaces the 1,635-line monolithic `pages/admin/server_control.rs` file.*

## Architecture
- **`page.rs`**: Main layout file (~250 LOC).
- **`server_cards.rs`**: Server state cards with Start, Stop, and Restart controls.
- **`rcon_console.rs`**: Real-time terminal emulator for issuing RCON commands (`#kick`, `#ban`, `#missions`).
