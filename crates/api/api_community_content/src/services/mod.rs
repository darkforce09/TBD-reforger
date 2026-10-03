//! Community-content services: the modpack manifest loaders shared by the modpack endpoints and
//! the telemetry dashboard, and the wiki markup renderer. The announcement webhook lives in
//! [`api_discord`] and the equipment datasets in [`api_equipment_datasets`].

pub mod modpack_lookup;
pub mod wiki_markup;
