//! Right dock markers behavior.

use super::*;

/// `mission.schema.json` via the crate's single embed — the ONE source of the marker icon vocabulary.
pub(in crate::v2::apps::editor::ui::docks::dock_right) const MISSION_SCHEMA_JSON: &str =
    crate::v2::apps::editor::ui::inspector::zones_panel::MISSION_SCHEMA;

/// The closed `$defs/marker.icon` alias list, in schema order, parsed once.
///
/// Schema order is kept rather than sorted alphabetically: the enum opens with the paired base
/// glyphs (`dot` / `dot2`, `objective_marker` / `objective_marker2`, …) and then runs through the
/// semantic aliases, which is a more useful browse order than the alphabet, and it is the order a
/// reader comparing this list against the schema will see.
///
/// An empty list is the honest answer if the schema ever stops declaring the enum — every writer
/// gates on [`marker_icon_is_authorable`], so the surface would refuse to author rather than fall
/// back to a guess.
#[must_use]
pub fn marker_icons() -> &'static [String] {
    static ICONS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    ICONS.get_or_init(|| {
        let Ok(schema) = serde_json::from_str::<serde_json::Value>(MISSION_SCHEMA_JSON) else {
            return Vec::new();
        };
        schema
            .get("$defs")
            .and_then(|d| d.get("marker"))
            .and_then(|m| m.get("properties"))
            .and_then(|p| p.get("icon"))
            .and_then(|i| i.get("enum"))
            .and_then(serde_json::Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(ToString::to_string))
                    .collect()
            })
            .unwrap_or_default()
    })
}

/// Is `icon` one of the closed `$defs/marker.icon` aliases?
///
/// Every marker write in [`website_map_engine::editing::hosted_commands::map_markers`] passes through this. It is exact and
/// case-SENSITIVE: the enum is lower-case and `additionalProperties`-style validators do not
/// case-fold, so accepting `"Objective"` here would author a value the schema rejects at save time,
/// far from the control that produced it.
#[must_use]
pub fn marker_icon_is_authorable(icon: &str) -> bool {
    marker_icons().iter().any(|a| a == icon)
}

/// The alias a fresh place uses when the author has not picked one — the schema enum's first entry
/// (`dot`), not a literal. Empty only if the schema stopped declaring the enum.
#[must_use]
pub fn default_marker_icon() -> &'static str {
    marker_icons().first().map_or("", String::as_str)
}

/// The icon rows a search box shows: a case-insensitive SUBSTRING match over the closed list, with
/// an empty/whitespace query meaning "all of them".
///
/// Substring rather than prefix because the aliases are compound (`point_of_interest`,
/// `rally_point`, `observation_post`) and an author looking for a rally point types "rally" or
/// "point" with equal likelihood. The match also folds `_` to a space so typing "rally point"
/// finds `rally_point` — the alias is a token, but nobody reads it as one.
#[must_use]
pub fn filter_marker_icons(query: &str) -> Vec<&'static str> {
    let q = query.trim().to_ascii_lowercase();
    marker_icons()
        .iter()
        .map(String::as_str)
        .filter(|a| q.is_empty() || a.contains(&q) || a.replace('_', " ").contains(&q))
        .collect()
}

/// One representative schema alias per canonical [`map_engine_render::scene::MarkerGlyph`] family,
/// in the schema-order the families first appear in (the browse order the picker keeps).
///
/// Each entry is (a) a member of the closed `$defs/marker.icon` enum — so a pick validates and saves —
/// and (b) folds back to its own family via `scene::marker_glyph_for_alias`, so picking it makes the
/// map draw that family's glyph. Invariant (a) is asserted natively
/// ([`tests::picker_has_one_row_per_canonical_icon`]); invariant (b) is enforced on the wasm side by
/// the runtime `debug_assert` + self-healing fallback in [`canonical_marker_rows`] (the mapper is a
/// wasm32-only dep the native tests cannot link).
///
/// The label a row shows is `humanize_token` of the slug, so the human-readable stem was chosen over
/// the schema's paired base glyph where they differ (`objective`, not `objective_marker`; `medical`,
/// not `cross`) — the base still lives in that family and matches search via the alias list.
pub(in crate::v2::apps::editor::ui::docks::dock_right) const CANONICAL_MARKER_SLUGS: [&str;
    CANONICAL_MARKER_GLYPH_COUNT] = [
    "dot",               // Disc      — dot / point / mark / marker (mod FALLBACK_ICON) family
    "objective",         // Square    — objective(_marker) / obj / target / task family
    "point_of_interest", // Diamond   — point_of_interest / poi / intel / contact family
    "observation_post",  // Target    — observation_post / op / observe / overwatch / recon family
    "destroy",           // Ex        — destroy / demolish / demo / sabotage family
    "attack",   // TriangleUp — attack / assault / capture / seize / advance / ambush family
    "defend",   // TriangleDown — defend / hold / garrison / fallback family
    "waypoint", // Chevron   — waypoint / move / wp / route / phase_line family
    "flag",     // Flag      — flag / rally / rally_point / base / hq / spawn family
    "medical",  // Cross     — cross / medical / medic / aid / casevac / medevac family
    "circle",   // Ring      — circle / area / zone / ao family
];

