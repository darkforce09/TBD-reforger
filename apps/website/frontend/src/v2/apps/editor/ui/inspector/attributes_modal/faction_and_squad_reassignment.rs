//! Attributes modal faction and squad reassignment behavior.

use super::*;

/// Renders faction and squad selection for the entire target set.
#[cfg(target_arch = "wasm32")]
pub(super) fn reassign_picker(targets: StoredValue<Vec<String>>) -> impl IntoView {
    use website_map_engine::editing::hosted_commands as ops;

    let (factions, squads) = ops::reassign_rows();
    let mut ordered = factions.clone();
    ordered.sort_by(|a, b| a.id.cmp(&b.id));

    let ids = targets.get_value();
    let squad_by_slot: std::collections::HashMap<&str, &str> = squads
        .iter()
        .flat_map(|s| {
            s.slot_ids
                .iter()
                .map(move |sid| (sid.as_str(), s.id.as_str()))
        })
        .collect();
    let current_squads: Vec<String> = ids
        .iter()
        .filter_map(|id| squad_by_slot.get(id.as_str()).map(|s| (*s).to_string()))
        .collect();
    let one_squad = current_squads
        .first()
        .filter(|first| {
            current_squads.len() == ids.len() && current_squads.iter().all(|s| &s == first)
        })
        .cloned()
        .unwrap_or_default();
    let faction_of = |sid: &String| {
        squads
            .iter()
            .find(|s| &s.id == sid)
            .map(|s| s.faction_id.clone())
            .unwrap_or_default()
    };
    let current_factions: Vec<String> = current_squads.iter().map(faction_of).collect();
    let one_faction = current_factions
        .first()
        .filter(|first| {
            current_factions.len() == ids.len() && current_factions.iter().all(|f| &f == first)
        })
        .cloned()
        .unwrap_or_default();

    let listed_faction = one_faction.clone();
    let squad_options: Vec<(String, String)> = ordered
        .iter()
        .find(|f| f.id == listed_faction)
        .map(|f| {
            f.squad_ids
                .iter()
                .filter_map(|sid| squads.iter().find(|s| &s.id == sid))
                .map(|s| {
                    let name = if s.name.trim().is_empty() {
                        s.id.clone()
                    } else {
                        s.name.clone()
                    };
                    (s.id.clone(), format!("{name} ({})", s.slot_ids.len()))
                })
                .collect()
        })
        .unwrap_or_default();

    let no_squads = squad_options.is_empty();
    let refusal = RwSignal::new(String::new());
    let n = ids.len();
    let commit = move |faction_id: String, squad_id: String| {
        let target = ops::ReassignTarget {
            faction_id,
            squad_id,
        };
        match ops::reassign_slots(&targets.get_value(), &target) {
            Ok(_) => refusal.set(String::new()),
            Err(reason) => refusal.set(reason),
        }
    };
    let commit_faction = commit;
    let commit_squad = commit;
    let faction_for_squad = one_faction.clone();

    view! {
        <div class="flex flex-col gap-3">
            <label class="flex flex-col gap-1">
                <span class="text-label-sm uppercase tracking-wider text-outline">"Faction"</span>
                <select
                    aria-label="Faction"
                    class=CONTROL
                    prop:value=one_faction.clone()
                    on:change=move |ev| {
                        let picked = event_target_value(&ev);
                        if !picked.is_empty() {
                            commit_faction(picked, String::new());
                        }
                    }
                >
                    <option value="" selected=one_faction.is_empty()>
                        {if n > 1 { "Mixed — pick a faction to move all" } else { "Unfiled" }}
                    </option>
                    {ordered
                        .iter()
                        .map(|f| {
                            let sel = f.id == one_faction;
                            view! {
                                <option value=f.id.clone() selected=sel>
                                    {faction_label(f)}
                                </option>
                            }
                        })
                        .collect_view()}
                </select>
            </label>
            <label class="flex flex-col gap-1">
                <span class="text-label-sm uppercase tracking-wider text-outline">"Squad"</span>
                <select
                    aria-label="Squad"
                    class=CONTROL
                    prop:value=one_squad.clone()
                    disabled=no_squads
                    on:change=move |ev| {
                        let picked = event_target_value(&ev);
                        if !picked.is_empty() {
                            commit_squad(faction_for_squad.clone(), picked);
                        }
                    }
                >
                    <option value="" selected=one_squad.is_empty()>
                        {if no_squads {
                            "No squads under this faction"
                        } else if n > 1 {
                            "Mixed — pick a squad to move all"
                        } else {
                            "Unfiled"
                        }}
                    </option>
                    {squad_options
                        .into_iter()
                        .map(|(id, label)| {
                            let sel = id == one_squad;
                            view! { <option value=id selected=sel>{label}</option> }
                        })
                        .collect_view()}
                </select>
            </label>
            {move || {
                let why = refusal.get();
                (!why.is_empty())
                    .then(|| {
                        view! {
                            <p
                                role="alert"
                                class="rounded-md border border-error/40 bg-error/10 px-2.5 py-1.5 text-label-sm normal-case text-error"
                            >
                                {why}
                            </p>
                        }
                    })
            }}
            <p class="text-label-sm normal-case text-outline">
                {if n > 1 {
                    format!("Applies to all {n} selected entities, as one undo step.")
                } else {
                    "Moving a slot out never deletes the squad it left.".to_string()
                }}
            </p>
        </div>
    }
}
