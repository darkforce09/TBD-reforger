//! Vehicle Database (/vehicles) — ported from pages/doctrine.tsx `VehicleDatabasePage`. `<AuthGate>`
//! → a `GlassSplit`: a faction-grouped vehicle list (master) + a cinematic dossier (detail). Fully
//! client-MOCK-driven (`VEHICLES`) until a vehicle intel API lands.
//!
//! **Gate scope (this slice):** the default render (search empty, `btr-70` selected) — the grouped
//! list + the BTR-70 `VehicleDossier` (hero + badges + directive + telemetry + armament/threats) is
//! byte-exact-verified (deterministic mock). Search + selection are behavior — a follow-up.
#![allow(dead_code)]
use crate::split_pane::{GlassSplit, ListDetailItem, SidebarSearch};
use crate::ui::MaterialIcon;
use leptos::prelude::*;

struct Vehicle {
    id: &'static str,
    name: &'static str,
    faction: &'static str,
    class: &'static str,
    threat: &'static str,
    short_desc: &'static str,
    critical_directive: &'static str,
    mobility: &'static str,
    defense: &'static str,
    capacity: &'static str,
    armament: &'static [&'static str],
    threats: &'static [&'static str],
    image: Option<&'static str>,
}

// Badge variant classes (ui/badge.tsx cn(), text-label-sm twMerge-dropped).
const BADGE_NEUTRAL: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-outline-variant/40 bg-surface-variant/40 text-on-surface-variant";
const BADGE_WARNING: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-tactical-yellow/30 bg-tactical-yellow/10 text-tactical-yellow";
const BADGE_SUCCESS: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-success/30 bg-success/15 text-success";
const BADGE_ERROR: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-error-alert/30 bg-error-alert/10 text-error-alert";
const BADGE_PRIMARY: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-primary/30 bg-primary/10 text-primary";

fn threat_badge(level: &str) -> &'static str {
    match level {
        "HIGH" => BADGE_ERROR,
        "MED" => BADGE_WARNING,
        _ => BADGE_SUCCESS,
    }
}

const VEHICLE_FACTION_ORDER: [&str; 2] = ["USSR Forces", "US Forces"];

