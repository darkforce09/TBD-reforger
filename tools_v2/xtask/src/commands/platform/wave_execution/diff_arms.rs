//! The verdict-diff ARMS — one scenario each.
//!
//! Split out of [`super::diff`] purely for size (SIZE-3); the driver, the comparator and the
//! anti-vacuity contract all live there and are documented there. Every function here builds a
//! state, runs both implementations over it, and hands the pair to `compare` together with the
//! expectation that proves the BASH side actually reached the state under test.

use std::path::Path;
use std::process::Command;

use super::diff::{
    ArmResult, Run, bash_side, compare, make_clone, normalise, rust_side, scratch,
    strip_parent_target_dir,
};
use super::{Ctx, host};

mod arm_noise_floor;
pub use arm_noise_floor::arm_base;
pub use arm_noise_floor::arm_lock;
pub use arm_noise_floor::arm_noise_floor;
pub use arm_noise_floor::arm_status;

mod arm_refusals;
pub use arm_refusals::arm_migrate_persist;
pub use arm_refusals::arm_push_guard;
pub use arm_refusals::arm_refusals;
pub use arm_refusals::first_line;
