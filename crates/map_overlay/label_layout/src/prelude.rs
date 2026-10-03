//! The names a caller of the label layout imports with `use label_layout::prelude::*;`.

pub use crate::declutter::{LabelSpec, declutter};
pub use crate::importance::{LocationLabel, declutter_town_labels};
pub use crate::label_ids::{LabelId, LocationId};
pub use crate::text_packing::pack_label_glyphs;
