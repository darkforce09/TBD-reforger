//! The surface wind: a speed and the compass direction it blows from.
//!
//! **Role:** the wind draft, its parse into the fire-mission [`FireMissionWind`] (or calm air),
//! and the two fields that edit it.
//! **Position:** a row of the inputs card; the solve bridge hands the parsed wind to the map
//! engine's fire-mission solve, and the save body carries it unchanged.
//! **Signals & state:** the view reads and writes the page's wind draft signal.
//! **Invariants:** the wind is constant with height and reported the meteorological way (the
//! direction it blows *from*, degrees clockwise from north); both fields empty, or a zero speed,
//! is calm air and yields `None`; a direction is normalised into `[0, 360)`; a negative or
//! non-numeric speed, a non-numeric direction, or a speed without a direction is refused.
//!
//! [`FireMissionWind`]: fire_mission_planning::fire_mission::FireMissionWind

#[cfg(target_arch = "wasm32")]
use super::INPUT_CLASS;
#[cfg(any(target_arch = "wasm32", test))]
use fire_mission_planning::fire_mission::FireMissionWind;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

/// The wind as typed.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct WindDraft {
    /// Speed text, metres per second.
    pub(crate) speed_m_s: String,
    /// Direction-from text, degrees clockwise from north.
    pub(crate) from_deg: String,
}

/// Why a wind draft does not parse.
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum WindInputError {
    /// The speed is not a finite number of zero or more.
    InvalidSpeed(String),
    /// The direction is not a finite number of degrees.
    InvalidDirection(String),
    /// A speed above zero came without a direction.
    DirectionMissing,
    /// A direction came without a speed.
    SpeedMissing,
}

/// The sentence the page shows for a wind error.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn wind_error_message(error: &WindInputError) -> String {
    match error {
        WindInputError::InvalidSpeed(text) => {
            format!("Wind speed \"{text}\" must be a number of metres per second, zero or more.")
        }
        WindInputError::InvalidDirection(text) => {
            format!("Wind direction \"{text}\" must be a number of degrees.")
        }
        WindInputError::DirectionMissing => {
            "Enter the direction the wind blows from, in degrees.".to_string()
        }
        WindInputError::SpeedMissing => "Enter the wind speed in metres per second.".to_string(),
    }
}

/// Parses a wind draft: `Ok(None)` is calm air.
///
/// # Errors
///
/// A [`WindInputError`] for an unreadable or incomplete report.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn parse_wind(draft: &WindDraft) -> Result<Option<FireMissionWind>, WindInputError> {
    let speed_text = draft.speed_m_s.trim();
    let from_text = draft.from_deg.trim();
    let from_deg = if from_text.is_empty() {
        None
    } else {
        Some(
            from_text
                .parse::<f64>()
                .ok()
                .filter(|d| d.is_finite())
                .ok_or_else(|| WindInputError::InvalidDirection(from_text.to_string()))?,
        )
    };
    if speed_text.is_empty() {
        return match from_deg {
            Some(_) => Err(WindInputError::SpeedMissing),
            None => Ok(None),
        };
    }
    let speed_m_s = speed_text
        .parse::<f64>()
        .ok()
        .filter(|s| s.is_finite() && *s >= 0.0)
        .ok_or_else(|| WindInputError::InvalidSpeed(speed_text.to_string()))?;
    if speed_m_s == 0.0 {
        return Ok(None);
    }
    let from_deg = from_deg.ok_or(WindInputError::DirectionMissing)?;
    Ok(Some(FireMissionWind {
        speed_m_s,
        from_deg: from_deg.rem_euclid(360.0),
    }))
}

/// The speed and direction fields.
#[cfg(target_arch = "wasm32")]
pub(crate) fn wind_inputs(draft: RwSignal<WindDraft>) -> impl IntoView {
    view! {
        <label class="text-sm">
            "Wind speed (m/s)"
            <input
                type="number"
                min="0"
                step="0.1"
                placeholder="0 = calm"
                prop:value=move || draft.with(|d| d.speed_m_s.clone())
                on:input=move |ev| draft.update(|d| d.speed_m_s = event_target_value(&ev))
                class=INPUT_CLASS
                data-mortar-input="wind-speed"
            />
        </label>
        <label class="text-sm">
            "Wind from (° from north)"
            <input
                type="number"
                step="1"
                placeholder="e.g. 270 = from the west"
                prop:value=move || draft.with(|d| d.from_deg.clone())
                on:input=move |ev| draft.update(|d| d.from_deg = event_target_value(&ev))
                class=INPUT_CLASS
                data-mortar-input="wind-from"
            />
        </label>
    }
}
