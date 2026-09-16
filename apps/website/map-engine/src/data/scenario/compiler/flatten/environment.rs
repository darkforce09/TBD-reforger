//! Role: environment.
//! Position: `mission/compiler/flatten` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::MissionMeta;

/// `""` is deliberately absent: `PATCH /missions/{id}` reads the empty string as "clear", but here an absent value must mean **not authored** so the mission row wins the fallback below.
pub(super) const WEATHER_PRESETS: [&str; 4] = ["clear", "overcast", "heavy_rain", "dense_fog"];

/// Wrong types and out-of-range values become `None` rather than a compile failure: stored payloads are immutable, and a bad `fog` must not 500 `GET /missions/:id/compiled`. The schema ranges are the gates; `0` is a real authored value for fog/wind/windDirDeg, so presence is `Some`, never a truthiness test.
pub(super) struct EnvironmentAxes {
    /// Wind dir deg.
    pub(super) wind_dir_deg: Option<f64>,
    /// Fog.
    pub(super) fog: Option<f64>,
    /// Wind.
    pub(super) wind: Option<f64>,
    /// View distance.
    pub(super) view_distance: Option<f64>,
}

impl EnvironmentAxes {
    /// From payload bag using the supplied domain data.
    pub(super) fn from_payload_bag(env: &serde_json::Value) -> Self {
        Self {
            wind_dir_deg: env_opt_closed(env, "windDirDeg", 0.0, 360.0),
            fog: env_opt_closed(env, "fog", 0.0, 1.0),
            wind: env_opt_min(env, "wind", 0.0),
            view_distance: env_opt_exclusive_min(env, "viewDistance", 0.0),
        }
    }

    /// Any on wire using the supplied domain data.
    pub(super) fn any_on_wire(&self) -> bool {
        self.wind_dir_deg.is_some()
            || self.fog.is_some()
            || self.wind.is_some()
            || self.view_distance.is_some()
    }
}

/// Env opt f64 using the supplied domain data.
pub(super) fn env_opt_f64(env: &serde_json::Value, key: &str) -> Option<f64> {
    let n = env.get(key)?.as_f64()?;
    if n.is_finite() { Some(n) } else { None }
}

/// Env opt closed using the supplied domain data.
pub(super) fn env_opt_closed(
    env: &serde_json::Value,
    key: &str,
    min: f64,
    max: f64,
) -> Option<f64> {
    let n = env_opt_f64(env, key)?;
    if n < min || n > max { None } else { Some(n) }
}

/// Env opt min using the supplied domain data.
pub(super) fn env_opt_min(env: &serde_json::Value, key: &str, min: f64) -> Option<f64> {
    let n = env_opt_f64(env, key)?;
    if n < min { None } else { Some(n) }
}

/// Env opt exclusive min using the supplied domain data.
pub(super) fn env_opt_exclusive_min(env: &serde_json::Value, key: &str, min: f64) -> Option<f64> {
    let n = env_opt_f64(env, key)?;
    if n <= min { None } else { Some(n) }
}

/// The Mission Settings dialog and the top-strip scrubber author `meta.environment.{time,weather}` into the editor document, and `mission::compile::compile_payload` carries them out to the saved payload's top-level `environment`. Reading the mission ROW alone therefore hands the game server the values the mission was *created* with while the author is looking at the ones they set.
pub fn apply_authored_environment(meta: &mut MissionMeta, payload: &[u8]) {
    #[derive(serde::Deserialize)]
    struct PayloadEnvelope {
        #[serde(default)]
        environment: serde_json::Value,
    }

    let Ok(envelope) = serde_json::from_slice::<PayloadEnvelope>(payload) else {
        return;
    };
    let env = envelope.environment;
    if let Some(t) = env
        .get("time")
        .and_then(serde_json::Value::as_str)
        .and_then(clock_hhmm)
    {
        meta.time_of_day = t;
    }
    if let Some(w) = env
        .get("weather")
        .and_then(serde_json::Value::as_str)
        .filter(|w| WEATHER_PRESETS.contains(w))
    {
        meta.weather_preset = w.to_string();
    }
}

/// `HH:MM` / `HH:MM:SS` → canonical `HH:MM`; anything else → `None`.
pub(super) fn clock_hhmm(s: &str) -> Option<String> {
    let mut parts = s.split(':');
    let h: u32 = parts.next()?.parse().ok()?;
    let m: u32 = parts.next()?.parse().ok()?;
    if let Some(sec) = parts.next() {
        let sec: u32 = sec.parse().ok()?;
        if sec > 59 {
            return None;
        }
    }
    if parts.next().is_some() || h > 23 || m > 59 {
        return None;
    }
    Some(format!("{h:02}:{m:02}"))
}
