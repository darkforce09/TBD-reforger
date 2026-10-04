//! The zone vocabulary and the zone geometry the draw tool commits.
//!
//! **Role:** reads the zone types and the per-type rule fields out of the embedded mission schema,
//! and holds the pure geometry a zone is authored from: the circle from two clicks, the polygon
//! that may commit, the quantisation the compile applies, the whole-terrain play area ring, and
//! the projected owner link.
//! **Position:** part of the editor's state layer. The zones panel, the trigger attributes, the
//! settings catalog, the marker icon tables and the zone draw host read it; it reads the mission
//! schema, `mission_payload` and, for the play area write, the hosted commands.
//! **Signals & state:** none; the parsed schema vocabulary is cached once per process.
//! **Invariants:** the schema embedded here is the one copy of `mission.schema.json` the editor
//! reads; a coordinate is quantised exactly as the compile quantises it.

mod zone_geometry;
mod zone_schema_vocabulary;

pub use zone_geometry::ProjectedOwnerLine;
pub use zone_geometry::add_whole_terrain_zone;
pub use zone_geometry::round_coord;
pub use zone_geometry::{
    MIN_AUTHORABLE_RADIUS_M, ZONE_GRID_M, circle_from_clicks, polygon_flat, polygon_is_committable,
    project_owner_line,
};
pub use zone_geometry::{
    WHOLE_TERRAIN_ZONE_LABEL, radius_survives_compile, terrain_rect_is_authorable,
    terrain_rect_ring, whole_terrain_zone_type,
};
pub use zone_schema_vocabulary::MISSION_SCHEMA;
pub use zone_schema_vocabulary::ZoneRuleField;
pub use zone_schema_vocabulary::{
    ZoneRuleKind, humanize_key, humanize_token, zone_rule_fields, zone_types,
};
