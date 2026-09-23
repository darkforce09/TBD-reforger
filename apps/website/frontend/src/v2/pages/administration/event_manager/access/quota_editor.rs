//! The Places section: the member, guest and open pools, how much of each is held, and their editor.
//!
//! **Role:** renders the places held by pool against the operation-wide limit, and a form of the
//! three pools — each capped at a number of places or uncapped, and opening at a time given in UTC —
//! with the save that replaces all three together.
//! **Position:** the third section of the access panel.
//! **Signals & state:** the form is one signal seeded from the access view each time the section is
//! built, so a saved or reloaded view reseeds it.
//! **Invariants:** an uncapped pool is sent as an explicit null limit and zero closes a pool — two
//! different decisions the form keeps apart. Opening times are edited and shown in UTC; an opening
//! time the operator did not touch is sent back exactly as it was read, sub-second part included,
//! so saving a limit change cannot move an opening. The backend refuses a limit below the places a
//! pool already holds, and that refusal is shown as it is worded.

use super::state::AccessPanel;
use crate::v2::core::api::dto::{QuotaUsageView, ReservationQuotaPool, ReservationQuotas};
use crate::v2::core::utils::utc_timestamp::UtcTimestamp;
use leptos::prelude::*;

/// One pool as the form holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PoolFields {
    pub(super) uncapped: bool,
    /// The limit as typed.
    pub(super) seats: String,
    /// The opening as a date-and-time field value, read as UTC.
    pub(super) opens_at: String,
    /// The opening as the access view carried it.
    pub(super) read_opens_at: String,
}

impl PoolFields {
    fn of(pool: &ReservationQuotaPool) -> Self {
        Self {
            uncapped: pool.seats.is_none(),
            seats: pool.seats.map(|s| s.to_string()).unwrap_or_default(),
            opens_at: UtcTimestamp::parse(&pool.opens_at)
                .map(|at| at.datetime_field_value())
                .unwrap_or_default(),
            read_opens_at: pool.opens_at.clone(),
        }
    }

    fn validated(&self, pool: &str) -> Result<ReservationQuotaPool, String> {
        let seats = if self.uncapped {
            None
        } else {
            match self.seats.trim().parse::<u32>() {
                Ok(limit) => Some(i64::from(limit)),
                Err(_) => {
                    return Err(format!(
                        "The {pool} pool's limit must be a whole number of 0 or more, or uncapped"
                    ))
                }
            }
        };
        let untouched = UtcTimestamp::parse(&self.read_opens_at)
            .is_some_and(|read| read.datetime_field_value() == self.opens_at);
        let opens_at = if untouched {
            self.read_opens_at.clone()
        } else {
            UtcTimestamp::from_datetime_field(&self.opens_at)
                .map(|at| at.to_rfc3339())
                .ok_or_else(|| format!("The {pool} pool needs an opening date and time (UTC)"))?
        };
        Ok(ReservationQuotaPool { seats, opens_at })
    }
}

/// The three pools as the form holds them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct QuotaFields {
    pub(super) member: PoolFields,
    pub(super) guest: PoolFields,
    pub(super) open: PoolFields,
}

impl QuotaFields {
    /// The form for the pools the access view carries.
    pub(super) fn of(quotas: &ReservationQuotas) -> Self {
        Self {
            member: PoolFields::of(&quotas.member),
            guest: PoolFields::of(&quotas.guest),
            open: PoolFields::of(&quotas.open),
        }
    }

    /// The pools the backend takes, or what is wrong with the form.
    pub(super) fn validated(&self) -> Result<ReservationQuotas, String> {
        Ok(ReservationQuotas {
            member: self.member.validated("member")?,
            guest: self.guest.validated("guest")?,
            open: self.open.validated("open")?,
        })
    }

    fn pool_mut(&mut self, pool: Pool) -> &mut PoolFields {
        match pool {
            Pool::Member => &mut self.member,
            Pool::Guest => &mut self.guest,
            Pool::Open => &mut self.open,
        }
    }

    fn pool(&self, pool: Pool) -> &PoolFields {
        match pool {
            Pool::Member => &self.member,
            Pool::Guest => &self.guest,
            Pool::Open => &self.open,
        }
    }
}

/// Which of the three pools a row edits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pool {
    Member,
    Guest,
    Open,
}

/// The places held, by pool, against the operation-wide limit.
pub(super) fn usage_line(usage: &QuotaUsageView, max_slots: i64) -> String {
    let limit = if max_slots > 0 {
        format!("{} of {max_slots} operation places held", usage.total)
    } else {
        format!("{} places held; no operation-wide limit", usage.total)
    };
    let legacy = if usage.legacy_unclassified > 0 {
        format!(" · {} from before the pools", usage.legacy_unclassified)
    } else {
        String::new()
    };
    format!(
        "{limit} — member {} · guest {} · open {}{legacy}",
        usage.member, usage.guest, usage.open
    )
}