/// The picker's row count: the number of canonical marker glyphs. Mirrors
/// `map_engine_render::scene::MARKER_GLYPH_COUNT` (the source of truth, a wasm32-only dep this native
/// const cannot reference directly); the wasm-side [`canonical_marker_rows`] asserts they agree.
/// The row count is lower than the alias count because aliases share glyphs.
pub(in crate::v2::apps::editor::ui::docks::dock_right) const CANONICAL_MARKER_GLYPH_COUNT: usize =
    11;

/// Compile-time tie between the mirrored count above and its source of truth. This fails the
/// `wasm32` build when the glyph set and picker count differ; the native
/// test cannot link `scene`, so THIS is what keeps [`CANONICAL_MARKER_GLYPH_COUNT`] honest. (The
/// per-slug glyph round-trip is a runtime `debug_assert` + self-healing fallback in
/// [`canonical_marker_rows`], since `marker_glyph_for_alias` is not a `const fn`.)
#[cfg(target_arch = "wasm32")]
const _: () = assert!(
    CANONICAL_MARKER_GLYPH_COUNT
        == website_map_engine::overlay::symbology::markers::MARKER_GLYPH_COUNT,
    "picker row count must equal scene::MARKER_GLYPH_COUNT (T-790 source of truth)"
);

/// A canonical picker row: the glyph to draw, the slug to STORE on pick (and show in the tooltip),
/// its human label, and every schema alias that folds into this family (the search-match set).
#[cfg(target_arch = "wasm32")]
pub(in crate::v2::apps::editor::ui::docks::dock_right) struct CanonicalMarkerRow {
    pub(super) glyph: website_map_engine::overlay::symbology::markers::MarkerGlyph,
    /// The canonical slug written to the document on pick — a closed-enum member.
    pub(super) slug: &'static str,
    /// `humanize_token(slug)`, the label that takes the row width.
    pub(super) label: String,
    /// Every `$defs/marker.icon` alias that folds to this glyph, for search + the tooltip.
    pub(super) aliases: Vec<&'static str>,
}