const VEHICLES: &[Vehicle] = &[
    Vehicle {
        id: "btr-70",
        name: "BTR-70",
        faction: "USSR Forces",
        class: "APC",
        threat: "MED",
        short_desc: "8×8 wheeled amphibious APC. A fast road mover for shuttling infantry — thin armour means it is a battle taxi, not a fighting vehicle.",
        critical_directive: "Do not let it bully you with the KPVT. The hull stops rifle rounds only — a single RPG or sustained .50 cal will brew it up with the whole squad inside.",
        mobility: "80 km/h · Amphibious",
        defense: "Light · ~10mm steel",
        capacity: "2 crew + 8 pax",
        armament: &["14.5mm KPVT HMG", "7.62mm PKT coax"],
        threats: &["Infantry AT", "Heavy MG", "Autocannon"],
        image: Some("https://lh3.googleusercontent.com/aida-public/AB6AXuAa6uQ_tsnbNhDf8cIp17ebpWDCdpJntK9g1ME75jrq_heGg9E-S3PYbbWNY2nunPGsJZDn-Zd7FEt3Jff2dDz_ZIqRZzxXlXp3OKqkQIoTmXkozbiwqK3iC_VLuc3hKtPKcznvLKREbs_XU_mNuUq7r7Wx9aX6GYMjJrlVza8sEz5zAAcKFdjbj5giyYLbY8jd3ZoBYl-IEL8aAWt9a9P6R7bs7wJyjK1DEuGhhu-z1dypXTXCul5dANMGZJAcAwNp4Hk4C_5-60c"),
    },
    Vehicle {
        id: "bmp-2",
        name: "BMP-2",
        faction: "USSR Forces",
        class: "IFV",
        threat: "HIGH",
        short_desc: "Tracked IFV pairing a hard-hitting 30mm autocannon with an ATGM. Lethal to infantry and light vehicles, but its armour is still thin.",
        critical_directive: "The 30mm is the real threat to your squad, not the hull. Break line of sight immediately — do not try to outrun it across open ground.",
        mobility: "65 km/h · Amphibious",
        defense: "Light · spaced steel",
        capacity: "3 crew + 7 pax",
        armament: &["30mm 2A42 autocannon", "9M113 Konkurs ATGM", "7.62mm PKT coax"],
        threats: &["Tank main gun", "Tandem ATGM", "Top-attack"],
        image: None,
    },
    Vehicle {
        id: "m1a1-abrams",
        name: "M1A1 Abrams",
        faction: "US Forces",
        class: "MBT",
        threat: "HIGH",
        short_desc: "Main battle tank. The frontal armour is near-impervious to most man-portable AT; the exploitable threat is its flanks, rear, and top.",
        critical_directive: "Never engage frontally with light AT — you will only give away your position. Maneuver for a side or rear shot, or hit the top with tandem/top-attack munitions.",
        mobility: "67 km/h · 1500 hp",
        defense: "Composite + DU armour",
        capacity: "4 crew",
        armament: &["120mm M256 smoothbore", "12.7mm M2 cupola", "7.62mm M240 coax"],
        threats: &["Tandem ATGM", "Top-attack", "AT mines"],
        image: None,
    },
    Vehicle {
        id: "m2-bradley",
        name: "M2 Bradley",
        faction: "US Forces",
        class: "IFV",
        threat: "HIGH",
        short_desc: "Tracked IFV pairing a 25mm autocannon with TOW missiles. It will shred infantry up close and kill armour at range.",
        critical_directive: "The TOW outranges your AT launchers. Close the distance through hard cover, or stay out of its line of sight entirely — do not trade in the open.",
        mobility: "66 km/h · 600 hp",
        defense: "Aluminium + appliqué",
        capacity: "3 crew + 6 pax",
        armament: &["25mm M242 Bushmaster", "TOW ATGM launcher", "7.62mm M240 coax"],
        threats: &["Tank main gun", "Tandem ATGM", "Autocannon"],
        image: None,
    },
];

#[component]
pub fn VehicleDatabasePage() -> impl IntoView {
    // Default: search empty, selectedId = "btr-70".
    let selected = &VEHICLES[0];
    view! {
        <crate::ui::AuthGate>
            <GlassSplit
                master_width="18rem"
                master_header=master_header().into_any()
                master=vehicle_list("btr-70").into_any()
                detail=dossier(selected).into_any()
            />
        </crate::ui::AuthGate>
    }
}

fn master_header() -> impl IntoView {
    view! {
        <div class="w-full space-y-3">
            <p class="font-mono text-xs font-bold tracking-widest text-on-surface-variant uppercase">
                "Vehicle Database"
            </p>
            <SidebarSearch placeholder="Search assets..." />
        </div>
    }
}

fn vehicle_list(selected_id: &'static str) -> impl IntoView {
    VEHICLE_FACTION_ORDER
        .iter()
        .filter_map(move |faction| {
            let rows: Vec<&Vehicle> = VEHICLES.iter().filter(|v| v.faction == *faction).collect();
            if rows.is_empty() {
                return None;
            }
            Some(view! {
                <div class="mb-3">
                    <p class="px-1 py-1 font-mono text-[11px] tracking-widest text-outline uppercase">
                        {*faction}
                    </p>
                    <div class="mt-1 flex flex-col gap-1">
                        {rows
                            .into_iter()
                            .map(|v| {
                                view! {
                                    <ListDetailItem
                                        active=v.id == selected_id
                                        title=view! { {v.name} }.into_any()
                                        preview=view! {
                                            <span class="font-mono uppercase text-outline">
                                                {v.class}
                                            </span>
                                        }
                                            .into_any()
                                    />
                                }
                            })
                            .collect_view()}
                    </div>
                </div>
            })
        })
        .collect_view()
}

