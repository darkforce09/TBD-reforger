//! Mission row mirror for the mission settings interface.

use super::*;

#[derive(Default, Debug, Clone, PartialEq, Eq)]
/// Prevents row loads from replacing an active patch result.
pub(super) struct ShapeSeq {
    generation: u64,
    pub(super) patch_inflight: u32,
}

impl ShapeSeq {
    /// Provides begin load for mission settings.
    pub(super) fn begin_load(&self) -> u64 {
        self.generation
    }

    /// Provides may apply load for mission settings.
    pub(super) fn may_apply_load(&self, captured: u64) -> bool {
        self.patch_inflight == 0 && self.generation == captured
    }

    /// Provides begin patch for mission settings.
    pub(super) fn begin_patch(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.patch_inflight = self.patch_inflight.saturating_add(1);
    }

    /// Provides end patch for mission settings.
    pub(super) fn end_patch(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.patch_inflight = self.patch_inflight.saturating_sub(1);
    }
}

#[cfg(target_arch = "wasm32")]
thread_local! {
    static SHAPE_SEQ: std::cell::RefCell<ShapeSeq> =
        const { std::cell::RefCell::new(ShapeSeq { generation: 0, patch_inflight: 0 }) };
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy)]
/// Reads and updates the mission row for the settings dialog.
pub(super) struct ShapeMirror {
    auth: crate::v2::core::auth::AuthStore,
    mission_id: StoredValue<String>,
    toasts: crate::v2::core::ui::toast::Toasts,
}

#[cfg(target_arch = "wasm32")]
impl ShapeMirror {
    /// Provides from route for mission settings.
    pub(super) fn from_route() -> Self {
        use leptos_router::hooks::use_params_map;
        let id = use_params_map()
            .get_untracked()
            .get("id")
            .map(|s| s.to_string())
            .unwrap_or_default();
        Self {
            auth: expect_context::<crate::v2::core::auth::AuthStore>(),
            mission_id: StoredValue::new(id),
            toasts: crate::v2::core::ui::toast::use_toasts(),
        }
    }

    /// Provides load for mission settings.
    pub(super) fn load(self, shape: RwSignal<Option<RowShape>>) {
        let id = self.mission_id.get_value();
        if !crate::v2::apps::editor::ui::docks::top_strip::is_mission_row_id(&id) {
            shape.set(None);
            return;
        }
        // A review workspace shows the row fields its artifact compiled from, not today's row.
        if !crate::v2::apps::editor::shell::review_mode::writes_mission() {
            let reviewed = crate::v2::apps::editor::shell::document_commands::hydrated_row();
            shape.set(reviewed.map(RowShape::from));
            return;
        }
        if shape.get_untracked().is_none() {
            if let Some(h) = crate::v2::apps::editor::shell::document_commands::hydrated_row() {
                shape.set(Some(RowShape::from(h)));
            }
        }
        let captured = SHAPE_SEQ.with(|s| {
            let seq = s.borrow();
            if seq.patch_inflight > 0 {
                None
            } else {
                Some(seq.begin_load())
            }
        });
        let Some(captured) = captured else {
            return;
        };
        let auth = self.auth;
        leptos::task::spawn_local(async move {
            let got = crate::v2::core::api::client::api_get::<
                crate::v2::core::api::dto::MissionDetail,
            >(auth, &format!("/missions/{id}"))
            .await;
            let apply = SHAPE_SEQ.with(|s| s.borrow().may_apply_load(captured));
            if !apply {
                return;
            }
            match got {
                Ok(d) => {
                    let row = RowShape {
                        game_mode: d.game_mode,
                        max_players: d.max_players,
                        briefing: d.briefing.unwrap_or_default(),
                        thumbnail_url: d.thumbnail_url.unwrap_or_default(),
                    };
                    crate::v2::apps::editor::shell::document_commands::note_hydrated_row(
                        crate::v2::apps::editor::shell::document_commands::HydratedRow {
                            game_mode: row.game_mode.clone(),
                            max_players: row.max_players,
                            briefing: row.briefing.clone(),
                            thumbnail_url: row.thumbnail_url.clone(),
                        },
                    );
                    shape.set(Some(row));
                }
                Err(e) => {
                    leptos::logging::warn!(
                        "T-694: could not read the mission row's shape: {}",
                        crate::v2::core::api::client::api_error_message(
                            &e,
                            "GET /missions/:id failed"
                        )
                    );
                    shape.set(None);
                }
            }
        });
    }

