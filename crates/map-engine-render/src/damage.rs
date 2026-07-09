//! T-151.8 — damage-driven render skip policy (Class R).
//!
//! Pure decision module: no wgpu. The wasm [`crate::engine::RenderEngine`] owns the same
//! flags and applies this policy before acquiring a surface texture.

/// Frame submit decision after consulting dirty / continuous flags.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameDecision {
    /// Whether this `render()` call should acquire/encode/submit.
    pub submit: bool,
}

/// Damage + continuous-render policy for the map engine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderDamage {
    /// Set on any mutation that changes what would be drawn.
    pub dirty: bool,
    /// When true, every `render()` submits (HUD fps path).
    pub continuous: bool,
}

impl Default for RenderDamage {
    fn default() -> Self {
        Self {
            dirty: true,
            continuous: false,
        }
    }
}

impl RenderDamage {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mark(&mut self) {
        self.dirty = true;
    }

    pub fn set_continuous(&mut self, on: bool) {
        self.continuous = on;
        if on {
            self.dirty = true;
        }
    }

    /// Decide whether this frame should submit GPU work.
    #[must_use]
    pub fn begin_frame(&self) -> FrameDecision {
        FrameDecision {
            submit: self.dirty || self.continuous,
        }
    }

    /// After a successful submit: clear dirty unless continuous.
    pub fn after_submit(&mut self) {
        if !self.continuous {
            self.dirty = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_r_clean_second_frame_skips() {
        let mut d = RenderDamage::new();
        assert!(d.begin_frame().submit);
        d.after_submit();
        assert!(!d.dirty);
        assert!(!d.begin_frame().submit);
    }

    #[test]
    fn class_r_pan_marks_dirty() {
        let mut d = RenderDamage::new();
        d.after_submit();
        assert!(!d.begin_frame().submit);
        d.mark();
        assert!(d.begin_frame().submit);
    }

    #[test]
    fn class_r_continuous_always_submits() {
        let mut d = RenderDamage::new();
        d.set_continuous(true);
        d.after_submit();
        assert!(d.begin_frame().submit);
        d.after_submit();
        assert!(d.begin_frame().submit);
    }

    #[test]
    fn class_r_skip_leaves_dirty_false() {
        let d = RenderDamage {
            dirty: false,
            continuous: false,
        };
        assert!(!d.begin_frame().submit);
        assert!(!d.dirty);
    }
}
