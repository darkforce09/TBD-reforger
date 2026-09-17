//! Transform.
pub use website_map_engine::data::store::operations::rotation::snap_value;
pub use website_map_engine::data::store::operations::rotation::ROTATE_LADDER_DEG;

/// Discrete translation grid sizes in metres; zero leaves translation free.
pub const TRANSLATE_LADDER_M: [f64; 4] = [0.0, 1.0, 5.0, 10.0];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// The active snap ladder selected by the transform widget.
pub enum Axis {
    Translate,
    Rotate,
}

impl Axis {
    #[must_use]
    /// Returns the ladder values for this axis.
    pub const fn ladder(self) -> &'static [f64] {
        match self {
            Axis::Translate => &TRANSLATE_LADDER_M,
            Axis::Rotate => &ROTATE_LADDER_DEG,
        }
    }
    #[must_use]
    /// Returns the displayed unit for this axis.
    pub const fn unit(self) -> &'static str {
        match self {
            Axis::Translate => "m",
            Axis::Rotate => "°",
        }
    }
}

#[must_use]
/// Moves a ladder index by one bounded step.
pub fn step(cur: usize, len: usize, delta: i32) -> usize {
    if len == 0 {
        return 0;
    }
    let last = len - 1;
    if delta > 0 {
        (cur + 1).min(last)
    } else if delta < 0 {
        cur.saturating_sub(1)
    } else {
        cur.min(last)
    }
}

#[must_use]
/// Rounds a translation to the selected grid rung.
pub fn snap_translate(value: f64, rung: usize) -> f64 {
    snap_value(value, *TRANSLATE_LADDER_M.get(rung).unwrap_or(&0.0))
}

#[must_use]
/// Formats a snap increment for the status readout.
pub fn fmt_step(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 {
        format!("{}", v.round() as i64)
    } else {
        let s = format!("{v:.2}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
/// The active transform widget mode.
pub enum WidgetVariant {
    None,
    #[default]
    Translate,
    Rotate,
}

impl WidgetVariant {
    #[must_use]
    /// Selects a widget mode from its keyboard digit.
    pub fn from_digit(self, digit: u8) -> Self {
        match digit {
            1 => WidgetVariant::None,
            2 => WidgetVariant::Translate,
            3 => WidgetVariant::Rotate,
            _ => self,
        }
    }
    #[must_use]
    /// Returns the keyboard digit for this widget mode.
    pub const fn to_digit(self) -> u8 {
        match self {
            WidgetVariant::None => 1,
            WidgetVariant::Translate => 2,
            WidgetVariant::Rotate => 3,
        }
    }
    #[must_use]
    /// Returns the displayed name of this widget mode.
    pub const fn label(self) -> &'static str {
        match self {
            WidgetVariant::None => "No Widget",
            WidgetVariant::Translate => "Translate",
            WidgetVariant::Rotate => "Rotate",
        }
    }
    #[must_use]
    /// Reports whether the widget rotates selections.
    pub const fn is_rotate(self) -> bool {
        matches!(self, WidgetVariant::Rotate)
    }
    #[must_use]
    /// Returns the snap ladder used by this widget mode.
    pub const fn snap_axis(self) -> Axis {
        match self {
            WidgetVariant::None | WidgetVariant::Translate => Axis::Translate,
            WidgetVariant::Rotate => Axis::Rotate,
        }
    }
}

/// Radius of the transform widget in screen pixels.
pub const WIDGET_RADIUS_PX: f64 = 42.0;
/// Screen pixel tolerance for a press on the rotation ring.
pub const RING_HIT_TOL_PX: f64 = 10.0;

#[must_use]
/// Tests whether a pointer press lands on the rotation ring.
pub fn press_on_ring(px: f64, py: f64, cx: f64, cy: f64) -> bool {
    let d = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt();
    (d - WIDGET_RADIUS_PX).abs() <= RING_HIT_TOL_PX
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Enablement and rung indexes for translation and rotation snapping.
pub struct SnapState {
    pub enabled: bool,
    pub translate_rung: usize,
    pub rotate_rung: usize,
}

impl Default for SnapState {
    fn default() -> Self {
        Self {
            enabled: false,
            translate_rung: 0,
            rotate_rung: 0,
        }
    }
}

impl SnapState {
    #[must_use]
    /// Returns the active translation rung, or free movement when disabled.
    pub fn effective_translate_rung(self) -> usize {
        if self.enabled {
            self.translate_rung
        } else {
            0
        }
    }
    #[must_use]
    /// Returns the active rotation rung, or free rotation when disabled.
    pub fn effective_rotate_rung(self) -> usize {
        if self.enabled {
            self.rotate_rung
        } else {
            0
        }
    }
    #[must_use]
    /// Returns the snap state with master enablement inverted.
    pub fn toggled(self) -> Self {
        Self {
            enabled: !self.enabled,
            ..self
        }
    }
    #[must_use]
    /// Moves a ladder index by one bounded step.
    pub fn stepped(self, axis: Axis, delta: i32) -> Self {
        match axis {
            Axis::Translate => Self {
                translate_rung: step(self.translate_rung, TRANSLATE_LADDER_M.len(), delta),
                ..self
            },
            Axis::Rotate => Self {
                rotate_rung: step(self.rotate_rung, ROTATE_LADDER_DEG.len(), delta),
                ..self
            },
        }
    }
    #[must_use]
    /// Returns the displayed label for an axis rung.
    pub fn rung_label(self, axis: Axis) -> String {
        let rung = match axis {
            Axis::Translate => self.effective_translate_rung(),
            Axis::Rotate => self.effective_rotate_rung(),
        };
        let step = axis.ladder().get(rung).copied().unwrap_or(0.0);
        if step <= 0.0 {
            "off".to_string()
        } else if axis == Axis::Rotate {
            format!("{}{}", fmt_step(step), axis.unit())
        } else {
            format!("{} {}", fmt_step(step), axis.unit())
        }
    }
    #[must_use]
    /// Formats the current snap mode for the toolbar.
    pub fn status_readout(self) -> String {
        if !self.enabled {
            return "SNAP  off".to_string();
        }
        format!(
            "SNAP  move {} \u{b7} rot {}",
            self.rung_label(Axis::Translate),
            self.rung_label(Axis::Rotate),
        )
    }
}