    /// Provides set game mode for mission settings.
    pub(super) fn set_game_mode(self, next: String, shape: RwSignal<Option<RowShape>>) {
        let id = self.mission_id.get_value();
        let Some(previous) = shape.get_untracked() else {
            return;
        };
        if !crate::v2::apps::editor::ui::docks::top_strip::is_mission_row_id(&id)
            || !is_known_game_mode(&next)
            || previous.game_mode == next
        {
            return;
        }
        if !crate::v2::apps::editor::shell::review_mode::writes_mission() {
            self.toasts
                .error(crate::v2::apps::editor::shell::review_mode::saves_nothing_message());
            shape.set(None);
            shape.set(Some(previous));
            return;
        }
        SHAPE_SEQ.with(|s| s.borrow_mut().begin_patch());
        shape.set(Some(RowShape {
            game_mode: next.clone(),
            ..previous.clone()
        }));
        let auth = self.auth;
        let toasts = self.toasts;
        leptos::task::spawn_local(async move {
            let body = serde_json::json!({ "game_mode": next.clone() });
            let res = crate::v2::core::api::client::api_patch::<serde_json::Value>(
                auth,
                &format!("/missions/{id}"),
                body,
            )
            .await;
            match &res {
                Ok(_) => {
                    crate::v2::apps::editor::shell::document_commands::note_hydrated_game_mode(
                        &next,
                    )
                }
                Err(e) => {
                    leptos::logging::warn!(
                        "T-694: could not save the mission's game mode: {}",
                        crate::v2::core::api::client::api_error_message(
                            e,
                            "PATCH /missions/:id failed"
                        )
                    );
                    toasts.error(game_mode_failure_message(e));
                    shape.set(Some(previous));
                }
            }
            SHAPE_SEQ.with(|s| s.borrow_mut().end_patch());
        });
    }

    /// Provides set presentation for mission settings.
    pub(super) fn set_presentation(
        self,
        field: PresentationField,
        next: String,
        shape: RwSignal<Option<RowShape>>,
    ) {
        let id = self.mission_id.get_value();
        let Some(previous) = shape.get_untracked() else {
            return;
        };
        if !crate::v2::apps::editor::ui::docks::top_strip::is_mission_row_id(&id)
            || field.read(&previous) == next
        {
            return;
        }
        if !crate::v2::apps::editor::shell::review_mode::writes_mission() {
            self.toasts
                .error(crate::v2::apps::editor::shell::review_mode::saves_nothing_message());
            shape.set(None);
            shape.set(Some(previous));
            return;
        }
        if field == PresentationField::Thumbnail && !is_acceptable_thumbnail_url(&next) {
            self.toasts.error(THUMBNAIL_REJECTED_NOTE);
            shape.set(None);
            shape.set(Some(previous));
            return;
        }
        let next = if field == PresentationField::Thumbnail {
            next.trim().to_string()
        } else {
            next
        };
        let mut optimistic = previous.clone();
        field.write(&mut optimistic, next.clone());
        SHAPE_SEQ.with(|s| s.borrow_mut().begin_patch());
        shape.set(Some(optimistic));
        let auth = self.auth;
        let toasts = self.toasts;
        let column = field.column();
        leptos::task::spawn_local(async move {
            let mut body = serde_json::Map::new();
            body.insert(column.to_string(), serde_json::Value::String(next.clone()));
            let body = serde_json::Value::Object(body);
            let res = crate::v2::core::api::client::api_patch::<serde_json::Value>(
                auth,
                &format!("/missions/{id}"),
                body,
            )
            .await;
            match &res {
                Ok(_) => match field {
                    PresentationField::Briefing => {
                        mirror_briefing_into_document(&next);
                        crate::v2::apps::editor::shell::document_commands::note_hydrated_presentation(
                            Some(&next),
                            None,
                        );
                    }
                    PresentationField::Thumbnail => {
                        crate::v2::apps::editor::shell::document_commands::note_hydrated_presentation(
                            None,
                            Some(&next),
                        );
                    }
                },
                Err(e) => {
                    leptos::logging::warn!(
                        "T-671: could not save the mission's {column}: {}",
                        crate::v2::core::api::client::api_error_message(
                            e,
                            "PATCH /missions/:id failed"
                        )
                    );
                    toasts.error(presentation_failure_message(field, e));
                    shape.set(Some(previous));
                }
            }
            SHAPE_SEQ.with(|s| s.borrow_mut().end_patch());
        });
    }
}
