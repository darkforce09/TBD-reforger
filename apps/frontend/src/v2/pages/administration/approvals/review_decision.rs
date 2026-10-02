//! The decision on the artifact under review: approve, approve with conditions, or reject.
//!
//! **Role:** the three decisions, the text each one needs, the request body each one sends, and the
//! form that sends it — the decision picker, the conditions or reason box, and the send control.
//! **Position:** the sticky foot of the approvals drawer, for a mission with a review under way.
//! **Signals & state:** owns the chosen decision, its text, the in-flight flag and the problem last
//! reported; writes the screen's notice and reads the queue again through the shared desk.
//! **Invariants:** every decision names the artifact of the pending review it was made on. A
//! conditional approval needs its conditions and a rejection its reason — trimmed, not blank, at
//! most 8000 bytes — and neither is sent without them. A refusal that means the queue is stale reads
//! it again and leaves its sentence on the desk, where the rebuilt drawer shows it. Sending is
//! browser-only.

use super::page::ApprovalsDesk;
use crate::v2::core::api::dto::{ApprovalDecision, RejectionDecision};
use crate::v2::core::ui::cn;
use crate::v2::pages::mission_hub::mission_review::review_wording::validated_review_text;
use leptos::prelude::*;

/// The decisions a reviewer can make on the artifact under review.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DecisionKind {
    Approve,
    ApproveWithConditions,
    Reject,
}

impl DecisionKind {
    /// Every decision, in the order the form offers them.
    pub(super) const ALL: [Self; 3] = [Self::Approve, Self::ApproveWithConditions, Self::Reject];

    /// The decision's name on its picker.
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Approve => "Approve",
            Self::ApproveWithConditions => "Approve with conditions",
            Self::Reject => "Reject",
        }
    }

    /// What the text box asks for under this decision, when it asks for anything.
    pub(super) fn text_label(self) -> Option<&'static str> {
        match self {
            Self::Approve => None,
            Self::ApproveWithConditions => {
                Some("Conditions — what the approval holds the mission to")
            }
            Self::Reject => Some("Reason — what the author must fix; it is all they are told"),
        }
    }

    /// What the reviewer is told once the decision is recorded.
    pub(super) fn recorded(self) -> &'static str {
        match self {
            Self::Approve => "Approved — the mission is live and deployments run this artifact",
            Self::ApproveWithConditions => {
                "Approved with conditions — the mission is live and its author sees the conditions"
            }
            Self::Reject => "Rejected — the mission is back with its author, with your reason",
        }
    }
}

/// A decision ready to send.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum DecisionBody {
    Approve(ApprovalDecision),
    Reject(RejectionDecision),
}

/// The body a decision sends about `artifact_id`, or what is wrong with its text.
pub(super) fn decision_body(
    kind: DecisionKind,
    artifact_id: &str,
    text: &str,
) -> Result<DecisionBody, String> {
    let artifact_id = artifact_id.to_string();
    match kind {
        DecisionKind::Approve => Ok(DecisionBody::Approve(ApprovalDecision {
            artifact_id,
            conditions: None,
        })),
        DecisionKind::ApproveWithConditions => {
            validated_review_text(text, "conditions").map(|conditions| {
                DecisionBody::Approve(ApprovalDecision {
                    artifact_id,
                    conditions: Some(conditions),
                })
            })
        }
        DecisionKind::Reject => validated_review_text(text, "reason").map(|reason| {
            DecisionBody::Reject(RejectionDecision {
                artifact_id,
                reason,
            })
        }),
    }
}

