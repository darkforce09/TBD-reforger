//! Right dock eden behavior.

use super::*;

/// Ordered chip labels the DockRight row iterates. Gate E1/E5 pin this exact list.
pub const EDEN_SIDE_CHIPS: &[&str] = &["BLUFOR", "OPFOR", "INDFOR", "Objects"];

/// Which Eden chip is selected (side place vs Objects world-entity place).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdenChip {
    Blufor,
    Opfor,
    Indfor,
    Objects,
}

impl EdenChip {
    /// Chip row label / `aria-label`.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Blufor => "BLUFOR",
            Self::Opfor => "OPFOR",
            Self::Indfor => "INDFOR",
            Self::Objects => "Objects",
        }
    }

    /// Tailwind fill class (Aegis tokens matching map SIDE_* / tactical-yellow).
    pub const fn fill_class(self) -> &'static str {
        match self {
            Self::Blufor => "bg-primary",
            Self::Opfor => "bg-error-alert",
            Self::Indfor => "bg-success",
            Self::Objects => "bg-tactical-yellow",
        }
    }

    /// Parse a chip label from [`EDEN_SIDE_CHIPS`].
    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "BLUFOR" => Some(Self::Blufor),
            "OPFOR" => Some(Self::Opfor),
            "INDFOR" => Some(Self::Indfor),
            "Objects" => Some(Self::Objects),
            _ => None,
        }
    }
}

/// Apply a chip click to the shared place signals (same `active_side` EditorContext / `place_at` read).
///
/// Side chips clear Objects mode and set the place side. Objects sets `objects_mode` only (leaves
/// `active_side` unchanged so flipping back restores the last side).
pub fn apply_eden_chip(
    chip: EdenChip,
    active_side: RwSignal<String>,
    objects_mode: RwSignal<bool>,
) {
    match chip {
        EdenChip::Objects => objects_mode.set(true),
        EdenChip::Blufor => {
            objects_mode.set(false);
            active_side.set(String::from("BLUFOR"));
        }
        EdenChip::Opfor => {
            objects_mode.set(false);
            active_side.set(String::from("OPFOR"));
        }
        EdenChip::Indfor => {
            objects_mode.set(false);
            active_side.set(String::from("INDFOR"));
        }
    }
}

/// Whether the chip row should show `chip` as selected given current side + objects mode.
pub fn eden_chip_selected(chip: EdenChip, active_side: &str, objects_mode: bool) -> bool {
    match chip {
        EdenChip::Objects => objects_mode,
        EdenChip::Blufor => !objects_mode && active_side == "BLUFOR",
        EdenChip::Opfor => !objects_mode && active_side == "OPFOR",
        EdenChip::Indfor => !objects_mode && active_side == "INDFOR",
    }
}

/// which right-dock sub-mode the palette is showing. Eden cycles these with `Tab`; here they
/// map onto the dock's tabs. Only [`EdenSubmode::Groups`] (the character/squad-placing surface, the
/// Factions tab) reveals the Custom chip — Vehicles / Objects / Markers / Zones never do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdenSubmode {
    /// The Factions tab — placing characters that form groups/squads. Eden's "Groups" mode.
    Groups,
    /// The Vehicles tab.
    Vehicles,
    /// The Objects world-entity place (the Objects chip on the Factions tab).
    Objects,
    /// The Markers tab.
    Markers,
    /// The Zones tab .
    Zones,
    /// the Compositions tab (RIGHT-MODE-002).
    Compositions,
    /// the Triggers tab (RIGHT-MODE-003).
    Triggers,
    /// the Favourites tab (NEW-F5 / 3den E3): the starred-asset collection, not a palette
    /// over `/registry`. It is its own sub-mode for the same reason Compositions and Triggers are —
    /// so `from_tab` never reports a surface the operator is not looking at, and so the Groups-only
    /// Custom chip cannot leak onto it.
    Favourites,
}

impl EdenSubmode {
    /// Map a DockRight tab index (`0` Factions, `1` Vehicles, `2` Markers, `3` Zones, `4`
    /// Compositions, `5` Triggers, `6` Favourites) plus the Objects-chip flag to the sub-mode. The Objects chip lives
    /// on the Factions tab but is its own place surface, so it reports [`EdenSubmode::Objects`], not
    /// `Groups` — which is exactly why the Custom slot hides the moment the operator flips to Objects.
    #[must_use]
    pub fn from_tab(tab: usize, objects_mode: bool) -> Self {
        match tab {
            1 => Self::Vehicles,
            2 => Self::Markers,
            3 => Self::Zones,
            4 => Self::Compositions,
            5 => Self::Triggers,
            6 => Self::Favourites,
            _ if objects_mode => Self::Objects,
            _ => Self::Groups,
        }
    }
}

/// (RIGHT-SUBMODE-001) — the Custom chip's `aria-label` / row text. The sixth slot; a fixed
/// label so the gate can pin it without a render.
pub const EDEN_CUSTOM_CHIP: &str = "Custom";

/// (RIGHT-SUBMODE-001) — whether the Custom slot is shown in the chip row.
///
/// The whole rule in one predicate: **Custom appears only under Groups.** Every other sub-mode hides
/// it, so an author on the Vehicles or Objects surface never sees a group-only affordance.
#[must_use]
pub fn custom_chip_visible(submode: EdenSubmode) -> bool {
    matches!(submode, EdenSubmode::Groups)
}

/// The `placeholder=` tail shared by all three asset-browser search boxes.
pub const SEARCH_PLACEHOLDER_GRAMMAR: &str = " — class: mod: * /re/";

/// The worked-example line under every asset-browser search box.
pub const SEARCH_GRAMMAR_HINT: &str =
    "class:Character_US · mod:ArmaReforger · *Rifleman · /^us (mg|ar)$/";

/// The hint row rendered under each `type="search"` box.
pub(super) fn search_grammar_hint() -> impl IntoView {
    view! {
        <p
            class="mt-1 text-[10px] leading-tight text-outline"
            title="class: matches the Enfusion classname (the bare name works — the GUID head is optional). \
                   mod: matches the addon. * and ? are wildcards over the whole name. /…/ is a regex."
        >
            {SEARCH_GRAMMAR_HINT}
        </p>
    }
}

/// Display the catalogue failure cause and a retry action. A 404 from the
/// registry probe identifies the missing modpack; other errors are request failures.
pub(super) fn catalog_failure_view(
    noun: &'static str,
    no_modpack: RwSignal<bool>,
    registry_fetch_gen: RwSignal<u64>,
) -> AnyView {
    let cause = if no_modpack.get() {
        format!(
            "No modpack is configured, so the {noun} is empty. Set a current modpack, then retry."
        )
    } else {
        format!("Could not load the {noun}. The request to the registry failed.")
    };
    view! {
        <div class="flex flex-col gap-2" data-testid="catalog-failure">
            <p class="text-label-sm text-error" data-testid="catalog-failure-cause">
                {cause}
            </p>
            <button
                type="button"
                data-testid="catalog-failure-retry"
                class="self-start rounded border border-outline-variant/40 px-2 py-1 text-label-sm text-on-surface transition hover:bg-surface-container-high"
                on:click=move |_| {
                    registry_fetch_gen.update(|n| *n = n.wrapping_add(1));
                }
            >
                "Retry"
            </button>
        </div>
    }
    .into_any()
}
