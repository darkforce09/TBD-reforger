//! Role: radio.
//! Position: `mission/compiler/flatten` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::{
    HashSet, MOD_MAX_NETS, ModNet, ModRadioPlan, NET_FREQ_BASE_MHZ, NET_FREQ_STEP_MHZ,
    RadioNetSource, cap_net_label, unique_net_id,
};

/// The cut is also made HERE rather than left to the mod, so the compiled document a human reads is the plan the server actually runs.
pub(super) fn resolve_radio_plan(
    authored: Option<&crate::mission::radio_plan::AuthoredRadioPlan>,
    sources: &[RadioNetSource],
) -> Option<ModRadioPlan> {
    if let Some(plan) = authored {
        return Some(mod_plan_from_authored(plan));
    }
    derive_radio_plan(sources)
}

/// Mod plan from authored using the supplied domain data.
pub(super) fn mod_plan_from_authored(
    plan: &crate::mission::radio_plan::AuthoredRadioPlan,
) -> ModRadioPlan {
    ModRadioPlan {
        nets: plan
            .nets
            .iter()
            .map(|n| ModNet {
                id: n.id.clone(),
                label: n.label.clone(),
                freq_mhz: n.freq_mhz,
                faction: n.faction.clone().unwrap_or_default(),
                range: n.range.clone(),
            })
            .collect(),
    }
}

/// Derive radio plan using the supplied domain data.
pub(super) fn derive_radio_plan(sources: &[RadioNetSource]) -> Option<ModRadioPlan> {
    let mut nets: Vec<ModNet> = Vec::new();
    let mut used_ids: HashSet<String> = HashSet::new();

    let mut push =
        |nets: &mut Vec<ModNet>, src: &RadioNetSource, slug: &str, label: &str, long: bool| {
            let index = nets.len();
            nets.push(ModNet {
                id: unique_net_id(&mut used_ids, &src.faction_key, slug),
                label: cap_net_label(label),
                freq_mhz: NET_FREQ_BASE_MHZ + NET_FREQ_STEP_MHZ * index as f64,
                faction: src.faction_key.clone(),
                range: long.then(|| "long".to_string()),
            });
        };

    for src in sources.iter().take(MOD_MAX_NETS) {
        let label = format!("{} Command", src.display_name);
        push(&mut nets, src, "cmd", &label, true);
    }

    let deepest = sources.iter().map(|s| s.callsigns.len()).max().unwrap_or(0);
    'ranks: for rank in 0..deepest {
        for src in sources {
            if nets.len() >= MOD_MAX_NETS {
                break 'ranks;
            }
            if let Some(callsign) = src.callsigns.get(rank) {
                push(&mut nets, src, callsign, callsign, false);
            }
        }
    }

    (!nets.is_empty()).then_some(ModRadioPlan { nets })
}
