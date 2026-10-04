//! The live entity operation sources the editor's structural source pins read.
//!
//! **Role:** joins the bridge's host state shards with the hosted command files of
//! `mission_editing_commands` ([`entity`], [`context`]) and the entity operations of
//! `mission_operations` ([`domain_entity`]), so a pin searches the code an operation runs.
//! **Position:** test support of the `mission_document` and `mission_operations` boundary: every
//! shard is embedded at compile time, the other crates' files by their path from this crate's
//! manifest.
//! **Signals & state:** one process-wide cached string per source set.
//! **Invariants:** each production shard is joined once; the domain and the adapter sources stay
//! distinct.

/// The live entity operation surface: the frontend shards that carry host state — the armed
/// placement and the entity selection — and the map-engine hosted commands the rest of them
/// became.
pub fn entity() -> &'static str {
    static SOURCE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    SOURCE.get_or_init(|| {
        [
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/bridge/host_state/armed_placement/mod.rs"
    )),
    "\n",
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/bridge/host_state/armed_placement/palette_arming.rs"
    )),
    "\n",
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/bridge/host_state/armed_placement/map_release.rs"
    )),
    "\n",
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/bridge/host_state/armed_placement/zone_draw.rs"
    )),
    "\n",
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/bridge/host_state/entity_selection.rs"
    )),
    "\n",
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../crates/mission_editing/mission_editing_commands/src/hosted_commands/document_edit.rs")),
    "\n",
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../crates/mission_editing/mission_editing_commands/src/hosted_commands/document_search.rs")),
    "\n",
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../crates/mission_editing/mission_editing_commands/src/hosted_commands/entity_clipboard.rs")),
    "\n",
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../crates/mission_editing/mission_editing_commands/src/hosted_commands/orbat_roster.rs")),
    "\n",
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../crates/mission_editing/mission_editing_commands/src/hosted_commands/placed_vehicles.rs")),
    "\n",
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../crates/mission_editing/mission_editing_commands/src/hosted_commands/entity_connections.rs")),
    "\n",
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../crates/mission_editing/mission_editing_commands/src/hosted_commands/map_markers.rs")),
    "\n",
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../crates/mission_editing/mission_editing_commands/src/hosted_commands/zone_authoring.rs")),
    "\n",
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../crates/mission_editing/mission_editing_commands/src/hosted_commands/map_comments.rs")),
    "\n",
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../crates/mission_editing/mission_editing_commands/src/hosted_commands/editor_layers.rs")),
    "\n",
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../crates/mission_editing/mission_editing_commands/src/hosted_commands/map_triggers.rs")),
    "\n",
]
        .concat()
    })
}

/// The installed editor context: the handles and signals the editor's panels reach the open
/// mission through, and the document reads and writes that go through them.
pub fn context() -> &'static str {
    static SOURCE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    SOURCE.get_or_init(|| {
        [
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/bridge/host_state/editor_context/mod.rs"
            )),
            "\n",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/bridge/host_state/editor_context/installation.rs"
            )),
            "\n",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/bridge/host_state/editor_context/document_fields.rs"
            )),
            "\n",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/bridge/host_state/editor_context/attributes_modal.rs"
            )),
            "\n",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/bridge/host_state/editor_context/dock_mirrors.rs"
            )),
            "\n",
        ]
        .concat()
    })
}

/// Mission document operations source.
pub fn domain_entity() -> &'static str {
    static SOURCE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    SOURCE.get_or_init(|| {
        [
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../../crates/mission/mission_operations/src/entity/mod.rs"
            )),
            "\n",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../../crates/mission/mission_operations/src/entity/selection.rs"
            )),
            "\n",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../../crates/mission/mission_operations/src/entity/identity.rs"
            )),
            "\n",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../../crates/mission/mission_operations/src/entity/comments.rs"
            )),
            "\n",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../../crates/mission/mission_operations/src/entity/connections.rs"
            )),
            "\n",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../../crates/mission/mission_operations/src/entity/roster.rs"
            )),
            "\n",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../../crates/mission/mission_operations/src/entity/vehicles.rs"
            )),
            "\n",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../../crates/mission/mission_operations/src/entity/markers.rs"
            )),
            "\n",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../../crates/mission/mission_operations/src/entity/zones.rs"
            )),
            "\n",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../../crates/mission/mission_operations/src/entity/clipboard.rs"
            )),
            "\n",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../../crates/mission/mission_operations/src/entity/placement.rs"
            )),
            "\n",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../../crates/mission/mission_operations/src/entity/factions.rs"
            )),
            "\n",
        ]
        .concat()
    })
}
