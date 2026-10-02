//! Toolbar dispatch.
use super::*;

#[cfg(target_arch = "wasm32")]
type ToolbarDispatch = std::rc::Rc<EditorToolbarDispatch>;

#[cfg(target_arch = "wasm32")]
/// Callbacks and tracked getters exposed to the editor toolbar.
pub(crate) struct EditorToolbarDispatch {
    pub set_widget: Box<dyn Fn(u8)>,
    pub toggle_snap: Box<dyn Fn()>,
    pub snap_step: Box<dyn Fn(i32)>,
    pub select_all: Box<dyn Fn()>,
    pub widget_digit: Box<dyn Fn() -> u8>,
    pub widget_is_rotate: Box<dyn Fn() -> bool>,
    pub snap_enabled: Box<dyn Fn() -> bool>,
}

thread_local! {
    #[cfg(target_arch = "wasm32")]
    static TOOLBAR_DISPATCH: std::cell::RefCell<Option<ToolbarDispatch>> =
        const { std::cell::RefCell::new(None) };

    static TOOLBAR_DISPATCH_GEN: std::cell::RefCell<Option<ArcRwSignal<u32>>> =
        const { std::cell::RefCell::new(None) };
}

/// Returns the generation signal that invalidates toolbar button state.
pub(crate) fn toolbar_dispatch_generation() -> ArcRwSignal<u32> {
    TOOLBAR_DISPATCH_GEN.with(|c| {
        c.borrow_mut()
            .get_or_insert_with(|| ArcRwSignal::new(0))
            .clone()
    })
}

#[cfg(target_arch = "wasm32")]
fn bump_toolbar_dispatch_generation() {
    let gen = toolbar_dispatch_generation();
    gen.set(gen.get_untracked().wrapping_add(1));
}

#[cfg(target_arch = "wasm32")]
/// Installs the mounted editor toolbar callbacks and invalidates button state.
pub(crate) fn register_editor_toolbar_dispatch(d: ToolbarDispatch) {
    TOOLBAR_DISPATCH.with(|c| *c.borrow_mut() = Some(d));
    bump_toolbar_dispatch_generation();
}

#[cfg(target_arch = "wasm32")]
/// Clears toolbar callbacks when the editor route unmounts.
pub(crate) fn unregister_editor_toolbar_dispatch() {
    TOOLBAR_DISPATCH.with(|c| *c.borrow_mut() = None);
    bump_toolbar_dispatch_generation();
}

#[cfg(target_arch = "wasm32")]
/// Runs a toolbar command against the mounted editor dispatch.
pub(crate) fn with_editor_toolbar_dispatch(f: impl FnOnce(&EditorToolbarDispatch)) {
    TOOLBAR_DISPATCH.with(|c| {
        if let Some(d) = c.borrow().as_ref() {
            f(d);
        }
    });
}
