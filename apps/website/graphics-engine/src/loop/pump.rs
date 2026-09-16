//! Role: pump.
//! Position: `loop` in the graphics engine.
//! Signals & state: the shared target cell, the disposal flag, a monotonic frame counter and
//! one optional per-frame hook.
//! Invariants: a frame renders, then polls, then counts, then calls the hook — in that order.
//! A contended target skips the frame and keeps the loop alive; it never panics and never
//! stops. Disposal stops the loop exactly once.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

/// What the pump drives, once per frame.
///
/// This is a **capability**, deliberately not a renderer type. `RenderEngine` lives in
/// `website-map-engine` and cannot move here: Rust's inherent-impl coherence (E0116) is
/// symmetric, so its ~124 `impl` blocks would have to come with it, and `#[wasm_bindgen]`
/// refuses trait impls, so its JS surface cannot be re-expressed as one. The engine therefore
/// implements this trait on its own side — the only direction both rules allow — and this
/// crate stays ignorant of the engine's name.
pub trait FrameTarget {
    /// Draw one frame.
    ///
    /// Returns nothing on purpose. A frame that fails is the target's to report; it must not
    /// stop the loop, which is what every call site this replaced spelled `let _ = e.render()`.
    fn render_frame(&mut self);

    /// Drain the device queue, after the frame has been submitted.
    ///
    /// Split from [`FrameTarget::render_frame`] so that the **order** — render, then poll — is
    /// the pump's single decision rather than a convention three call sites each had to
    /// remember. It is what lets a readback `map_async` callback land on the WebGL2-fallback
    /// and cull-counter paths.
    fn poll_device(&self);
}

/// The caller's per-frame hook: the target that has just rendered, and the running frame count.
type AfterFrame<T> = Box<dyn FnMut(&mut T, u32)>;

/// A self-rescheduling `requestAnimationFrame` loop over one [`FrameTarget`].
///
/// The target is shared (`Rc<RefCell<Option<T>>>`) because the browser reaches it from
/// elsewhere between frames — resize handlers, pointer handlers, self-check callbacks — and
/// because it is `None` until the async GPU boot finishes.
///
/// Everything except [`RafPump::start`] is DOM-free and compiles natively, which is what lets
/// the frame contract be unit-tested off-browser.
pub struct RafPump<T>
where
    T: 'static,
{
    target: Rc<RefCell<Option<T>>>,
    disposed: Arc<AtomicBool>,
    after_frame: Option<AfterFrame<T>>,
    frames: u32,
}

impl<T: FrameTarget + 'static> RafPump<T> {
    /// A pump over `target`, stopping once `disposed` is set.
    #[must_use]
    pub fn new(target: Rc<RefCell<Option<T>>>, disposed: Arc<AtomicBool>) -> Self {
        Self {
            target,
            disposed,
            after_frame: None,
            frames: 0,
        }
    }

    /// Run `hook(target, frames)` at the end of every rendered frame, inside the same borrow.
    ///
    /// `frames` is the running count *including* this frame. Callers that need a rate measure
    /// the difference between two samples of it; the count is monotonic (it wraps, and
    /// `wrapping_sub` keeps the difference right across the wrap) so the pump never has to
    /// know when a caller's sampling window opens or closes.
    ///
    /// Skipped frames — disposed, contended, or a target still `None` — do not call the hook
    /// and do not advance the count.
    #[must_use]
    pub fn after_frame(mut self, hook: impl FnMut(&mut T, u32) + 'static) -> Self {
        self.after_frame = Some(Box::new(hook));
        self
    }

    /// Frames rendered so far.
    #[must_use]
    pub fn frames(&self) -> u32 {
        self.frames
    }

    /// One frame's work, with no DOM in it: the disposal check, the contention-tolerant
    /// borrow, render → poll, the count, and the caller's hook.
    ///
    /// Returns `false` once `disposed` is set, which is the caller's signal to drop the loop
    /// closure; `true` otherwise, including for a frame that rendered nothing.
    ///
    /// **T-631 — the double-panic fix.** The borrow is `try_borrow_mut`, never `borrow_mut`.
    /// When a `render()` panicked (the observed `createBuffer size too large` → wasm
    /// `unreachable`), the abort re-entered the app while the first panic was unwinding; the
    /// next frame's `borrow_mut` then found the cell still held and panicked a SECOND time
    /// with "RefCell already borrowed", and that second panic — not the render failure — is
    /// what surfaced, burying the real cause. Making a contended frame a no-op means a
    /// re-entrant borrow can never overwrite the first, true panic. It is also the correct
    /// steady-state behaviour: a frame that cannot get the target simply waits for the next
    /// one rather than taking the tab down.
    pub fn tick(&mut self) -> bool {
        if self.disposed.load(Ordering::Relaxed) {
            return false;
        }
        let Ok(mut guard) = self.target.try_borrow_mut() else {
            // Contended: skip this frame, keep the loop alive.
            return true;
        };
        if let Some(target) = guard.as_mut() {
            target.render_frame();
            target.poll_device();
            self.frames = self.frames.wrapping_add(1);
            if let Some(hook) = self.after_frame.as_mut() {
                hook(target, self.frames);
            }
        }
        true
    }
}

#[cfg(target_arch = "wasm32")]
impl<T: FrameTarget + 'static> RafPump<T> {
    /// Schedule the first frame and keep scheduling until disposal.
    ///
    /// The closure owns the pump and owns a handle to its own slot, so the loop keeps itself
    /// alive for exactly as long as it runs and frees itself the moment it does not — no
    /// `forget()`, no leak.
    pub fn start(mut self) {
        use wasm_bindgen::prelude::Closure;

        let slot: RafSlot = Rc::new(RefCell::new(None));
        let handle = slot.clone();
        *handle.borrow_mut() = Some(Closure::wrap(Box::new(move || {
            if !self.tick() {
                slot.borrow_mut().take(); // drop the loop closure — no further frames
                return;
            }
            schedule(&slot);
        }) as Box<dyn FnMut()>));
        schedule(&handle);
    }
}

/// The self-referential rAF closure slot: the callback has to outlive the call that scheduled
/// it and has to be reachable from inside itself to schedule the next frame.
#[cfg(target_arch = "wasm32")]
type RafSlot = Rc<RefCell<Option<wasm_bindgen::prelude::Closure<dyn FnMut()>>>>;

/// Ask the browser for the next frame. A missing `window` (a worker, a torn-down document)
/// simply ends the loop.
#[cfg(target_arch = "wasm32")]
fn schedule(slot: &RafSlot) {
    use wasm_bindgen::JsCast;

    let cb_ref = slot.borrow();
    if let (Some(cb), Some(win)) = (cb_ref.as_ref(), web_sys::window()) {
        let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
    }
}

#[cfg(test)]
#[path = "tests/pump_tests.rs"]
mod tests;
