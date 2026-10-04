//! The registered context-menu opener the canvas right-click gesture hands its request to.
//!
//! **Role:** carries one right click's facts — the viewport point, the entity under the cursor,
//! the selection at that moment and the world point — from the input layer to the context menu
//! the workspace renders, so the gesture opens the menu without naming the workspace's docks.
//! **Position:** part of the input layer. The workspace's context menu registers its opener here
//! (`ui::docks::context_menu::register_canvas_context_menu`), and the Mission Creator page runs
//! that registration first thing at mount, before the canvas mount installs the gestures; the
//! right-click handler in `pointer_gestures` calls `open_context_menu`.
//! **Signals & state:** one thread-local cell, `CONTEXT_MENU_OPENER`, holding the registered
//! opener; `None` until the first page mount registers it.
//! **Invariants:** a registration replaces the previous one, so a remount never stacks two
//! openers; the registered opener is a capture-free function, so a registration that outlives its
//! page is harmless. With nothing registered, a right click opens no menu: the gesture has
//! already suppressed the browser's own menu and finished any armed tactical draw.

use std::cell::RefCell;
use std::rc::Rc;

/// One right click over the canvas, as the gesture measured it.
#[derive(Clone, Debug, PartialEq)]
pub struct ContextMenuRequest {
    /// The click's viewport x, in CSS pixels.
    pub client_x: f64,
    /// The click's viewport y, in CSS pixels.
    pub client_y: f64,
    /// The slot or vehicle id under the cursor, `None` over empty ground.
    pub hit: Option<String>,
    /// The selected ids when the click landed.
    pub selection: Vec<String>,
    /// The world point under the cursor, `(x, z)` in metres.
    pub world: (f64, f64),
}

/// The registered context-menu opener.
pub type ContextMenuOpener = Rc<dyn Fn(ContextMenuRequest)>;

thread_local! {
    /// The opener the workspace's context menu registers. Thread-local because the menu it
    /// opens lives in a `!Send` page signal.
    static CONTEXT_MENU_OPENER: RefCell<Option<ContextMenuOpener>> = const { RefCell::new(None) };
}

/// Registers the context-menu opener, replacing any earlier registration.
pub fn register_context_menu_opener(opener: ContextMenuOpener) {
    CONTEXT_MENU_OPENER.with(|slot| *slot.borrow_mut() = Some(opener));
}

/// Hands one right click to the registered opener. Returns `true` when an opener ran and `false`
/// when none is registered, in which case no menu opens.
pub fn open_context_menu(request: ContextMenuRequest) -> bool {
    let opener = CONTEXT_MENU_OPENER.with(|slot| slot.borrow().clone());
    match opener {
        Some(opener) => {
            opener(request);
            true
        }
        None => false,
    }
}

#[cfg(test)]
#[path = "tests/context_menu_opener.rs"]
mod tests;
