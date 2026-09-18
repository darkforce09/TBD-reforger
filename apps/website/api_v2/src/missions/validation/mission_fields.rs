//! Write-boundary predicates for the scalar columns of a `missions` row: the display strings, the
//! three Postgres ENUMs, the clock, and the numeric range filter the library list accepts.
//!
//! CREATE and PATCH both go through these, so the two writers cannot hold different accept sets.

use crate::core::error_handling::api_error::ApiError;
use crate::core::text::http_url_guard::is_http_url;
use crate::missions::models::mission::{GameMode, TerrainType, WeatherType};

/// `missions.thumbnail_url`, validated at the write boundary.
///
/// CREATE hardcodes `thumbnail_url` to `''` and accepts no body field for it — PATCH is the only
/// HTTP writer. The sink is an `<img src>` in the frontend mission library, so an unvalidated
/// scheme here is a script-URL hole in a page every member loads.
pub(crate) fn validated_thumbnail_url(raw: &str) -> Result<String, ApiError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || is_http_url(trimmed) {
        return Ok(trimmed.to_string());
    }
    Err(ApiError::bad_request(
        "thumbnail_url must be an absolute http:// or https:// URL",
    ))
}

/// Mission `title` write guard.
///
/// There is no CHECK or trigger on `missions.title`, and a blank title sorts first in the in-game
/// mission browser. A bare `is_empty()` would let whitespace-only through, and an unguarded PATCH
/// would let `""` clobber a real title. Trim once, reject empty — one accept set for both writers.
/// Stores the trimmed form; repair is intentional here, because the column is display text rather
/// than a join key.
pub(crate) fn validated_mission_title(raw: &str) -> Result<String, ApiError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(ApiError::bad_request("title is required"));
    }
    Ok(trimmed.to_string())
}

pub(crate) fn valid_terrain(s: &str) -> Option<TerrainType> {
    match s {
        "everon" => Some(TerrainType::Everon),
        "arland" => Some(TerrainType::Arland),
        "custom" => Some(TerrainType::Custom),
        _ => None,
    }
}

pub(crate) fn valid_game_mode(s: &str) -> Option<GameMode> {
    match s {
        "pve_coop" => Some(GameMode::PveCoop),
        "pvp" => Some(GameMode::Pvp),
        "zeus" => Some(GameMode::Zeus),
        _ => None,
    }
}

/// Weather enum parse for CREATE / PATCH. Blank is **not** Clear.
///
/// A `"" | "clear"` arm would coerce blank → `Clear`, so PATCH `{"weather":""}` would silently
/// rewrite a stored `dense_fog` to `clear` and answer 200. The compile/flatten path
/// (`flatten::apply_authored_environment` / `WEATHER_PRESETS`) treats `""` as not-authored and
/// keeps the row — the two halves would disagree. Keeping `""` out of the clear arm keeps PATCH
/// rejecting blank (400 `invalid weather`) and compile falling through to the row.
///
/// CREATE is different: `CreateMissionInput.weather` is `#[serde(default)] String`, so an omitted
/// JSON field deserializes as `""`. `create_mission` defaults that empty to `WeatherType::Clear`
/// **before** calling this helper — the new-mission canonical default. This function itself must
/// still return `None` for `""` so PATCH never silently rewrites.
pub(crate) fn valid_weather(s: &str) -> Option<WeatherType> {
    match s {
        "clear" => Some(WeatherType::Clear),
        "overcast" => Some(WeatherType::Overcast),
        "heavy_rain" => Some(WeatherType::HeavyRain),
        "dense_fog" => Some(WeatherType::DenseFog),
        _ => None,
    }
}

/// `HH:MM` or `HH:MM:SS` → the same string, bound verbatim to the `missions.time_of_day` `time`
/// column; `None` when it is not a clock this platform can round-trip.
///
/// ── Why this exists ─────────────────────────────────────────────────────────────────────────
/// Without a validator of its own, `time_of_day` reaches `$N::time` and Postgres does the
/// validating, so its rejection surfaces as **HTTP 500 `{"error":"internal error"}`**. Driven on
/// the live path: POST `"   "` / `"not-a-time"` / `"\t"` / `"25:00"` → 500 (an untrimmed
/// `is_empty()` guard lets whitespace walk straight through), and a PATCH with no guard at all
/// 500s on `""` too. A caller cannot tell any of those from a genuine server fault.
///
/// ── Why it is NARROWER than the column, deliberately ────────────────────────────────────────
/// Measured against Postgres 18 directly: `time` also accepts `24:00`, `0600`, `4:05 PM`, `allballs`,
/// `06:00:00.5` and `06:00:60` (a leap second, silently normalised to `06:01:00`). Every one of those
/// would store fine and then be unreadable to the editor: the SPA's clock parser
/// (`eden_chrome::hhmm_to_minutes`) takes `HH:MM`/`HH:MM:SS` with `h <= 23`, `m <= 59`, `sec <= 59`,
/// and a value that parser cannot read parks the time-of-day scrubber at the 06:00 default **in
/// silence** — an author who sets 21:45 sees 06:00 after a reload. So "what the column accepts" is
/// the wrong bar; the right one is "what the platform can round-trip", and this mirrors
/// `hhmm_to_minutes` exactly so the two boundaries agree: the bug class is DISAGREEMENT between two
/// sites, not either site's rule. It is stricter in one place only — every component must be ASCII
/// digits, because Rust's `u32::from_str` accepts a leading `+` (`"+6:00"` would parse here and then
/// be rejected by Postgres, which is the 500 all over again).
///
/// Blast radius measured before tightening: all **87** live `missions` rows are plain `HH:MM:SS` with
/// zero sub-second components, and every producer that goes through this API emits `HH:MM` (the
/// create dialog, `RowMirror::set_time` via `normalize_clock`) or `HH:MM:SS` (the row hydrate
/// round-trip). The committed seeds `INSERT` directly and never touch this path. Nothing live is
/// rejected.
///
/// Returns the input UNCHANGED rather than a canonical form: this layer stores the author's bytes
/// verbatim, and normalising one side of a column two sites write is how they come apart. This
/// REJECTS; it does not repair.
pub(crate) fn valid_time_of_day(s: &str) -> Option<&str> {
    let mut parts = s.split(':');
    let h: u32 = digits(parts.next()?)?;
    let m: u32 = digits(parts.next()?)?;
    if let Some(sec) = parts.next()
        && digits(sec)? > 59
    {
        return None;
    }
    if parts.next().is_some() || h > 23 || m > 59 {
        return None;
    }
    Some(s)
}

/// An inclusive `lo-hi` integer range from a library list filter, or `None` when it is not one.
pub(crate) fn parse_range(s: &str) -> Option<(i64, i64)> {
    let (lo, hi) = s.split_once('-')?;
    let lo: i64 = lo.trim().parse().ok()?;
    let hi: i64 = hi.trim().parse().ok()?;
    (lo <= hi).then_some((lo, hi))
}

/// One `HH`/`MM`/`SS` component: non-empty ASCII digits only.
///
/// ASCII-only is the point, and it is stricter than `u32::from_str`, which accepts a leading `+`;
/// [`valid_time_of_day`] needs that strictness.
fn digits(part: &str) -> Option<u32> {
    if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    part.parse().ok()
}

#[cfg(test)]
#[path = "tests/mission_fields.rs"]
mod tests;
