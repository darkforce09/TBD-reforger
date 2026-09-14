# Dashboard Page (`/`)

## Architecture
Structured as a top-level layout file (`page.rs`) assembling 5 focused panel components:
- **`hero_banner.rs`**: Unit greeting, operational motto, and T-minus clock to the next major operation.
- **`server_uplink.rs`**: Quick live status of primary and secondary dedicated servers.
- **`deployment.rs`**: Personal attendance summary, operational service record, and active LOA badge.
- **`modpack.rs`**: Active modpack version status and fast 1-click update link.
- **`recent_intel.rs`**: Summary feed of the latest announcements and intelligence updates.
