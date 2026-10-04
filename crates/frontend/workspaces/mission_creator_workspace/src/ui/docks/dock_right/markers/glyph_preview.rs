//! The marker glyph preview the markers panel draws beside each canonical picker row.

use leptos::prelude::*;

/// Draw a marker glyph preview using the same shape vocabulary as the map.
pub(in crate::ui::docks::dock_right) fn marker_glyph_svg(
    glyph: unit_symbology::markers::MarkerGlyph,
) -> AnyView {
    use unit_symbology::markers::MarkerGlyph;

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
