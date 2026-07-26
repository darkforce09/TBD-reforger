//! Kit-aliases table — Rust port of the `KitAliases` half of `internal/contract/mission.go`.
//!
//! Embedded directly from the canonical `packages/tbd-schema/registry/kit-aliases.json`
//! (Rust `include_str!` can reach outside the crate, so no copy step is needed — unlike
//! Go's `go:embed`). Consumed by the mission-compile flatten (Phase 8).

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

const KIT_ALIASES_RAW: &str =
    include_str!("../../../../packages/tbd-schema/registry/kit-aliases.json");

#[derive(Debug, Deserialize)]
struct KitEntry {
    alias: String,
    #[serde(rename = "resourceName")]
    resource_name: String,
}

#[derive(Debug, Deserialize)]
struct FactionDefault {
    kit: String,
    preset: String,
}

#[derive(Debug, Deserialize)]
struct KitAliasesRaw {
    kits: Vec<KitEntry>,
    #[serde(rename = "factionDefaults")]
    faction_defaults: HashMap<String, FactionDefault>,
    #[serde(rename = "fallbackFaction")]
    fallback_faction: String,
}

/// Parsed kit-aliases: the `resourceName → kit:` alias map + per-faction fallbacks.
pub struct KitAliases {
    resource_to_kit: HashMap<String, String>,
    faction_defaults: HashMap<String, FactionDefault>,
    fallback_faction: String,
}

impl KitAliases {
    /// Resolve a slot `assetId` (full Enfusion ResourceName) to its `kit:` alias.
    /// `None` means the caller should fall back to the faction default kit.
    pub fn kit_for_resource(&self, resource_name: &str) -> Option<&str> {
        self.resource_to_kit.get(resource_name).map(String::as_str)
    }

    /// Fallback `(kit, preset)` aliases for a (lowercased) faction key, falling back
    /// to the table's `fallbackFaction` for unknown factions.
    pub fn faction_default(&self, faction_key: &str) -> (&str, &str) {
        let d = self
            .faction_defaults
            .get(faction_key)
            .or_else(|| self.faction_defaults.get(&self.fallback_faction));
        match d {
            Some(fd) => (fd.kit.as_str(), fd.preset.as_str()),
            None => ("", ""),
        }
    }
}

/// Parse the embedded kit-aliases.json exactly once (the embedded copy is committed
/// and known-good, so a parse failure is a build-time bug).
pub fn load_kit_aliases() -> &'static KitAliases {
    static ALIASES: OnceLock<KitAliases> = OnceLock::new();
    ALIASES.get_or_init(|| {
        let raw: KitAliasesRaw =
            serde_json::from_str(KIT_ALIASES_RAW).expect("parse embedded kit-aliases.json");
        let resource_to_kit = raw
            .kits
            .into_iter()
            .map(|k| (k.resource_name, k.alias))
            .collect();
        KitAliases {
            resource_to_kit,
            faction_defaults: raw.faction_defaults,
            fallback_faction: raw.fallback_faction,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_known_kits_and_faction_defaults() {
        let k = load_kit_aliases();
        assert_eq!(
            k.kit_for_resource(
                "{84029128FA6F6BB9}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_GL.et"
            ),
            Some("kit:us_sl")
        );
        assert_eq!(k.kit_for_resource("unknown-resource"), None);
        assert_eq!(
            k.faction_default("blufor"),
            ("kit:us_rifleman", "preset:us_army_82nd")
        );
        assert_eq!(
            k.faction_default("opfor"),
            ("kit:sov_rifleman", "preset:sov_vdv")
        );
        // Unknown faction → fallbackFaction (blufor).
        assert_eq!(
            k.faction_default("mystery"),
            ("kit:us_rifleman", "preset:us_army_82nd")
        );
    }

    /// T-183 — `fallbackFaction` is the guard for a genuinely UNKNOWN faction key (the
    /// `"mystery"` case above). It must never be load-bearing for a key the mod itself knows.
    /// `TBD_SpawnManager.EngineFactionKey` maps blufor/opfor/indfor/civ, but `factionDefaults`
    /// carried only blufor/opfor — so `indfor` silently took the blufor fallback and every
    /// INDFOR slot compiled to a US Army body that the mod then spawned under engine faction
    /// FIA. `civ` had the same hole, latent only because `apply_faction.rs` VALID_SIDES cannot
    /// mint it. Nothing errored: an unmapped faction is indistinguishable from a mapped one
    /// once it resolves, which is precisely why this needs an explicit test.
    #[test]
    fn every_mod_faction_key_has_its_own_default() {
        let k = load_kit_aliases();
        assert_eq!(
            k.faction_default("indfor"),
            ("kit:fia_rifleman", "preset:fia")
        );
        assert_eq!(k.faction_default("civ"), ("kit:civ_generic", "preset:civ"));

        // Resolving is not enough — deleting a row would still return a valid-looking pair
        // (the blufor fallback). Pin the DIFFERENCE, which is what the bug actually was.
        let blufor = k.faction_default("blufor");
        for side in ["opfor", "indfor", "civ"] {
            assert_ne!(
                k.faction_default(side),
                blufor,
                "{side} resolved to the blufor fallback — its factionDefaults row is missing"
            );
        }
    }
}
