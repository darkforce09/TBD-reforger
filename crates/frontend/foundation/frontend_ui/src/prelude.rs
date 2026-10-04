//! The primitives and helpers most views reach for, for `use frontend_ui::prelude::*;`.
//!
//! **Role:** re-exports the components every page composes, the class joiner, the toast context
//! and the avatar sanitiser.
//! **Position:** a re-export list over the crate's own modules.
//! **Signals & state:** none.
//! **Invariants:** re-exports only; every item keeps its home module.

pub use crate::badge::badge_class;
pub use crate::dialog::Dialog;
pub use crate::icons::{DEFAULT_AVATAR, MaterialIcon};
pub use crate::page_header::{PageHeader, cn};
pub use crate::sanitize::safe_avatar_url;
pub use crate::search_box::SearchBox;
pub use crate::select::Select;
pub use crate::sheet::Sheet;
pub use crate::slider::Slider;
pub use crate::toast::Toasts;
