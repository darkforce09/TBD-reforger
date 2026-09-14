//! The deployment card: where the viewer is slotted for the next operation.
//!
//! **Role:** renders the viewer's faction, squad and role, over a watermark icon.
//! **Position:** the middle cell of the dashboard's three-column card grid.
//! **Signals & state:** none — the assignment arrives owned and is read once.
//! **Invariants:** with no assignment the card keeps its heading and replaces the body with a
//! single line of empty-state text, so the grid keeps its three cells either way.
#![allow(dead_code)]

use super::helpers::vstr;
use crate::v2::core::ui::MaterialIcon;
use leptos::prelude::*;
use serde_json::Value;

/// The deployment card for `assignment`, the viewer's slot in the next operation.
///
/// Reads `faction`, `squad` and `role` from the assignment object.
pub(super) fn deployment(assignment: Option<Value>) -> impl IntoView {
    view! {
        <div class="relative flex flex-col overflow-hidden rounded-xl p-6 glass group gap-4">
            <div class="pointer-events-none absolute -right-10 -bottom-10 opacity-5 transition-opacity group-hover:opacity-10">
                <MaterialIcon name="military_tech" filled=true class="text-[200px]" />
            </div>
            <div class="relative z-10 flex items-center justify-between border-b border-border-subtle pb-3">
                <h3 class="flex items-center gap-2 text-label-sm text-on-surface-variant uppercase">
                    <MaterialIcon name="person" class="text-[18px]" />
                    "Deployment"
                </h3>
            </div>
            {match &assignment {
                Some(a) => {
                    view! {
                        <div class="relative z-10 mt-4 flex items-center gap-6">
                            <div class="flex gap-2">
                                <div class="flex h-12 w-12 items-center justify-center rounded-lg border border-border-subtle bg-surface-container-highest">
                                    <MaterialIcon
                                        name="swords"
                                        filled=true
                                        class="text-[28px] text-primary"
                                    />
                                </div>
                                <div class="flex h-12 w-12 items-center justify-center rounded-lg border border-border-subtle bg-surface-container-highest">
                                    <MaterialIcon
                                        name="security"
                                        filled=true
                                        class="text-[28px] text-primary"
                                    />
                                </div>
                            </div>
                            <div class="flex flex-col">
                                <span class="font-bold tracking-wide text-on-surface">
                                    {vstr(a, "faction")}
                                </span>
                                <span class="text-sm text-on-surface-variant">
                                    {vstr(a, "squad")}
                                </span>
                                <span class="mt-1 font-mono text-xs text-primary uppercase">
                                    {format!("Role: {}", vstr(a, "role"))}
                                </span>
                            </div>
                        </div>
                    }
                        .into_any()
                }
                None => {
                    view! {
                        <p class="relative z-10 mt-4 text-sm text-on-surface-variant">
                            "No active assignment"
                        </p>
                    }
                        .into_any()
                }
            }}
        </div>
    }
}