/// The Places section.
pub(super) fn quota_editor(panel: AccessPanel) -> impl IntoView {
    move || {
        let Some(access) = panel.access.with(|a| a.loaded().cloned()) else {
            return ().into_any();
        };
        let form = RwSignal::new(QuotaFields::of(&access.reservation_quotas));
        let usage = usage_line(&access.quota_usage, access.max_slots);
        let held = [
            (Pool::Member, "Member places", access.quota_usage.member),
            (Pool::Guest, "Guest places", access.quota_usage.guest),
            (Pool::Open, "Open places", access.quota_usage.open),
        ];
        let save = move |_| {
            #[cfg(target_arch = "wasm32")]
            send_quotas(panel, form.get_untracked());
        };
        view! {
            <div class="space-y-4">
                <p class="font-mono text-sm text-on-surface">{usage}</p>
                <p class="text-xs text-on-surface-variant">
                    "Members draw from the member pool and everyone else from the guest pool; either overflows to the open pool once it opens. Zero closes a pool; uncapped leaves only the operation-wide limit. A limit cannot go below the places the pool already holds."
                </p>
                <div class="space-y-2">
                    {held.into_iter().map(|(pool, name, taken)| pool_row(form, pool, name, taken)).collect_view()}
                </div>
                <div class="flex justify-end">
                    <button
                        type="button"
                        on:click=save
                        prop:disabled=move || panel.busy.get()
                        class="rounded-full bg-action px-4 py-1.5 text-sm font-medium text-on-action disabled:opacity-50"
                    >
                        "Save pools"
                    </button>
                </div>
            </div>
        }
        .into_any()
    }
}

/// One pool's row: its limit or uncapped, its opening in UTC, and what it holds.
fn pool_row(
    form: RwSignal<QuotaFields>,
    pool: Pool,
    name: &'static str,
    held: i64,
) -> impl IntoView {
    let uncapped = Memo::new(move |_| form.with(|f| f.pool(pool).uncapped));
    view! {
        <div class="flex flex-wrap items-center gap-3 rounded-lg border border-white/10 px-3 py-2 text-sm">
            <span class="w-32 text-on-surface">{name}</span>
            <label class="flex items-center gap-1.5 text-xs text-on-surface-variant">
                <input
                    type="checkbox"
                    prop:checked=move || uncapped.get()
                    on:change=move |ev| {
                        let checked = event_target_checked(&ev);
                        form.update(|f| f.pool_mut(pool).uncapped = checked);
                    }
                />
                "Uncapped"
            </label>
            <input
                type="number"
                min="0"
                aria-label=format!("{name}: limit")
                placeholder="Limit"
                prop:disabled=move || uncapped.get()
                prop:value=move || form.with(|f| f.pool(pool).seats.clone())
                on:input=move |ev| form.update(|f| f.pool_mut(pool).seats = event_target_value(&ev))
                class="w-24 rounded-md border border-outline-variant/40 bg-surface px-2 py-1 font-mono text-sm text-on-surface disabled:opacity-40 [color-scheme:dark]"
            />
            <label class="flex items-center gap-1.5 text-xs text-on-surface-variant">
                "Opens (UTC)"
                <input
                    type="datetime-local"
                    aria-label=format!("{name}: opens at, UTC")
                    prop:value=move || form.with(|f| f.pool(pool).opens_at.clone())
                    on:input=move |ev| form.update(|f| f.pool_mut(pool).opens_at = event_target_value(&ev))
                    class="rounded-md border border-outline-variant/40 bg-surface px-2 py-1 font-mono text-sm text-on-surface [color-scheme:dark]"
                />
            </label>
            <span class="ml-auto font-mono text-xs text-on-surface-variant">{format!("{held} held")}</span>
        </div>
    }
}

/// Replace the three pools with the form's.
#[cfg(target_arch = "wasm32")]
fn send_quotas(panel: AccessPanel, form: QuotaFields) {
    use crate::v2::core::api::dto::ReservationQuotaChange;
    use crate::v2::core::api::endpoints::event_access_administration::put_reservation_quotas;
    use futures::FutureExt;
    let quotas = match form.validated() {
        Ok(quotas) => quotas,
        Err(problem) => {
            crate::v2::core::ui::toast::use_toasts().error(problem);
            return;
        }
    };
    panel.apply(
        "Reservation pools saved".to_string(),
        move |store, event, revision| {
            async move {
                let change = ReservationQuotaChange {
                    expected_access_revision: revision,
                    reservation_quotas: quotas,
                };
                put_reservation_quotas(store, &event, &change).await
            }
            .boxed_local()
        },
    );
}
