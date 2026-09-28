//! A map position typed as a grid reference, with its height from the terrain or by hand.
//!
//! **Role:** the terrain choice, the draft of one position (grid text, height source, manual
//! height text), its resolution into map metres and a height, and the fields that edit it.
//! **Position:** the target row and every gun row of the inputs card; the battery reuses
//! [`PositionDraft`] and [`resolve_position`] per gun, and the solve bridge reads the resolved
//! [`FireMissionPoint`]. Grid references go through the map engine's
//! [`website_map_engine::camera::grid_reference::parse_grid`]; terrain heights come from the
//! page's [`crate::v2::core::map_view::terrain_height::TerrainHeights`].
//! **Signals & state:** the view reads and writes the page's draft and terrain signals; the pure
//! functions hold nothing.
//! **Invariants:** a grid reference of 6, 8 or 10 figures resolves to its cell centre; a terrain
//! without an elevation model (Arland) only ever resolves manual heights; a terrain height that
//! is not there (raster not loaded, or outside it) is an error, never a guessed zero.

use super::INPUT_CLASS;
use leptos::prelude::*;
use website_map_engine::camera::grid_reference::{parse_grid, GridParseError};
use website_map_engine::data::scenario::ballistics::fire_mission::{
    FireMissionPoint, HeightSource,
};

/// The terrain the positions are on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum MortarTerrain {
    /// Everon: served elevation model, heights sampled at 2 m.
    Everon,
    /// Arland: no served elevation model, heights typed by hand.
    Arland,
}

impl MortarTerrain {
    /// Every terrain the calculator offers.
    pub(crate) const ALL: [Self; 2] = [Self::Everon, Self::Arland];

    /// Terrain identifier of the map assets (`/map-assets/<id>/…`).
    pub(crate) fn terrain_id(self) -> &'static str {
        match self {
            Self::Everon => "everon",
            Self::Arland => "arland",
        }
    }

    /// The terrain named by `terrain_id`, if offered.
    pub(crate) fn from_terrain_id(terrain_id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.terrain_id() == terrain_id)
    }

    /// Name shown in the picker.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Everon => "Everon",
            Self::Arland => "Arland (manual heights)",
        }
    }

    /// Whether heights can be sampled from a served elevation model.
    pub(crate) fn has_elevation_model(self) -> bool {
        matches!(self, Self::Everon)
    }
}

/// Where a position's height is taken from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HeightChoice {
    /// Sampled from the terrain's elevation model.
    Terrain,
    /// Typed by the operator.
    Manual,
}

/// The height source that applies on `terrain`: a terrain without an elevation model forces a
/// manual height whatever was chosen.
pub(crate) fn effective_height_choice(
    terrain: MortarTerrain,
    choice: HeightChoice,
) -> HeightChoice {
    if terrain.has_elevation_model() {
        choice
    } else {
        HeightChoice::Manual
    }
}

/// One position as typed.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PositionDraft {
    /// Grid reference text, 6, 8 or 10 figures.
    pub(crate) grid: String,
    /// Where the height comes from.
    pub(crate) height_choice: HeightChoice,
    /// Manual height text in metres; read only for a manual height.
    pub(crate) manual_height: String,
}

impl Default for PositionDraft {
    fn default() -> Self {
        Self {
            grid: String::new(),
            height_choice: HeightChoice::Terrain,
            manual_height: String::new(),
        }
    }
}

/// Why a position does not resolve.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum PositionError {
    /// The grid reference does not parse.
    Grid(GridParseError),
    /// No terrain height exists there yet: the raster is not loaded, or the point is off it.
    TerrainHeightUnavailable,
    /// The manual height is empty.
    ManualHeightMissing,
    /// The manual height is not a finite number of metres.
    ManualHeightInvalid(String),
}

/// The sentence the page shows for a position error.
pub(crate) fn position_error_message(error: &PositionError) -> String {
    match error {
        PositionError::Grid(grid) => format!("Grid: {grid}."),
        PositionError::TerrainHeightUnavailable => "No terrain height here yet (the map heights \
             have not loaded, or the point is off the terrain); enter a manual height."
            .to_string(),
        PositionError::ManualHeightMissing => "Enter a height in metres.".to_string(),
        PositionError::ManualHeightInvalid(text) => {
            format!("Height \"{text}\" is not a number of metres.")
        }
    }
}

