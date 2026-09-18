//! Content domain — [`cms`], [`wiki`], public [`announcements`] reads, and
//! [`modpacks`]. T-934.15: the flat files moved here unchanged. The plan's
//! `mod_portal.rs` slot is empty on purpose: the flat tree never had a `mod`-named
//! handler file — `handlers/mod.rs` was (and remains) the module root + shared
//! helpers, and the mod-portal surface is `modpacks.rs`.

pub mod announcements;
pub mod cms;
pub mod modpacks;
pub mod wiki;