/// The canonical picker rows, built by folding the live schema alias list through the real
/// [`map_engine_render::scene::marker_glyph_for_alias`] so DISPLAY and MAP can never disagree.
///
/// Row order is schema-first-seen (the same browse order [`marker_icons`] documents). Each row's
/// `slug` is [`CANONICAL_MARKER_SLUGS`] chosen for that glyph, its `aliases` are every schema alias
/// that folds into it, and `filter` (empty ⇒ all) keeps a row when the human label, the slug, or any
/// alias substring-matches — so "Search icons" still matches slugs and names.
#[cfg(target_arch = "wasm32")]
pub(in crate::v2::apps::editor::ui::docks::dock_right) fn canonical_marker_rows(
    filter: &str,
) -> Vec<CanonicalMarkerRow> {
    use crate::v2::apps::editor::ui::inspector::zones_panel::humanize_token;
    use website_map_engine::overlay::symbology::markers::marker_glyph_for_alias;
    use website_map_engine::overlay::symbology::markers::MarkerGlyph;
    use website_map_engine::overlay::symbology::markers::MARKER_GLYPH_COUNT;

    debug_assert_eq!(
        CANONICAL_MARKER_GLYPH_COUNT, MARKER_GLYPH_COUNT,
        "picker row count must track scene::MARKER_GLYPH_COUNT"
    );

    let mut order: Vec<MarkerGlyph> = Vec::with_capacity(MARKER_GLYPH_COUNT);
    let mut aliases_by_glyph: Vec<(MarkerGlyph, Vec<&'static str>)> = Vec::new();
    for alias in marker_icons() {
        let g = marker_glyph_for_alias(alias);
        if let Some(slot) = aliases_by_glyph.iter_mut().find(|(gg, _)| *gg == g) {
            slot.1.push(alias.as_str());
        } else {
            order.push(g);
            aliases_by_glyph.push((g, vec![alias.as_str()]));
        }
    }

    let q = filter.trim().to_ascii_lowercase();
    order
        .into_iter()
        .map(|g| {
            let aliases = aliases_by_glyph
                .iter()
                .find(|(gg, _)| *gg == g)
                .map(|(_, a)| a.clone())
                .unwrap_or_default();
            let slug = *CANONICAL_MARKER_SLUGS
                .iter()
                .find(|s| marker_glyph_for_alias(s) == g)
                .unwrap_or_else(|| aliases.first().unwrap_or(&"dot"));
            CanonicalMarkerRow {
                glyph: g,
                slug,
                label: humanize_token(slug),
                aliases,
            }
        })
        .filter(|row| {
            q.is_empty()
                || row.label.to_ascii_lowercase().contains(&q)
                || row.slug.contains(&q)
                || row.slug.replace('_', " ").contains(&q)
                || row
                    .aliases
                    .iter()
                    .any(|a| a.contains(&q) || a.replace('_', " ").contains(&q))
        })
        .collect()
}

/// Draw a marker glyph preview using the same shape vocabulary as the map.
#[cfg(target_arch = "wasm32")]
pub(in crate::v2::apps::editor::ui::docks::dock_right) fn marker_glyph_svg(
    glyph: website_map_engine::overlay::symbology::markers::MarkerGlyph,
) -> AnyView {
    use website_map_engine::overlay::symbology::markers::MarkerGlyph;

    let inner = match glyph {
        MarkerGlyph::Ring => view! {
            <circle cx="8" cy="8" r="5.5" fill="none" stroke="currentColor" stroke-width="1.6" />
        }
        .into_any(),
        MarkerGlyph::Disc => view! {
            <circle cx="8" cy="8" r="4" fill="currentColor" />
        }
        .into_any(),
        MarkerGlyph::Square => view! {
            <rect x="3.5" y="3.5" width="9" height="9" fill="currentColor" />
        }
        .into_any(),
        MarkerGlyph::Diamond => view! {
            <polygon points="8,2.5 13.5,8 8,13.5 2.5,8" fill="currentColor" />
        }
        .into_any(),
        MarkerGlyph::TriangleUp => view! {
            <polygon points="8,2.5 14,13.5 2,13.5" fill="currentColor" />
        }
        .into_any(),
        MarkerGlyph::TriangleDown => view! {
            <polygon points="2,2.5 14,2.5 8,13.5" fill="currentColor" />
        }
        .into_any(),
        MarkerGlyph::Cross => view! {
            <path
                d="M6.5 2.5 h3 v4 h4 v3 h-4 v4 h-3 v-4 h-4 v-3 h4 z"
                fill="currentColor"
            />
        }
        .into_any(),
        MarkerGlyph::Ex => view! {
            <path
                d="M3.5 3.5 L12.5 12.5 M12.5 3.5 L3.5 12.5"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                fill="none"
            />
        }
        .into_any(),
        MarkerGlyph::Flag => view! {
            <path
                d="M4 2 v12"
                stroke="currentColor"
                stroke-width="1.4"
                stroke-linecap="round"
                fill="none"
            />
            <polygon points="4,2.5 13,4.5 4,7.5" fill="currentColor" />
        }
        .into_any(),
        MarkerGlyph::Chevron => view! {
            <path
                d="M3 10 L8 4 L13 10"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                fill="none"
            />
        }
        .into_any(),
        MarkerGlyph::Target => view! {
            <circle cx="8" cy="8" r="5.5" fill="none" stroke="currentColor" stroke-width="1.4" />
            <circle cx="8" cy="8" r="1.8" fill="currentColor" />
        }
        .into_any(),
    };

    view! {
        <svg
            class="block h-3.5 w-3.5 shrink-0"
            viewBox="0 0 16 16"
            aria-hidden="true"
            fill="none"
        >
            {inner}
        </svg>
    }
    .into_any()
}