/// Resolves a draft on `terrain` into map metres and a height; `height_at(x, y)` samples the
/// terrain's elevation model.
///
/// # Errors
///
/// The first [`PositionError`]: the grid is checked before the height.
pub(crate) fn resolve_position(
    draft: &PositionDraft,
    terrain: MortarTerrain,
    height_at: impl Fn(f64, f64) -> Option<f64>,
) -> Result<FireMissionPoint, PositionError> {
    let (x, y) = parse_grid(&draft.grid).map_err(PositionError::Grid)?;
    let (height_m, height_source) = match effective_height_choice(terrain, draft.height_choice) {
        HeightChoice::Terrain => (
            height_at(x, y)
                .filter(|h| h.is_finite())
                .ok_or(PositionError::TerrainHeightUnavailable)?,
            HeightSource::Dem,
        ),
        HeightChoice::Manual => (
            parse_manual_height(&draft.manual_height)?,
            HeightSource::Manual,
        ),
    };
    Ok(FireMissionPoint {
        x,
        y,
        height_m,
        height_source,
    })
}

/// Parses a manual height: a finite number of metres, surrounding whitespace ignored.
fn parse_manual_height(text: &str) -> Result<f64, PositionError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(PositionError::ManualHeightMissing);
    }
    trimmed
        .parse::<f64>()
        .ok()
        .filter(|h| h.is_finite())
        .ok_or_else(|| PositionError::ManualHeightInvalid(trimmed.to_string()))
}

/// The line under a grid field: the cell centre in metres, or why the text does not parse.
pub(crate) fn grid_preview(grid: &str) -> String {
    if grid.trim().is_empty() {
        return "6, 8 or 10 figures, e.g. 064 129".to_string();
    }
    match parse_grid(grid) {
        Ok((x, y)) => format!("x {x:.1} m, y {y:.1} m"),
        Err(error) => format!("{error}"),
    }
}

/// The terrain picker.
pub(crate) fn terrain_select(terrain: RwSignal<MortarTerrain>) -> impl IntoView {
    view! {
        <label class="text-sm">
            "Terrain"
            <select
                prop:value=move || terrain.get().terrain_id()
                on:change=move |ev| {
                    if let Some(t) = MortarTerrain::from_terrain_id(&event_target_value(&ev)) {
                        terrain.set(t);
                    }
                }
                class=INPUT_CLASS
                data-mortar-input="terrain"
            >
                {MortarTerrain::ALL
                    .into_iter()
                    .map(|t| view! { <option value=t.terrain_id()>{t.label()}</option> })
                    .collect_view()}
            </select>
        </label>
    }
}

/// The grid, height-source and manual-height fields of one position. `set` writes an edited
/// draft back; `draft` reads the current one.
pub(crate) fn position_fields(
    name: &'static str,
    draft: Signal<PositionDraft>,
    set: impl Fn(PositionDraft) + Copy + Send + Sync + 'static,
    terrain: RwSignal<MortarTerrain>,
) -> impl IntoView {
    let choice = move || effective_height_choice(terrain.get(), draft.with(|d| d.height_choice));
    view! {
        <div class="grid gap-2 sm:grid-cols-3" data-mortar-position=name>
            <label class="text-sm">
                {format!("{name} grid")}
                <input
                    type="text"
                    inputmode="numeric"
                    placeholder="064 129"
                    prop:value=move || draft.with(|d| d.grid.clone())
                    on:input=move |ev| {
                        let mut next = draft.get_untracked();
                        next.grid = event_target_value(&ev);
                        set(next);
                    }
                    class=INPUT_CLASS
                />
                <span class="text-xs text-on-surface-variant">
                    {move || draft.with(|d| grid_preview(&d.grid))}
                </span>
            </label>
            <label class="text-sm">
                "Height from"
                <select
                    prop:value=move || match choice() {
                        HeightChoice::Terrain => "terrain",
                        HeightChoice::Manual => "manual",
                    }
                    on:change=move |ev| {
                        let mut next = draft.get_untracked();
                        next.height_choice = if event_target_value(&ev) == "terrain" {
                            HeightChoice::Terrain
                        } else {
                            HeightChoice::Manual
                        };
                        set(next);
                    }
                    class=INPUT_CLASS
                >
                    <option value="terrain" disabled=move || !terrain.get().has_elevation_model()>
                        "Terrain (2 m elevation model)"
                    </option>
                    <option value="manual">"Manual height"</option>
                </select>
            </label>
            <label class="text-sm">
                "Height (m)"
                <input
                    type="number"
                    step="0.1"
                    prop:value=move || draft.with(|d| d.manual_height.clone())
                    prop:disabled=move || choice() == HeightChoice::Terrain
                    on:input=move |ev| {
                        let mut next = draft.get_untracked();
                        next.manual_height = event_target_value(&ev);
                        set(next);
                    }
                    class=INPUT_CLASS
                />
            </label>
        </div>
    }
}
