//! The burst height of a time-fuzed shell, and the fuze window it must fit.
//!
//! **Role:** decides whether the chosen shell takes a burst height, parses the typed height, and
//! renders the field with the shell's fuze window.
//! **Position:** a row of the inputs card shown only for a shell with a
//! [`TimeFuze`]; the parsed height goes into the save body's `burst_height_m` and the fuze
//! setting the solution reports.
//! **Signals & state:** the view reads the page's catalog and selection signals and reads and
//! writes its burst-height text signal.
//! **Invariants:** a shell without a time fuze never carries a burst height, whatever was typed;
//! an empty field is no burst height; a typed height is a finite number of metres above zero,
//! measured above the target.

use super::weapon_and_shell::{find_shell, ArmamentSelection};
use super::INPUT_CLASS;
use crate::v2::core::api::dto::ballistics_catalogs::{BallisticsCatalog, TimeFuze};
use leptos::prelude::*;
use std::sync::Arc;

/// Why a burst height does not parse.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum BurstHeightError {
    /// The text is not a finite number above zero.
    Invalid(String),
}

/// The sentence the page shows for a burst-height error.
pub(crate) fn burst_height_error_message(error: &BurstHeightError) -> String {
    match error {
        BurstHeightError::Invalid(text) => {
            format!("Burst height \"{text}\" must be a number of metres above zero.")
        }
    }
}

/// The time fuze of the selected shell, if it has one.
pub(crate) fn selected_time_fuze(
    catalog: &BallisticsCatalog,
    selection: &ArmamentSelection,
) -> Option<TimeFuze> {
    find_shell(catalog, &selection.shell_id).and_then(|shell| shell.time_fuze)
}

/// Parses the burst height for a shell whose time fuze is `fuze`: `Ok(None)` for a shell
/// without one, or for an empty field.
///
/// # Errors
///
/// [`BurstHeightError::Invalid`] for text that is not a finite number above zero.
pub(crate) fn parse_burst_height(
    text: &str,
    fuze: Option<&TimeFuze>,
) -> Result<Option<f64>, BurstHeightError> {
    let trimmed = text.trim();
    if fuze.is_none() || trimmed.is_empty() {
        return Ok(None);
    }
    trimmed
        .parse::<f64>()
        .ok()
        .filter(|h| h.is_finite() && *h > 0.0)
        .map(Some)
        .ok_or_else(|| BurstHeightError::Invalid(trimmed.to_string()))
}

/// The fuze window line, e.g. "Fuze 10–40 s, default 24 s".
pub(crate) fn fuze_window_text(fuze: &TimeFuze) -> String {
    format!(
        "Fuze {}–{} s, default {} s",
        trim_seconds(fuze.min_s),
        trim_seconds(fuze.max_s),
        trim_seconds(fuze.default_s)
    )
}

/// Seconds without a trailing `.0`, one decimal otherwise.
fn trim_seconds(seconds: f64) -> String {
    if seconds.fract() == 0.0 {
        format!("{seconds:.0}")
    } else {
        format!("{seconds:.1}")
    }
}

/// The burst-height field, rendered only while the selected shell has a time fuze.
pub(crate) fn illumination_inputs(
    catalog: RwSignal<Option<Arc<BallisticsCatalog>>>,
    selection: RwSignal<ArmamentSelection>,
    burst_height: RwSignal<String>,
) -> impl IntoView {
    let fuze = move || {
        catalog
            .get()
            .and_then(|c| selection.with(|s| selected_time_fuze(&c, s)))
    };
    move || {
        fuze().map(|fuze| {
            view! {
                <label class="text-sm" data-mortar-input="burst-height">
                    "Burst height above target (m)"
                    <input
                        type="number"
                        min="0"
                        step="1"
                        prop:value=move || burst_height.get()
                        on:input=move |ev| burst_height.set(event_target_value(&ev))
                        class=INPUT_CLASS
                    />
                    <span class="text-xs text-on-surface-variant">{fuze_window_text(&fuze)}</span>
                </label>
            }
        })
    }
}
