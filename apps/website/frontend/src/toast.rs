//! Toast notifications — the `sonner` replacement (T-159.25). React mounts `<Toaster theme="dark"
//! position="top-right" richColors />` once in main.tsx and pages fire `toast.success/error(msg)`;
//! here a `Toasts` context (provided at the AppLayout root) exposes the same two verbs and
//! `ToastViewport` renders the stack top-right.
//!
//! V-gate stability: the viewport renders **no DOM at all while empty** (sonner's idle `<section>`
//! markup is not replicated), so default-state DOM captures are unaffected by this module — toasts
//! only exist mid-interaction, which the V suite never captures.
// Fired only from wasm-gated handlers; the native view shell compiles the module for the viewport
// alone, so the verbs/fields read as dead there — the settings.rs file-level idiom.
#![allow(dead_code)]
use leptos::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub enum ToastKind {
    Success,
    Error,
    /// Neutral — sonner's `toast.message` (T-172, e.g. the modpacks Launch stub).
    Info,
}

#[derive(Clone)]
struct Toast {
    id: u64,
    kind: ToastKind,
    msg: String,
}

/// Auto-dismiss delay — sonner's default duration (4 s).
const TOAST_MS: f64 = 4000.0;

#[derive(Clone, Copy)]
pub struct Toasts {
    list: RwSignal<Vec<Toast>>,
    next_id: StoredValue<u64>,
}

impl Toasts {
    pub fn new() -> Self {
        Self {
            list: RwSignal::new(Vec::new()),
            next_id: StoredValue::new(0),
        }
    }

    pub fn success(&self, msg: impl Into<String>) {
        self.push(ToastKind::Success, msg.into());
    }

    pub fn error(&self, msg: impl Into<String>) {
        self.push(ToastKind::Error, msg.into());
    }

    /// Neutral notice — `toast.message(...)` parity.
    pub fn message(&self, msg: impl Into<String>) {
        self.push(ToastKind::Info, msg.into());
    }

    fn push(&self, kind: ToastKind, msg: String) {
        let id = self.next_id.with_value(|v| *v);
        self.next_id.set_value(id + 1);
        self.list.update(|l| l.push(Toast { id, kind, msg }));
        // Auto-dismiss (wasm only — the native view shell renders no timers).
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

/// Install the context at the shell root (peer of `AuthStore`).
pub fn provide_toasts() {
    provide_context(Toasts::new());
}

/// Page-side accessor — `use_toasts().success("…")`, the `toast.success('…')` call-site parity.
pub fn use_toasts() -> Toasts {
    expect_context::<Toasts>()
}

/// Top-right toast stack. Renders NOTHING while the list is empty (see module note).
#[component]
pub fn ToastViewport() -> impl IntoView {
    let toasts = use_toasts();
    move || {
        let items = toasts.list.get();
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