/// The decision form for one mission's artifact under review.
pub(super) fn decision_form(
    mission_id: String,
    artifact_id: String,
    desk: ApprovalsDesk,
) -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    let kind = RwSignal::new(DecisionKind::Approve);
    let text = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let problem = RwSignal::new(None::<String>);
    let target = StoredValue::new((mission_id, artifact_id));
    let send = move |_| {
        let (mission, artifact) = target.get_value();
        let decision = kind.get_untracked();
        let body = match decision_body(decision, &artifact, &text.get_untracked()) {
            Ok(body) => body,
            Err(why) => {
                problem.set(Some(why));
                return;
            }
        };
        #[cfg(target_arch = "wasm32")]
        {
            use super::decision_refusal::DecisionRefusal;
            use crate::v2::core::api::endpoints::mission_reviews::{approve_review, reject_review};
            if busy.get_untracked() {
                return;
            }
            busy.set(true);
            problem.set(None);
            let toasts = crate::v2::core::ui::toast::use_toasts();
            leptos::task::spawn_local(async move {
                let answer = match &body {
                    DecisionBody::Approve(approval) => {
                        approve_review(store, &mission, approval).await
                    }
                    DecisionBody::Reject(rejection) => {
                        reject_review(store, &mission, rejection).await
                    }
                };
                match answer {
                    Ok(_) => {
                        toasts.success(decision.recorded());
                        desk.notice.set(None);
                        desk.refetch.run(());
                    }
                    Err(refused) => {
                        let refusal = DecisionRefusal::from_refusal(
                            &refused,
                            "The decision could not be sent",
                        );
                        if refusal.reloads_queue() {
                            desk.notice.set(Some((mission.clone(), refusal.sentence())));
                            desk.refetch.run(());
                        } else {
                            let _ = problem.try_set(Some(refusal.sentence()));
                        }
                    }
                }
                let _ = busy.try_set(false);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = (body, store, desk, mission);
    };
    view! {
        <div class="sticky bottom-0 mt-auto flex flex-col gap-3 border-t border-white/5 bg-surface-container/40 p-6 backdrop-blur-xl">
            <div role="radiogroup" aria-label="Decision" class="flex flex-wrap gap-2">
                {DecisionKind::ALL
                    .into_iter()
                    .map(|option| {
                        view! {
                            <button
                                type="button"
                                role="radio"
                                aria-checked=move || (kind.get() == option).to_string()
                                on:click=move |_| {
                                    kind.set(option);
                                    problem.set(None);
                                }
                                class=move || {
                                    cn(
                                        &[
                                            "rounded-full border px-4 py-2 text-label-md transition",
                                            if kind.get() == option {
                                                "border-primary bg-primary/15 text-primary"
                                            } else {
                                                "border-white/10 text-on-surface-variant hover:bg-white/5"
                                            },
                                        ],
                                    )
                                }
                            >
                                {option.label()}
                            </button>
                        }
                    })
                    .collect_view()}
            </div>
            {move || {
                kind.get()
                    .text_label()
                    .map(|label| {
                        view! {
                            <label class="flex flex-col gap-1">
                                <span class="text-label-sm font-medium tracking-wide text-on-surface-variant uppercase">
                                    {label}
                                </span>
                                <textarea
                                    rows="2"
                                    prop:value=move || text.get()
                                    on:input=move |ev| text.set(event_target_value(&ev))
                                    class="w-full resize-y rounded-xl border border-white/10 bg-white/5 px-4 py-3 text-label-md text-on-surface outline-none transition focus:border-primary/40"
                                ></textarea>
                            </label>
                        }
                    })
            }}
            <div class="flex items-center justify-end gap-3">
                <span class="mr-auto text-label-sm text-error-alert">{move || problem.get()}</span>
                <button
                    type="button"
                    data-testid="approvals-send-decision"
                    on:click=send
                    prop:disabled=move || busy.get()
                    class="rounded-full bg-emerald-600 px-7 py-3 text-label-md font-bold text-white shadow-[0_0_20px_rgba(16,185,129,0.3)] transition hover:bg-emerald-500 disabled:opacity-50"
                >
                    {move || format!("Send decision: {}", kind.get().label())}
                </button>
            </div>
        </div>
    }
}
