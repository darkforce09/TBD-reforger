//! Role: aliases.
//! Position: `mission/compiler/kit` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::{Deserialize, HashMap, OnceLock};

/// Canonical kit aliases raw value.
pub(super) const KIT_ALIASES_RAW: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../contracts_v2/rules/kit-aliases.json"
));

/// Domain representation of kit entry.
#[derive(Debug, Deserialize)]
pub(super) struct KitEntry {
    /// Alias.
    pub(super) alias: String,
    /// Resource name.
    #[serde(rename = "resourceName")]
    pub(super) resource_name: String,
}

/// Domain representation of faction default.
#[derive(Debug, Deserialize)]
pub(super) struct FactionDefault {
    /// Kit.
    pub(super) kit: String,
    /// Preset.
    pub(super) preset: String,
}

/// Domain representation of kit aliases raw.
#[derive(Debug, Deserialize)]
pub(super) struct KitAliasesRaw {
    /// Kits.
    pub(super) kits: Vec<KitEntry>,

    /// Vehicles.
    #[serde(default)]
    pub(super) vehicles: Vec<KitEntry>,
    /// Faction defaults.
    #[serde(rename = "factionDefaults")]
    pub(super) faction_defaults: HashMap<String, FactionDefault>,
    /// Fallback faction.
    #[serde(rename = "fallbackFaction")]
    pub(super) fallback_faction: String,
}

/// Parsed kit-aliases: the `resourceName → kit:` / `veh:` maps + per-faction fallbacks.
pub struct KitAliases {
    /// Resource to kit.
    pub(super) resource_to_kit: HashMap<String, String>,
    /// Resource to vehicle.
    pub(super) resource_to_vehicle: HashMap<String, String>,
    /// Faction defaults.
    pub(super) faction_defaults: HashMap<String, FactionDefault>,
    /// Fallback faction.
    pub(super) fallback_faction: String,
}

impl KitAliases {
    /// Resolve a slot `assetId` (full Enfusion ResourceName) to its `kit:` alias. `None` means the caller should fall back to the faction default kit.
    pub fn kit_for_resource(&self, resource_name: &str) -> Option<&str> {
        self.resource_to_kit.get(resource_name).map(String::as_str)
    }

    /// Vehicle for resource using the supplied domain data.
    pub fn vehicle_for_resource(&self, resource_name: &str) -> Option<&str> {
        self.resource_to_vehicle
            .get(resource_name)
            .map(String::as_str)
    }

    /// Fallback `(kit, preset)` aliases for a (lowercased) faction key, falling back to the table's `fallbackFaction` for unknown factions.
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

/// Parse the embedded kit-aliases.json exactly once (the embedded copy is committed and known-good, so a parse failure is a build-time bug).
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
        let resource_to_vehicle = raw
            .vehicles
            .into_iter()
            .map(|k| (k.resource_name, k.alias))
            .collect();
        KitAliases {
            resource_to_kit,
            resource_to_vehicle,
            faction_defaults: raw.faction_defaults,
            fallback_faction: raw.fallback_faction,
        }
    })
}
