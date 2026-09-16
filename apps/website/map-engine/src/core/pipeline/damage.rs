//! Role: damage.
//! Position: `core/pipeline` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

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
    /// New.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark.
    pub fn mark(&mut self) {
        self.dirty = true;
    }

    /// Set continuous.
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
#[path = "tests/damage_tests.rs"]
mod tests;
