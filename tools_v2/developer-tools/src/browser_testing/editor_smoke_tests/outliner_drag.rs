//! Real editor handlers over a controlled hydrated mission. Coordinates come from the rendered
//! widget; assertions read the production compile payload and history. No copied gesture model.
use super::*;

const ID: &str = "94686000-0000-4000-a000-000000000001";
const LARGE_ID: &str = "94686000-0000-4000-a000-000000000003";
const DUP_ID: &str = "94686000-0000-4000-a000-000000000002";
const MIXED_ID: &str = "94686000-0000-4000-a000-000000000004";
const PAYLOAD: &str = "JSON.parse(window.__editorCommands.compile_save_json())";

// Keep CDP's held button consistent with its buttons mask. With button:none Chromium emits a
// gotpointercapture carrying buttons=0 and drops capture on the next move, before any real release.

// A release outside the tree and a cancellation must clear both pending representations.

#[path = "outliner_drag/mission.rs"]
mod mission;
use mission::canvas_hit;
use mission::click_at;
use mission::click_selector;
use mission::depth;
use mission::drag;
use mission::fixture_ready;
use mission::intercept;
use mission::payload;
use mission::row_drag;
use mission::same_positions;
use mission::select_five;
use mission::settle;
use mission::undo;
use mission::vehicle_point;
use mission::widget;
use mission::z_start;

#[path = "outliner_drag/vehicle_snap_cases.rs"]
mod vehicle_snap_cases;
use vehicle_snap_cases::mixed_orbat_cases;
use vehicle_snap_cases::orbat_cancel_cases;
use vehicle_snap_cases::orbat_cases;
use vehicle_snap_cases::outside_drop_cases;
use vehicle_snap_cases::vehicle_snap_cases;
use vehicle_snap_cases::z_lifecycle_cases;

#[path = "outliner_drag/execution.rs"]
mod execution;
pub(super) use execution::run;