fn dossier(v: &'static Vehicle) -> impl IntoView {
    let hero = match v.image {
        Some(src) => view! { <img src=src alt="" class="h-full w-full object-cover" /> }.into_any(),
        None => view! {
            <div class="flex h-full w-full items-center justify-center bg-surface-container-low">
                <MaterialIcon name="directions_car" class="text-7xl text-outline" />
            </div>
        }
        .into_any(),
    };
    view! {
        <div>
            <div class="relative h-72 w-full overflow-hidden">
                {hero}
                <div class="absolute inset-0 bg-gradient-to-t from-surface-dim to-transparent"></div>
                <div class="absolute right-8 bottom-6 left-8">
                    <div class="mb-3 flex flex-wrap items-center gap-2">
                        <span class=BADGE_NEUTRAL>"CLASS: "{v.class}</span>
                        <span class=threat_badge(v.threat)>"THREAT: "{v.threat}</span>
                        <span class=BADGE_PRIMARY>{v.faction}</span>
                    </div>
                    <h1 class="text-4xl font-black tracking-tighter text-white uppercase">
                        {v.name}
                    </h1>
                    <p class="mt-2 max-w-2xl text-body-md text-on-surface-variant">{v.short_desc}</p>
                </div>
            </div>
            <div class="space-y-8 p-8 md:p-12">
                <div class="rounded-2xl border-l-4 border-tactical-yellow bg-tactical-yellow/10 p-4 shadow-lg backdrop-blur-md">
                    <p class="mb-1 font-mono text-xs font-bold tracking-widest text-tactical-yellow uppercase">
                        "Critical Directive"
                    </p>
                    <p class="text-body-md leading-relaxed text-on-surface-variant">
                        {v.critical_directive}
                    </p>
                </div>
                <div>
                    {section_title("Telemetry")}
                    <div class="grid grid-cols-1 gap-4 sm:grid-cols-3">
                        {vehicle_stat("Mobility", v.mobility)} {vehicle_stat("Defense", v.defense)}
                        {vehicle_stat("Capacity", v.capacity)}
                    </div>
                </div>
                <div class="grid grid-cols-1 gap-8 md:grid-cols-2">
                    <div>
                        {section_title("Armament")}
                        <ul class="space-y-2">
                            {v.armament
                                .iter()
                                .map(|w| {
                                    view! {
                                        <li class="flex items-center gap-3 rounded-lg border border-white/10 bg-white/5 px-3 py-2.5 font-mono text-sm text-on-surface-variant">
                                            <span class="h-1.5 w-1.5 shrink-0 rounded-full bg-primary"></span>
                                            {*w}
                                        </li>
                                    }
                                })
                                .collect_view()}
                        </ul>
                    </div>
                    <div>
                        {section_title("Primary Threats")}
                        <div class="flex flex-wrap gap-2">
                            {v.threats
                                .iter()
                                .map(|t| {
                                    view! {
                                        <span class="rounded-full border border-error-alert/30 bg-error-alert/10 px-3 py-1 font-mono text-xs tracking-wide text-error-alert uppercase">
                                            {*t}
                                        </span>
                                    }
                                })
                                .collect_view()}
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}

fn section_title(t: &'static str) -> impl IntoView {
    view! {
        <h2 class="mb-3 font-mono text-xs font-bold tracking-widest text-on-surface-variant uppercase">
            {t}
        </h2>
    }
}

fn vehicle_stat(label: &'static str, value: &'static str) -> impl IntoView {
    view! {
        <div class="rounded-xl border border-white/10 bg-white/5 p-4">
            <p class="font-mono text-[11px] tracking-widest text-on-surface-variant uppercase">
                {label}
            </p>
            <p class="mt-1 font-mono text-base text-white">{value}</p>
        </div>
    }
}
