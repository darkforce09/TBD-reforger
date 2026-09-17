//! Mission row sections for the mission settings interface.

use super::*;

/// Renders mission briefing and thumbnail controls.
pub(super) fn render_presentation_section(
    ctrl: &'static str,
    shape: RwSignal<Option<RowShape>>,
) -> AnyView {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (ctrl, shape);
        return ().into_any();
    }
    #[cfg(target_arch = "wasm32")]
    {
        let sect = "text-label-sm uppercase tracking-wider text-outline";
        let hint = "text-label-sm normal-case text-outline";
        let mirror = ShapeMirror::from_route();

        let Some(row) = shape.get() else {
            return view! {
                <div class="mt-2 flex flex-col gap-2 border-t border-outline-variant/30 pt-4">
                    <span class=sect>"Presentation"</span>
                    <p class=hint>{PRESENTATION_UNAVAILABLE_NOTE}</p>
                </div>
            }
            .into_any();
        };

        let thumbnail = row.thumbnail_url.clone();
        let preview = (!thumbnail.trim().is_empty() && is_acceptable_thumbnail_url(&thumbnail))
            .then(|| {
                view! {
                    <img
                        src=thumbnail.trim().to_string()
                        alt="Mission thumbnail preview"
                        class="h-28 w-full rounded-md border border-outline-variant/20 object-cover"
                    />
                }
            });

        view! {
            <div class="mt-2 flex flex-col gap-4 border-t border-outline-variant/30 pt-4">
                <span class=sect>"Presentation"</span>
                <label class="flex flex-col gap-1">
                    <span class=sect>"Briefing"</span>
                    <textarea
                        rows="5"
                        placeholder="What is this operation, who is involved, and what does winning look like?"
                        prop:value=row.briefing.clone()
                        on:change=move |ev| {
                            mirror
                                .set_presentation(
                                    PresentationField::Briefing,
                                    event_target_value(&ev),
                                    shape,
                                );
                        }
                        class=format!("{ctrl} min-h-24 resize-y leading-relaxed")
                    ></textarea>
                </label>
                <span class=hint>{BRIEFING_NOTE}</span>

                <label class="flex flex-col gap-1">
                    <span class=sect>"Thumbnail link"</span>
                    <input
                        type="url"
                        placeholder="https://example.com/operation.jpg"
                        prop:value=row.thumbnail_url.clone()
                        on:change=move |ev| {
                            mirror
                                .set_presentation(
                                    PresentationField::Thumbnail,
                                    event_target_value(&ev),
                                    shape,
                                );
                        }
                        class=ctrl
                    />
                </label>
                <span class=hint>{THUMBNAIL_URL_NOTE}</span>
                {preview}
            </div>
        }
        .into_any()
    }
}

/// Renders the mission game mode and player figures.
pub(super) fn render_shape_section(
    ctrl: &'static str,
    shape: RwSignal<Option<RowShape>>,
) -> AnyView {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (ctrl, shape);
        return ().into_any();
    }
    #[cfg(target_arch = "wasm32")]
    {
        let sect = "text-label-sm uppercase tracking-wider text-outline";
        let hint = "text-label-sm normal-case text-outline";
        let readonly = "rounded-md border border-outline-variant/20 bg-surface-container-lowest/30 px-2.5 py-1.5 font-mono text-code-md text-on-surface-variant";
        let figure = "font-mono text-headline-sm text-on-surface";
        let mirror = ShapeMirror::from_route();

        let row = shape.get();
        let placed = match crate::v2::apps::editor::bridge::document_host::history::doc_handle() {
            Some(handle) => {
                let doc = handle.borrow();
                doc.as_ref().map_or(
                    0,
                    website_map_engine::data::store::MissionDocCore::slot_count,
                )
            }
            None => 0,
        };
        let counts = PlayerCount {
            placed,
            declared: row.as_ref().map(|r| r.max_players),
        };

        let mode_control = match row.as_ref() {
            Some(r) => {
                let current = r.game_mode.clone();
                let options = GAME_MODES
                    .into_iter()
                    .map(|(value, label)| view! { <option value=value>{label}</option> })
                    .collect::<Vec<_>>();
                view! {
                    <select
                        prop:value=current
                        on:change=move |ev| {
                            let v = event_target_value(&ev);
                            mirror.set_game_mode(v, shape);
                        }
                        class=ctrl
                    >
                        {options}
                    </select>
                }
                .into_any()
            }
            None => view! { <p class=hint>{SHAPE_UNAVAILABLE_NOTE}</p> }.into_any(),
        };

        let declared_block = counts.declared.map(|d| {
            let ruling = counts
                .disagrees()
                .then(|| view! { <span class=hint>{PLAYER_COUNT_RULING_NOTE}</span> });
            view! {
                <div class="flex flex-col gap-1 border-t border-outline-variant/20 pt-3">
                    <span class=sect>"Max players (declared at creation)"</span>
                    <div class=readonly>{d.to_string()}</div>
                    <span class=hint>{MAX_PLAYERS_KEPT_NOTE}</span>
                    {ruling}
                </div>
            }
        });

        view! {
            <div class="mt-2 flex flex-col gap-4 border-t border-outline-variant/30 pt-4">
                <span class=sect>"Mission shape"</span>
                <label class="flex flex-col gap-1">
                    <span class=sect>"Game mode"</span>
                    {mode_control}
                </label>

                <div class="flex flex-col gap-1">
                    <span class=sect>"Players"</span>
                    <div class=figure>{counts.player_figure().to_string()}</div>
                    <span class=hint>{SLOTS_PLACED_NOTE}</span>
                </div>

                {declared_block}
            </div>
        }
        .into_any()
    }
}
