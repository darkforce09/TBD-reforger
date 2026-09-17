//! Validation panel validation runtime.

use super::*;

/// Mounts the document change validation loop without rendering a card.
#[component]
pub fn ValidationPanel(doc_tick: RwSignal<u64>) -> impl IntoView {
    let findings = RwSignal::new(Vec::<PanelFinding>::new());
    let rechecking = RwSignal::new(false);

    register_panel_sink(findings);

    #[cfg(target_arch = "wasm32")]
    {
        use std::cell::RefCell;
        use std::rc::Rc;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        let disposed = Arc::new(AtomicBool::new(false));
        let deb = Rc::new(RefCell::new(Debouncer::new(REEVAL_DEBOUNCE_MS)));
        let timer: Rc<RefCell<Option<leptos::leptos_dom::helpers::TimeoutHandle>>> =
            Rc::new(RefCell::new(None));

        let run_eval = {
            let disposed = disposed.clone();
            move || {
                if disposed.load(Ordering::Relaxed) {
                    return;
                }
                findings.set(evaluate_now());
                rechecking.set(false);
            }
        };

        let arm = {
            let deb = deb.clone();
            let timer = timer.clone();
            let run_eval = run_eval.clone();
            let disposed = disposed.clone();
            Rc::new(move || {
                if disposed.load(Ordering::Relaxed) {
                    return;
                }
                if let Some(h) = timer.borrow_mut().take() {
                    h.clear();
                }
                let deb2 = deb.clone();
                let timer2 = timer.clone();
                let run_eval = run_eval.clone();
                let handle = set_timeout_with_handle(
                    move || {
                        timer2.borrow_mut().take();
                        let now = now_ms();
                        let fire = {
                            let mut d = deb2.borrow_mut();
                            if d.should_fire(now) {
                                d.take_fire()
                            } else {
                                false
                            }
                        };
                        if fire {
                            run_eval(); // itself guarded by `disposed`
                        }
                    },
                    std::time::Duration::from_millis(REEVAL_DEBOUNCE_MS as u64),
                );
                if let Ok(h) = handle {}
            })
        };

        {
            let run_eval = run_eval.clone();
            let disposed = disposed.clone();
            let attempts = Rc::new(std::cell::Cell::new(0u32));
            let seed: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(None));
            let seed_run: Rc<dyn Fn()> = {
                let seed = seed.clone();
                Rc::new(move || {
                    if disposed.load(Ordering::Relaxed) {
                        return;
                    }
                    run_eval();
                    let ready = read_payload_source().is_some();
                    let n = attempts.get() + 1;
                    attempts.set(n);
                    if !ready && n < INITIAL_EVAL_MAX_TICKS {
                        if let Some(next) = seed.borrow().as_ref().cloned() {
                            set_timeout(
                                move || next(),
                                std::time::Duration::from_millis(INITIAL_EVAL_TICK_MS),
                            );
                        }
                    } else {
                        seed.borrow_mut().take();
                    }
                })
            };
            set_timeout(move || seed_run(), std::time::Duration::from_millis(0));
        }

        {
            let deb = deb.clone();
            let arm = arm.clone();
            Effect::new(move |_| {
                let _ = doc_tick.get(); // subscribe — re-run on every doc change
                deb.borrow_mut().bump(now_ms());
                rechecking.set(true);
                arm();
            });
        }

        {
            let disposed = disposed.clone();
            on_cleanup(move || disposed.store(true, Ordering::Relaxed));
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (doc_tick, rechecking);
    }
}
#[cfg(target_arch = "wasm32")]
#[must_use]
fn now_ms() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map_or(0.0, |p| p.now())
}

#[cfg(not(target_arch = "wasm32"))]
#[must_use]
fn now_ms() -> f64 {
    0.0
}
