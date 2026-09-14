//! Transient notices: the toast stack, and the context pages raise them through.
//!
//! **Role:** owns the list of live notices, the three verbs that add one, and the viewport that
//! renders them.
//! **Position:** the context is provided once at the shell root; the viewport renders in the
//! top-right corner, above everything.
//! **Signals & state:** the context holds the notice list and the id counter. A notice removes
//! itself on a timer.
//! **Invariants:** the viewport renders no DOM at all while the list is empty, so a page with no
//! notices has no markup from this module in it — which keeps a captured default state free of it.
//! The dismissal timer exists only in the browser; the native build renders the viewport and no
//! timers.
// Notices are raised only from browser-gated handlers, so the verbs and fields read as unused on a
// native build.
#![allow(dead_code)]
use leptos::prelude::*;

/// What kind of notice this is, which picks its accent colour and glyph.
#[derive(Clone, Copy, PartialEq)]
pub enum ToastKind {
    /// Something worked.
    Success,
    /// Something failed.
    Error,
    /// A neutral statement of fact.
    Info,
}

/// One live notice.
#[derive(Clone)]
struct Toast {
    id: u64,
    kind: ToastKind,
    msg: String,
}

/// How long a notice stays up before it removes itself.
const TOAST_MS: f64 = 4000.0;

/// The notice list, as a context every page can raise a notice through.
///
/// `Copy`, so it threads through views without cloning.
#[derive(Clone, Copy)]
pub struct Toasts {
    list: RwSignal<Vec<Toast>>,
    next_id: StoredValue<u64>,
}

impl Toasts {
    /// An empty stack.
    pub fn new() -> Self {
        Self {
            list: RwSignal::new(Vec::new()),
            next_id: StoredValue::new(0),
        }
    }

    /// Raise a success notice.
    pub fn success(&self, msg: impl Into<String>) {
        self.push(ToastKind::Success, msg.into());
    }

    /// Raise a failure notice.
    pub fn error(&self, msg: impl Into<String>) {
        self.push(ToastKind::Error, msg.into());
    }

    /// Raise a neutral notice.
    pub fn message(&self, msg: impl Into<String>) {
        self.push(ToastKind::Info, msg.into());
    }

    fn push(&self, kind: ToastKind, msg: String) {
        let id = self.next_id.with_value(|v| *v);
        self.next_id.set_value(id + 1);
        self.list.update(|l| l.push(Toast { id, kind, msg }));
        // The dismissal timer exists only in the browser.
        #[cfg(target_arch = "wasm32")]
        {
            let list = self.list;
            set_timeout(
                move || list.update(|l| l.retain(|t| t.id != id)),
                std::time::Duration::from_millis(TOAST_MS as u64),
            );
        }
    }
}

/// Install the notice context. Called once, at the shell root, beside the session store.
pub fn provide_toasts() {
    provide_context(Toasts::new());
}

/// The notice context. Panics when called outside the shell, which is a wiring error.
pub fn use_toasts() -> Toasts {
    expect_context::<Toasts>()
}

/// The notice stack, in the top-right corner.
///
/// Renders one absolutely positioned column of notices, or nothing at all while the list is empty.
/// Mounted once by the shell, above every page.
#[component]
pub fn ToastViewport() -> impl IntoView {
    let toasts = use_toasts;
    move || {
        let items = toasts().list.get();
        (!items.is_empty()).then(|| {
            view! {
                <div
                    class="fixed right-4 top-4 z-[100] flex w-80 flex-col gap-2"
                    role="status"
                    aria-live="polite"
                >
                    {items
                        .into_iter()
                        .map(|t| {
                            let accent = match t.kind {
                                ToastKind::Success => "border-success/40 text-success",
                                ToastKind::Error => "border-error-alert/40 text-error-alert",
                                ToastKind::Info => "border-outline-variant/40 text-on-surface-variant",
                            };
                            let icon = match t.kind {
                                ToastKind::Success => "check_circle",
                                ToastKind::Error => "error",
                                ToastKind::Info => "info",
                            };
                            view! {
                                <div class=format!(
                                    "glass flex items-start gap-2 rounded-lg border px-4 py-3 text-sm shadow-lg {accent}",
                                )>
                                    <span class="material-symbols-outlined text-[18px] leading-5">
                                        {icon}
                                    </span>
                                    <span class="text-on-surface">{t.msg}</span>
                                </div>
                            }
                        })
                        .collect_view()}
                </div>
            }
        })
    }
}
