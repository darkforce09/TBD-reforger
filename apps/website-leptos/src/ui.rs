//! Small UI helpers ported from lib/utils.ts (`cn`) + components/MaterialIcon.tsx.
use leptos::prelude::*;

/// Minimal class-string join (clsx-like): drop empties, space-join. NOTE: unlike the React `cn`
/// (clsx + tailwind-merge), this does NOT resolve Tailwind conflicts — the V gate proves the
/// shell's class combos have none; a twMerge-equivalent lands only if a conflicting combo appears.
pub fn cn(classes: &[&str]) -> String {
    classes.iter().filter(|c| !c.is_empty()).copied().collect::<Vec<_>>().join(" ")
}

/// Material Symbols icon — a font-glyph span whose text is the ligature name. Ported from
/// MaterialIcon.tsx (`<span class="material-symbols-outlined …">{name}</span>`).
#[component]
pub fn MaterialIcon(name: &'static str, #[prop(optional)] class: &'static str) -> impl IntoView {
    view! { <span class=cn(&["material-symbols-outlined", class])>{name}</span> }
}
