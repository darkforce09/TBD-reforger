//! Mission compile flatten (gate G6) — Rust port of `services/mission_compile.go`,
//! the twin of the frontend `flattenModDocument.ts`. Derives the CANONICAL mod
//! mission document (mission.schema.json, string schemaVersion "1.1"/"1.2") from a
//! mission row + its version payload, mirroring the TS traversal EXACTLY so
//! `/missions/:id/compiled` and the client-side flatten agree.
//!
//! Locked coordinate mapping: editor `position.x → x`, `position.y → z`,
//! `position.z → y` (optional, 1.2), `position.rotation → headingDeg`.
//!
//! @contract mission.schema.json#/

use std::collections::{BTreeMap, HashMap, HashSet};

use serde::Serialize;

use crate::mission::kit::load_kit_aliases;
use crate::mission::wire_safety::is_wire_unsafe;

// ---- output document types (camelCase — the game-server contract) ----

/// One flattened `slots[]` entry.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModSlot {
    pub id: String,
    /// Stable slot identity (B1): the editor doc's slot id, carried verbatim so the
    /// identity survives recompiles — `id` above is DERIVED (faction:callsign:role:
    /// occurrence) and shifts under role renames/reorders/deletes. Spawn points,
    /// rosters and logs should key on `uid`; `id` stays the human-readable label.
    /// (Named `uid`, not `ref` — `ref` is an EnforceScript keyword and the mod
    /// struct field names must equal the JSON keys.)
    pub uid: String,
    pub faction: String,
    pub group_callsign: String,
    pub role: String,
    pub kit: String,
    pub x: f64,
    pub z: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<f64>,
    pub heading_deg: f64,
    /// Optional Arsenal loadout (T-068.11) — omitted when the editor slot carries none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loadout: Option<ModSlotLoadout>,
}

/// Per-slot loadout block (mission.schema.json `slot.loadout`): fixed gear + container
/// cargo, derived from the editor `SlotLoadoutV2`. Kit alias stays the base character;
/// this layers on top (T-068.12 equips it onto the spawned player).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModSlotLoadout {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gear: Option<ModSlotGear>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cargo: Vec<ModSlotCargo>,
}

/// Fixed gear ResourceNames — the v1 mod-reader shape, same derivation the
/// loadout-export schema documents: jacket→uniform, **armoredVest else vest→vest
/// (known collapse: a chest rig layered under a plate carrier loses the rig —
/// single-vest rule, documented)**, headCover→helmet; A3 widens with
/// pants/boots/handwear/backpack so an Arsenal-authored slot arrives complete.
/// **T-182** adds the three weapon slots the compiler used to discard, so all
/// four authored weapons now reach the wire — see `mod_slot_loadout` for the
/// `(slotIndex, slotType)` selectors. Empty slots are omitted, never empty
/// strings.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModSlotGear {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub magazine: Option<String>,
    /// T-182 — the other three authored weapon slots. Named with the EDITOR's own vocabulary
    /// (`arsenal_rules.rs` `WEAPON_SLOTS`) so the compiled document reads the same words the
    /// Arsenal UI shows. None of the three carry optic/magazine sub-slots — those ride the
    /// slotIndex-0 primary alone.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub launcher: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handgun: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub throwable: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uniform: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub helmet: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pants: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boots: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handwear: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backpack: Option<String>,
}

impl ModSlotGear {
    fn is_empty(&self) -> bool {
        self.primary.is_none()
            && self.optic.is_none()
            && self.magazine.is_none()
            // T-182 — a launcher-only (or throwable-only) gear block is authored content. Omit
            // these three and `mod_slot_loadout` would drop the whole `loadout` key for such a
            // slot, so the fields would never reach the wire in the one case they are the only
            // thing on it.
            && self.launcher.is_none()
            && self.handgun.is_none()
            && self.throwable.is_none()
            && self.uniform.is_none()
            && self.vest.is_none()
            && self.helmet.is_none()
            && self.pants.is_none()
            && self.boots.is_none()
            && self.handwear.is_none()
            && self.backpack.is_none()
    }
}

/// One container cargo row (`{container, item, qty}` — loadout-export v2), copied
/// verbatim from the editor cargo.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModSlotCargo {
    pub container: String,
    pub item: String,
    pub qty: i64,
}

#[derive(Debug, Serialize)]
pub struct ModOrbatRole {
    pub slot: String,
    pub kit: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct ModOrbatGroup {
    pub callsign: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub roles: Vec<ModOrbatRole>,
}

#[derive(Debug, Serialize)]
pub struct ModOrbatFaction {
    pub groups: Vec<ModOrbatGroup>,
}

/// One `radioPlan.nets[]` entry (`mission.schema.json#/$defs/net`) — see
/// [`derive_radio_plan`] for where the values come from and what they do not claim.
#[derive(Debug, Serialize)]
pub struct ModNet {
    /// `^net:[a-z0-9_]+$`. Unique within the document — the mod treats it as the stable
    /// channel key and the VOIP bridge keys voice channels on it
    /// (`packages/tbd-schema/bridge/bridge-contract.md` §radioPlan → voice net mapping).
    pub id: String,
    /// Display name. Capped at [`MOD_MAX_LABEL_CHARS`] here so the mod never has to.
    pub label: String,
    /// NOT `#[serde(rename_all = "camelCase")]`: serde would camel-case `freq_mhz` to
    /// `freqMhz`, and the mod binds this block by field NAME through `JsonLoadContext`,
    /// which ignores keys it does not recognise. The whole radio plan would arrive with
    /// every frequency at 0 — which `TBD_RadioPlan.Fault` then rejects as out-of-band,
    /// so the failure mode is a silently empty plan, not a parse error.
    #[serde(rename = "freqMHz")]
    pub freq_mhz: f64,
    /// Always set. Every derived net belongs to exactly one side — see [`derive_radio_plan`].
    pub faction: String,
    /// `"long"` on command nets, ABSENT on squad nets. Never `"short"` — see
    /// [`derive_radio_plan`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<String>,
}

/// `mission.schema.json#/$defs/radioPlan`. `nets` carries `minItems: 1`, so an empty plan
/// is a schema violation rather than an empty block — the whole key is omitted instead
/// (`radioPlan` is not in the schema's top-level `required`, and `TBD_RadioPlan.Parse`
/// treats an absent plan as legal and logs `nets=0`).
#[derive(Debug, Serialize)]
pub struct ModRadioPlan {
    pub nets: Vec<ModNet>,
}

#[derive(Debug, Serialize)]
pub struct ModCircle {
    pub x: f64,
    pub z: f64,
    pub r: f64,
}

#[derive(Debug, Serialize)]
pub struct ModZoneShape {
    pub circle: ModCircle,
}

#[derive(Debug, Serialize)]
pub struct ModZone {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub faction: String,
    pub shape: ModZoneShape,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModFaction {
    pub key: String,
    pub display_name: String,
    pub preset_id: String,
    pub tickets: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModMeta {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub author: String,
    pub terrain: String,
    pub template_id: String,
    pub player_range: [i64; 2],
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModEnvironment {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub date_time: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub weather_preset: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModFlow {
    pub briefing_seconds: i64,
    pub safe_start_seconds: i64,
    pub time_limit_seconds: i64,
    pub jip: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModWinConditions {
    pub mode: String,
    pub end_on: Vec<String>,
}

// ---- T-202: per-faction briefings (prose + map markers) ----

/// One `briefing.markers[]` entry (`mission.schema.json#/$defs/marker`).
///
/// All four keys are in the schema's `required` and `additionalProperties` is `false`, so every
/// field is serialised UNCONDITIONALLY — no `skip_serializing_if` anywhere in this struct. An
/// emitter that omitted `label` because it happened to be empty would produce a document the
/// `/compiled` route then rejects, and `""` is explicitly legal content
/// (`golden-missions/empty-warning-fields.json` ships a marker with `icon` and `label` both empty).
///
/// `label` is deliberately NOT run through [`crate::mission::wire_safety`]. Markers do not ride a
/// delimited wire: `TBD_MarkerController.TBD_RpcDo_Markers` takes
/// `(array<int> xs, array<int> zs, array<string> icons, array<string> labels, …)` — four PARALLEL
/// arrays with no delimiter at all, chosen for that reason (`TBD_MarkerData.c` header, "Why
/// parallel arrays and not a delimited string"). Nothing in the five-file marker pipeline calls
/// `Join`, `Split`, `Sanitise` or `FIELD_MARK`. The schema agrees: `marker.label` is a plain
/// `{"type":"string"}`, NOT a `$ref` to `wireSafeString`, unlike the roster fields a TAB really
/// does break (T-181.42). Sanitising here would rewrite a value the author typed for no gain.
#[derive(Debug, Serialize)]
pub struct ModMarker {
    pub x: f64,
    pub z: f64,
    pub icon: String,
    /// Capped at [`MOD_MAX_MARKER_LABEL_CHARS`] here so the mod never has to — the same reason
    /// [`ModNet::label`] is capped: `TBD_MarkerService.CapLabel` truncates without telling anyone,
    /// and the compiled document a human can read should already show the string the player sees.
    pub label: String,
}

/// One `briefings` entry (`mission.schema.json#/$defs/briefing`), keyed by faction.
///
/// `briefing` declares NO `required`, so every field here is optional and an entry that carries
/// nothing serialises to `{}` — which is not a degenerate case but a shape the fixtures already
/// ship (`empty-warning-fields.json` gives `opfor` exactly `{}`) and which the mod treats as one
/// of three legal empty states, all rendering zero paragraphs (`TBD_BriefingData.BuildOrders`).
///
/// The three prose fields are `Option<String>` rather than `String` so "the author wrote nothing"
/// and "the author wrote an empty string" stay distinguishable in the emitted bytes. The mod
/// collapses them (`AppendParagraphs` does a CONTENT test, not a presence test), but the compiled
/// document is also read by humans and by the schema, and a key that is absent because nothing
/// authored it should not look like a key somebody deliberately blanked.
///
/// Prose is deliberately NOT sanitised. `wireSafeString` explicitly EXCLUDES
/// `briefing.situation`/`mission`/`execution` (`mission.schema.json` `$defs/wireSafeString`,
/// final paragraph) because prose does not ride a delimited wire either: `TBD_BriefingService`
/// ships it as three parallel `array<string>` RPC parameters, and the mod SPLITS on newlines to
/// get display paragraphs (`AppendParagraphs` → `SplitLines`). An embedded newline is therefore
/// legitimate authoring — a multi-paragraph situation report is the feature, not a hazard — and
/// stripping it here would silently merge an author's paragraphs into one wall of text.
#[derive(Debug, Default, Serialize)]
pub struct ModBriefing {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub situation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mission: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub markers: Vec<ModMarker>,
}

// ---- T-200: the kit substitutions this compile made ----

/// One character the author placed that `kit-aliases.json` has no row for, and the faction
/// default the compile used instead.
///
/// Deduped on `(asset_id, faction)` and not on the asset alone: the same prefab placed on two
/// sides resolves to two DIFFERENT faction defaults, so those are two distinct substitutions and
/// collapsing them would hide one of them behind the other's kit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KitSubstitution {
    /// The full Enfusion ResourceName the author placed — verbatim, because it is exactly the
    /// string a new `kit-aliases.json` row has to carry to make this stop happening.
    pub asset_id: String,
    /// The slugged faction key whose default was used.
    pub faction: String,
    /// The `kit:` alias that reached `slots[].kit` and `orbat.*.groups[].roles[].kit` instead.
    pub kit: String,
    /// Derived id (`faction:callsign:role:occurrence`) of the FIRST slot that hit this pair —
    /// the same string the compiled document carries in `slots[].id`, so a reader holding the
    /// document can find the seat this is talking about.
    pub example_slot_id: String,
    /// That slot's editor id, as carried to `slots[].uid`. Kept alongside `example_slot_id`
    /// because the derived id shifts under role renames/reorders/deletes and this one does not.
    pub example_slot_uid: String,
    /// How many slots in this compile resolved through this same pair.
    pub occurrences: usize,
}

/// Distinct `(assetId, faction)` pairs named before the report stops listing them. The same 20 as
/// [`crate::mission::wire_safety::MAX_REPORTED`] and the `/compiled` handler's finding cap,
/// because these lines land in the same places those do. Nothing is lost to the cap:
/// [`KitSubstitutionReport::slots`] counts EVERY substituted slot, so the tail line can say
/// exactly how many the list does not name.
const MAX_REPORTED_SUBSTITUTIONS: usize = 20;

/// What [`flatten_to_mod_document`] silently swallowed before T-200.
///
/// ── Why this is a report and not an error ────────────────────────────────────────────────────
/// **342 of the 354 `kind: "character"` rows** in `packages/tbd-schema/registry/
/// registry-items.workbench.json` have no `kit-aliases.json` row (measured 2026-07-26; 12 aliases,
/// 8 of them pre-T-183). The palette offers all 354. So the substitution is not a rare defect to
/// fail on — it is what happens to almost every character an author can place, and rejecting it
/// would make the editor's own asset browser mostly unusable. The compile is also not *wrong* to
/// substitute: `mission.schema.json` requires `slots[].kit` to match `^kit:[a-z0-9_]+$`, and the
/// faction default is the only value on hand that does. What was wrong is that nobody was told.
///
/// ── Why it does not ride the wire ───────────────────────────────────────────────────────────
/// `mission.schema.json` sets top-level `additionalProperties: false`, and
/// `validated_compiled_body` (`apps/website/api/src/handlers/missions.rs`) holds the SERIALIZED
/// document to that schema before serving it and answers **500** on any finding. A new top-level
/// key would therefore turn every `GET /missions/:id/compiled` into a 500 — so this rides the
/// returned Rust value with `#[serde(skip)]` instead, and the served bytes are byte-identical to
/// what they were before this ticket. Pinned by `substitutions_never_reach_the_compiled_wire`.
///
/// ── Why it hangs off the document rather than a second entry point ──────────────────────────
/// `flatten_to_mod_document`'s signature is the spine of three consumers (`/compiled`, the event
/// ORBAT derivation in `handlers/events.rs`, and the wasm client via
/// [`flatten_mod_document_json`]). A `-> Result<(Doc, Report), _>` would have changed all of them
/// and every test that calls it; a parallel `flatten_to_mod_document_with_report` would be a
/// second entry point to keep in step with the first. Riding the value that already comes back
/// means every existing caller ALREADY holds this — surfacing it is `doc.kit_substitutions` at
/// the call site, with no signature to renegotiate.
#[derive(Debug, Clone, Default)]
pub struct KitSubstitutionReport {
    rows: Vec<KitSubstitution>,
    slots: usize,
}

impl KitSubstitutionReport {
    /// True when every placed character resolved to its own kit.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.slots == 0
    }

    /// Every slot that got a faction default in place of its own kit — uncapped and NOT deduped,
    /// so this is the number of seats that will spawn as somebody else.
    #[must_use]
    pub fn slots(&self) -> usize {
        self.slots
    }

    /// The named substitutions, at most [`MAX_REPORTED_SUBSTITUTIONS`] of them.
    #[must_use]
    pub fn rows(&self) -> &[KitSubstitution] {
        &self.rows
    }

    /// One readable line per substitution, for a log line or an editor dialog — the shape
    /// `wire_safety::scan_editor_payload` returns, so a caller can render both the same way.
    ///
    /// Each line answers the three questions the silence left open: WHICH seat, WHAT was placed,
    /// and WHAT it became. The remedy (`kit-aliases.json` + `apps/mod/tbd-framework/Data/
    /// registry.json`, both sides or the T-181.36 gate fails closed) is not repeated on every
    /// line — it belongs in this module's docs, not twenty times in an operator's face.
    #[must_use]
    pub fn details(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .rows
            .iter()
            .map(|r| {
                let more = if r.occurrences > 1 {
                    format!(" (and {} more slot(s) on this side)", r.occurrences - 1)
                } else {
                    String::new()
                };
                format!(
                    "{}: \"{}\" has no kit-aliases.json row — compiled as the {} default \"{}\", \
                     so this seat spawns a different character than the one placed{}",
                    r.example_slot_id,
                    escape_resource_name(&r.asset_id),
                    r.faction,
                    r.kit,
                    more,
                )
            })
            .collect();

        // Deliberately counted in SLOTS, not in distinct assets. Naming distinct assets past the
        // cap would mean remembering every key that did not make the list — which is the one
        // allocation `SubstitutionAcc::record` exists to avoid on a 367k-slot mission. Slots is
        // both cheap and the number that matters: it is how many seats are affected.
        let unnamed = self.slots - self.rows.iter().map(|r| r.occurrences).sum::<usize>();
        if unnamed > 0 {
            out.push(format!(
                "+ {unnamed} further slot(s) were substituted under assets not named above"
            ));
        }
        out
    }
}

/// Accumulator for [`KitSubstitutionReport`], filled during the one existing slot walk.
///
/// A linear scan of `rows` rather than a `HashMap`, because `rows` is capped at
/// [`MAX_REPORTED_SUBSTITUTIONS`]: 20 `&str` comparisons that almost always fail inside the first
/// two bytes (the `{GUID}` prefix leads the ResourceName) beat allocating the two owned Strings a
/// `HashMap<(String, String), _>` lookup would need on EVERY substituted slot. That matters here
/// and not in `wire_safety`, because with 342 of 354 characters unaliased this is not the
/// exceptional path today — it is the common one.
#[derive(Default)]
struct SubstitutionAcc {
    rows: Vec<KitSubstitution>,
    slots: usize,
}

impl SubstitutionAcc {
    /// `slot_id` is a closure so the derived id is only formatted for the first slot of a new
    /// pair — the 300,000th slot carrying an unaliased asset costs one bounded scan and nothing
    /// else.
    fn record(
        &mut self,
        asset_id: &str,
        faction: &str,
        kit: &str,
        slot_uid: &str,
        slot_id: impl FnOnce() -> String,
    ) {
        self.slots += 1;
        if let Some(row) = self
            .rows
            .iter_mut()
            .find(|r| r.asset_id == asset_id && r.faction == faction)
        {
            row.occurrences += 1;
            return;
        }
        if self.rows.len() >= MAX_REPORTED_SUBSTITUTIONS {
            return;
        }
        self.rows.push(KitSubstitution {
            asset_id: asset_id.to_string(),
            faction: faction.to_string(),
            kit: kit.to_string(),
            example_slot_id: slot_id(),
            example_slot_uid: slot_uid.to_string(),
            occurrences: 1,
        });
    }

    fn finish(self) -> KitSubstitutionReport {
        KitSubstitutionReport {
            rows: self.rows,
            slots: self.slots,
        }
    }
}

/// Render a ResourceName into a line a human reads.
///
/// `assetId` is the one authored string the compile never copies into the document, so nothing
/// holds it to `mission.schema.json#/$defs/wireSafeString` and `wire_safety::scan_editor_payload`
/// does not scan it — a TAB in an imported payload arrives here intact and would silently shift
/// the columns of whatever log line this lands in.
///
/// Deliberately NOT `wire_safety::quote_value`: that elides at 60 characters and a vanilla
/// character ResourceName is ~84 (`{26A9756790131354}Prefabs/Characters/Factions/BLUFOR/US_Army/
/// Character_US_Rifleman.et`), so the elision would cut off the character name — the one part of
/// the string the reader is looking for. Nothing is truncated here; control characters are
/// escaped, and the definition of "control" is `wire_safety`'s, not a second copy of it.
fn escape_resource_name(s: &str) -> String {
    if !s.bytes().any(is_wire_unsafe) {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if (c as u32) < 0x80 && is_wire_unsafe(c as u8) {
            out.push_str(&format!("\\u{{{:02x}}}", c as u32));
        } else {
            out.push(c);
        }
    }
    out
}

/// The full compiled document served to the game server.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModMissionDocument {
    pub schema_version: String,
    pub meta: ModMeta,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<ModEnvironment>,
    pub factions: Vec<ModFaction>,
    /// `BTreeMap` → sorted keys, matching Go's map marshalling.
    pub orbat: BTreeMap<String, ModOrbatFaction>,
    pub slots: Vec<ModSlot>,
    /// T-203 — derived, never authored (nothing in the editor authors nets yet). `None`
    /// omits the key entirely; see [`derive_radio_plan`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radio_plan: Option<ModRadioPlan>,
    pub zones: Vec<ModZone>,
    pub flow: ModFlow,
    pub win_conditions: ModWinConditions,
    /// T-202 — per-faction orders + map markers, keyed by the SAME slugged faction key as
    /// [`Self::factions`]`[].key` and [`Self::orbat`]. Declared after `winConditions` to match the
    /// schema's own property order, and a `BTreeMap` for the same reason `orbat` is one.
    ///
    /// **PASSED THROUGH, never derived.** Unlike `radioPlan` (T-203) there is no honest derivation
    /// here: a radio net follows deterministically from the ORBAT, but orders are AUTHORED PROSE and
    /// a marker is an authored decision about what a side is told. Synthesising either would put
    /// words in the mission author's mouth and ship them to players as orders — the same reason
    /// `TBD_MarkerIcons` refuses to invent a colour policy ("inventing a colour policy the operator
    /// did not ask for would be a silent product decision").
    ///
    /// Empty → the key is omitted entirely, which is legal (`briefings` is not in the schema's
    /// top-level `required`) and is `TBD_BriefingData.BuildOrders`' documented empty state #1. Today
    /// it is ALWAYS empty, because no mutator writes `editor.factions[].briefing` yet — see
    /// [`derive_briefings`] for exactly what this reads and where the authoring gap is.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub briefings: BTreeMap<String, ModBriefing>,
    /// T-200 — **not part of the document.** `#[serde(skip)]`, so the served JSON is byte-identical
    /// to what it was before this field existed; the schema's top-level
    /// `additionalProperties: false` would 500 the whole `/compiled` route otherwise. This is what
    /// the compile SUBSTITUTED on its way to producing the document above — see
    /// [`KitSubstitutionReport`] for why it hangs here rather than on a second entry point.
    #[serde(skip)]
    pub kit_substitutions: KitSubstitutionReport,
}

/// Compile failure — mirrors `ErrNoSlots` + a payload-parse error.
#[derive(Debug, thiserror::Error)]
pub enum CompileError {
    #[error("mission version has no placed slots")]
    NoSlots,
    #[error("parse mission version payload: {0}")]
    Parse(String),
}

// ---- input payload (the editor graph the TS flatten walks) ----

#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
struct EditorPayload {
    editor: EditorGraph,
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
struct EditorGraph {
    factions: Vec<FactionIn>,
    squads: Vec<SquadIn>,
    slots: Vec<SlotIn>,
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct FactionIn {
    key: String,
    name: String,
    squad_ids: Vec<String>,
    /// T-202 — this side's authored orders and markers, hanging on the FACTION ROW rather than in a
    /// sibling map keyed by faction (T-214's shape, adopted here so the authoring and emitting halves
    /// read the same object).
    ///
    /// The row is the better home for a reason that outlives the convenience: the compiled
    /// `briefings` map is `additionalProperties`-open, so an entry naming a faction the author later
    /// DELETED still validates, and the compile would ship orders for a side that no longer exists.
    /// On the row that state is unrepresentable — delete the faction and its briefing goes with it.
    ///
    /// `Option` so "no briefing" and "an empty briefing" stay distinct; absent when unauthored.
    briefing: Option<BriefingIn>,
}

#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct SquadIn {
    id: String,
    callsign: String,
    name: String,
    slot_ids: Vec<String>,
}

#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct SlotIn {
    id: String,
    index: i64,
    role: String,
    asset_id: String,
    position: PositionIn,
    /// The editor `SlotLoadoutV2` dict (T-068.10/.15.2) — mapped by [`mod_slot_loadout`].
    loadout: Option<serde_json::Value>,
}

#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(default)]
struct PositionIn {
    x: f64,
    y: f64,
    z: f64,
    rotation: f64,
}

/// One authored `editor.factions[].briefing` (T-202). Mirrors `#/$defs/briefing`.
///
/// The three prose fields are `Option<String>` so an ABSENT key and an authored `""` stay
/// distinguishable all the way through to the emitted bytes. Both are legal and the mod renders
/// both as nothing, but they are different authorial acts and the compiled document should not
/// claim the author blanked a field they never opened.
///
/// `markers` is on this object and NOT on a separate authoring surface, which is the one place this
/// slice extends T-214's shape rather than merely consuming it — see [`derive_briefings`] §markers
/// for the reasoning and the open question it raises.
#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(default)]
struct BriefingIn {
    situation: Option<String>,
    mission: Option<String>,
    execution: Option<String>,
    markers: Vec<MarkerIn>,
}

/// One authored `briefing.markers[]` row (T-202). Mirrors `#/$defs/marker`.
///
/// Every field defaults, so a row missing `icon` or `label` still deserialises and is emitted with
/// the empty string rather than dropped. That direction is deliberate: `x`/`z` are the part a reader
/// cannot reconstruct, an empty `icon`/`label` is schema-legal content the fixtures already ship,
/// and `TBD_MarkerIcons.Resolve` handles an unknown or empty icon by drawing the fallback dot and
/// logging once — never by losing the marker. Dropping the row instead would delete an authored
/// position to avoid emitting an empty caption.
#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(default)]
struct MarkerIn {
    x: f64,
    z: f64,
    icon: String,
    label: String,
}

/// Mission-level metadata the flatten needs. The backend builds this from its `Mission` sqlx
/// model; the wasm client passes it as JSON (camelCase). Decouples the core compiler from any
/// backend type (T-145 Phase 2b). `terrain`/`weather_preset` are already the `as_str()` values.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MissionMeta {
    pub id: String,
    pub title: String,
    pub author: String,
    pub terrain: String,
    pub custom_terrain_name: String,
    pub max_players: i64,
    pub time_of_day: String,
    pub weather_preset: String,
}

const COMPILE_DATE_ANCHOR: &str = "1989-06-14";
const SPAWN_ZONE_RADIUS_M: f64 = 150.0;

/// `mission.schema.json#/$defs/meta/name` — `maxLength: 120`.
const META_NAME_MAX_CHARS: usize = 120;

/// Stand-in for a slot the author never gave a role. The schema demands
/// `minLength: 1` on both `slots[].role` and `orbat.*.groups[].roles[].slot`;
/// the editor does not require the field, so the compile must supply something
/// rather than emit a document we would then reject (T-181.31).
const ROLE_FALLBACK: &str = "unassigned";

/// Stand-in for a squad with neither `callsign` nor `name`. Only reached when the
/// squad also has no id, because the id is preferred — two unnamed squads must not
/// collapse onto one callsign, or their derived slot ids collide and the mod's
/// duplicate-id check (a hard error there) rejects the whole document.
const CALLSIGN_FALLBACK: &str = "squad";

/// `TBD_RadioPlan.MAX_NETS` (`apps/mod/tbd-framework/.../Radio/TBD_RadioPlan.c:91`).
/// **The schema states no `maxItems` on `radioPlan.nets`** — this limit exists only in the
/// mod, which accepts the first 32 nets in DOCUMENT ORDER and drops the rest. It is
/// mirrored here so the cut is made by the side that can make it fairly: see
/// [`derive_radio_plan`] for why document order is load-bearing.
const MOD_MAX_NETS: usize = 32;

/// `TBD_RadioPlan.MAX_LABEL_CHARS` (`TBD_RadioPlan.c:94`). Again mod-only — the schema puts
/// no `maxLength` on `net.label`. `TBD_RadioPlan.CapLabel` truncates past it without a word
/// to anyone, so the truncation is done here instead, where the compiled document a human
/// can read already shows the string the player will see.
const MOD_MAX_LABEL_CHARS: usize = 48;

/// `TBD_MarkerService.MAX_LABEL_CHARS` (`Markers/TBD_MarkerData.c:63`). A DIFFERENT consumer with
/// a different budget from [`MOD_MAX_LABEL_CHARS`] — the radio plan's 48 is `TBD_RadioPlan`'s, this
/// 64 is the marker wire's — so the two are deliberately separate constants rather than one shared
/// number that would silently retune whichever mod class changed second.
///
/// Same rationale as the net label: `TBD_MarkerService.CapLabel` truncates without telling anyone,
/// so the cut is made here where the compiled document already shows the caption the player sees.
///
/// Truncated on a CHAR boundary, not a byte one, matching [`cap_net_label`] and `meta.name`. The
/// mod's own cut is by BYTES (`string.Length()` counts bytes on this engine — recorded landmine in
/// `$defs/wireSafeString`), so on a multi-byte label the mod may shorten a little further. That is
/// safe in the one direction that matters: both ends only ever SHORTEN a display string, and a
/// char-boundary cut here cannot hand the mod invalid UTF-8.
const MOD_MAX_MARKER_LABEL_CHARS: usize = 64;

/// Bottom of `mission.schema.json#/$defs/net/freqMHz` (`minimum: 30`) and the base of the
/// net frequency allocation. Deliberately the schema's own floor and not a number lifted
/// from a golden mission — see [`derive_radio_plan`].
const NET_FREQ_BASE_MHZ: f64 = 30.0;

/// Spacing between allocated nets, in MHz. 0.5 MHz is exactly representable in binary
/// floating point (so `base + step * i` is exact, and two compiles of one mission cannot
/// drift), and it is a whole multiple of any transceiver frequency resolution the engine
/// is likely to report — `TBD_RadioTuner.Constrain` rounds a requested frequency to
/// `BaseTransceiver.GetFrequencyResolution()`, and a spacing finer than that step would
/// round two distinct nets onto one frequency and silently merge two channels.
const NET_FREQ_STEP_MHZ: f64 = 0.5;

/// The schema's `minLength: 1` string fields cannot take the empty string, and the
/// editor does not guarantee these are set. Substitute rather than emit a document
/// that fails our own contract.
fn or_fallback<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    if value.is_empty() { fallback } else { value }
}

/// Lowercase into the schema's `^[a-z][a-z0-9_]*$` pattern.
fn slug_key(raw: &str, fallback: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut prev_repl = false;
    for c in raw.to_lowercase().chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' {
            out.push(c);
            prev_repl = false;
        } else if !prev_repl {
            out.push('_');
            prev_repl = true;
        }
    }
    let trimmed = out.trim_matches('_');
    if trimmed.is_empty() {
        return fallback.to_string();
    }
    match trimmed.chars().next() {
        Some(c) if c.is_ascii_lowercase() => trimmed.to_string(),
        _ => format!("f_{trimmed}"),
    }
}

/// The compiled document's `meta.terrain` — the ONE definition, because the mod
/// routes worlds on it. `TBD_FrameworkManager.SelectMissionByNumber` compares the
/// mission-list entry's `terrain` against the loaded document's `meta.terrain` and
/// feeds it to `TBD_ScenarioRouter.GetScenarioForTerrain`; a list that said
/// `"Everon"` where the document says `"everon"` would restart the scenario it was
/// already on, or fail to find one at all. `GET /api/v1/ingest/missions` therefore
/// calls THIS rather than re-deriving the slug (T-181.51).
pub fn mission_terrain_key(terrain: &str, custom_terrain_name: &str) -> String {
    let raw = if terrain == "custom" && !custom_terrain_name.is_empty() {
        custom_terrain_name
    } else {
        terrain
    };
    slug_key(raw, "everon")
}

/// Reduce the mission UUID to the schema's `^msn_[a-z0-9]+$` id space.
fn mission_doc_id(id: &str) -> String {
    let hex: String = id
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        .collect();
    format!("msn_{}", if hex.is_empty() { "editor" } else { &hex })
}

/// Map an editor `SlotLoadoutV2` dict onto the compiled loadout block. Empty
/// strings and malformed cargo rows drop (the editor tolerance); an all-empty
/// result returns `None` so the whole `loadout` key is omitted. Gear derivation
/// is the locked loadout-export rule: jacket→uniform, armoredVest else
/// vest→vest, headCover→helmet; and, since T-182, ALL FOUR authored weapon slots
/// by `(slotIndex, slotType)` — `(0,primary)`→primary (+optic/magazine),
/// `(1,primary)`→launcher, `(2,secondary)`→handgun, `(3,grenade)`→throwable.
/// Before T-182 only `(0,primary)` was selected, so a player authored with a
/// launcher, a sidearm or a grenade spawned without it.
fn mod_slot_loadout(lo: &serde_json::Value) -> Option<ModSlotLoadout> {
    let non_empty = |v: Option<&serde_json::Value>| {
        v.and_then(serde_json::Value::as_str)
            .filter(|s| !s.is_empty())
            .map(String::from)
    };
    let wear = lo.get("wear");
    let wear_key = |k: &str| non_empty(wear.and_then(|w| w.get(k)));

    let mut gear = ModSlotGear {
        uniform: wear_key("jacket"),
        vest: wear_key("armoredVest").or_else(|| wear_key("vest")),
        helmet: wear_key("headCover"),
        pants: wear_key("pants"),
        boots: wear_key("boots"),
        handwear: wear_key("handwear"),
        backpack: wear_key("backpack"),
        ..ModSlotGear::default()
    };
    // T-182 — select ALL FOUR authored weapon slots, each by its exact (slotIndex, slotType) pair.
    // This used to match only (0, "primary") and silently drop the rest, so a slot authored with a
    // launcher, a sidearm and a grenade spawned carrying none of them. The pairs are the editor's
    // own table — keep byte-identical to `arsenal_rules.rs` `WEAPON_SLOTS`. Matching on the PAIR
    // rather than the index alone matters: slots 0 and 1 are both slotType "primary" (two untyped
    // long slots), so the index is what separates rifle from launcher, while slotType is what
    // stops a mis-authored row landing in the wrong key.
    let weapons = lo.get("weapons").and_then(serde_json::Value::as_array);
    let weapon_at = |slot_index: i64, slot_type: &'static str| {
        weapons.and_then(|ws| {
            ws.iter().find(|w| {
                w.get("slotIndex").and_then(serde_json::Value::as_i64) == Some(slot_index)
                    && w.get("slotType").and_then(serde_json::Value::as_str) == Some(slot_type)
            })
        })
    };

    if let Some(primary) = weapon_at(0, "primary") {
        gear.primary = non_empty(primary.get("weapon"));
        // optic/magazine exist on the primary rifle alone — the other three slots have no
        // sub-slots in the editor, so nothing is being dropped by not reading them there.
        gear.optic = non_empty(primary.get("optic"));
        gear.magazine = non_empty(primary.get("magazine"));
    }
    gear.launcher = weapon_at(1, "primary").and_then(|w| non_empty(w.get("weapon")));
    gear.handgun = weapon_at(2, "secondary").and_then(|w| non_empty(w.get("weapon")));
    gear.throwable = weapon_at(3, "grenade").and_then(|w| non_empty(w.get("weapon")));

    let cargo: Vec<ModSlotCargo> = lo
        .get("cargo")
        .and_then(serde_json::Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|r| {
                    Some(ModSlotCargo {
                        container: non_empty(r.get("container"))?,
                        item: non_empty(r.get("item"))?,
                        qty: r
                            .get("qty")
                            .and_then(serde_json::Value::as_i64)
                            .filter(|q| *q >= 1)?,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let gear = (!gear.is_empty()).then_some(gear);
    if gear.is_none() && cargo.is_empty() {
        return None;
    }
    Some(ModSlotLoadout { gear, cargo })
}

/// T-202 — the `briefings` block: per-faction orders + map markers, the ONE thing two shipped mod
/// subsystems read (`TBD_BriefingData.BuildOrders`, and the five-file marker pipeline through
/// `TBD_MarkerService.Build`).
///
/// ## Where the authored side lives
///
/// Read from `editor.factions[].briefing` — the FACTION ROW — which is the shape T-214 writes. Two
/// halves of one contract, so they read and write the same object rather than two invented ones.
///
/// The row beats a sibling `briefings` map on a point that outlives convenience: the compiled map is
/// `additionalProperties`-open, so an entry naming a faction the author later DELETED still
/// validates, and the compile would ship orders to a side that no longer exists. Hanging prose on
/// the row makes that state unrepresentable.
///
/// ## The key is slugged, and that is load-bearing
///
/// The mod resolves a reader's orders with `GetBriefingForFaction(slot.faction)`, where
/// `slot.faction` is the COMPILED slot's faction — i.e. [`slug_key`]'s output. So the map key must go
/// through the same slug or the lookup misses and the side silently gets no orders, which is
/// indistinguishable from "this mission authored none". `editor.factions[].key` is `BLUFOR` in every
/// fixture in the tree, so the unslugged form would miss on every mission that has ever been saved.
/// Slugging also guarantees `#/$defs/factionKey` (`^[a-z][a-z0-9_]*$`), which the raw string does not.
///
/// ## §markers — the one place this extends T-214's shape
///
/// T-214's faction-row `briefing` carries the three PROSE fields only. `briefing.markers` is the
/// other half of this ticket (the five-file replicated marker pipeline reads nothing else), so it is
/// read from the same per-faction object here.
///
/// That placement is not arbitrary: markers are SIDE-SCOPED INTELLIGENCE — `TBD_MarkerService.Build`
/// resolves the reader's side server-side and only ever puts that side's rows in the arrays it sends,
/// because `bridgehead-at-levie.json` gives blufor and opfor different orders at the SAME
/// coordinates. Per-faction is the only correct scope, and the faction row is per-faction by
/// construction. It also inherits the deleted-faction property above.
///
/// **This is the field to confirm against T-214's report before either half lands** — it is the sole
/// respect in which the two shapes differ, and nothing authors it yet either way.
///
/// ## What is NOT done here, and why
///
/// **No sanitising, of prose or of labels.** Neither rides a delimited wire — see [`ModBriefing`]
/// and [`ModMarker`] for the per-field evidence. This is the one authored-string family in the
/// document that [`crate::mission::wire_safety`] deliberately does not cover, and the schema says so
/// itself by not `$ref`-ing `wireSafeString` on any of the four fields.
///
/// **No filtering against the compiled factions.** A briefing authored for a side that ended up with
/// no slots is kept. The mod simply never looks it up (map-lookup miss → legal empty state), whereas
/// dropping it would silently delete authored prose — the "no silent repair" rule
/// [`crate::mission::wire_safety`] states for exactly this class of value.
///
/// **No marker-count cap.** The mod caps at `TBD_MarkerService.MAX_MARKERS` (64) and WARNS with the
/// authored total, which is a diagnostic aimed at the author; cutting the list here would suppress
/// that warning and lose markers with nobody told. Contrast the LABEL cap, which only shortens a
/// caption and is applied here so the compiled document shows what the player will read.
///
/// ## Today this returns an empty map on every payload in existence
///
/// Not a stub — the mapping below is complete and pinned by tests against a committed golden — but
/// its INPUT is unauthored. `doc/store.rs` has no `set_faction_briefing` mutator yet (T-214 proved
/// the read and hydrate halves round-trip a per-faction briefing with no `store.rs` change, but the
/// writer is a separate outstanding handoff), and the editor's Markers tab is still the literal stub
/// "Marker placement lands in T-069.". An empty map omits the key, so the compiled bytes stay
/// byte-identical to the pre-T-202 document and no golden needs regenerating.
///
/// The emitter is the half that can land independently, and it is the half that was missing: once a
/// mutator writes this object, both mod subsystems light up with no further compiler change.
fn derive_briefings(factions: &[FactionIn]) -> BTreeMap<String, ModBriefing> {
    let mut out: BTreeMap<String, ModBriefing> = BTreeMap::new();

    for f in factions {
        let Some(briefing) = f.briefing.as_ref() else {
            continue; // this side authored no orders. Legal, and the common case.
        };

        // Two rows can slug onto one faction (`BLUFOR` and `blufor`), which the editor does not
        // prevent. Merge rather than let document order decide which half of the author's orders
        // survives — silently discarding authored prose is the failure mode `wire_safety`'s
        // "no silent repair" rule exists to prevent.
        let entry = out.entry(slug_key(&f.key, "faction")).or_default();

        merge_prose(&mut entry.situation, briefing.situation.as_deref());
        merge_prose(&mut entry.mission, briefing.mission.as_deref());
        merge_prose(&mut entry.execution, briefing.execution.as_deref());

        for m in &briefing.markers {
            entry.markers.push(ModMarker {
                x: m.x,
                z: m.z,
                icon: m.icon.clone(),
                label: m.label.chars().take(MOD_MAX_MARKER_LABEL_CHARS).collect(),
            });
        }
    }

    out
}

/// Fold one authored prose field into a possibly-already-populated slot (the slug-collision case).
///
/// Joined with a blank line, because that is what the mod reads as a paragraph break
/// (`TBD_BriefingData.AppendParagraphs` → `SplitLines`, dropping blank parts) — so a merge reads as
/// two paragraphs rather than as two sentences run together.
fn merge_prose(slot: &mut Option<String>, authored: Option<&str>) {
    let Some(text) = authored else {
        return;
    };
    match slot {
        // Preserve an authored empty string as an authored empty string: `Some("")` is a different
        // fact from `None`, and only a non-empty addition is worth a separator.
        Some(existing) if !existing.is_empty() && !text.is_empty() => {
            existing.push_str("\n\n");
            existing.push_str(text);
        }
        Some(existing) if existing.is_empty() => *existing = text.to_string(),
        Some(_) => {}
        None => *slot = Some(text.to_string()),
    }
}

/// One side's contribution to the radio plan, harvested from the ORBAT as it is built.
/// `callsigns` are the group callsigns in document order — the same strings that reach
/// `orbat.*.groups[].callsign` and `slots[].groupCallsign`, so a net cannot name a squad
/// the compiled document does not contain.
struct RadioNetSource {
    faction_key: String,
    display_name: String,
    callsigns: Vec<String>,
}

/// Allocate a net id nothing else in this document has taken.
///
/// `^net:[a-z0-9_]+$` is a smaller alphabet than the callsigns feeding it, so two distinct
/// squads CAN slug to one id ("Alpha 1" and "Alpha-1" both reduce to `alpha_1`). The
/// numeric suffix is retried rather than assumed free, because `_2` can itself be an
/// authored callsign — a squad literally called "Alpha 2" collides with the disambiguator
/// for a duplicate "Alpha". Ids are the mod's stable channel key and the VOIP bridge's
/// voice-channel key; two nets sharing one is two channels sharing one.
fn unique_net_id(used: &mut HashSet<String>, faction_key: &str, source: &str) -> String {
    let base = format!("net:{faction_key}_{}", slug_key(source, "net"));
    if used.insert(base.clone()) {
        return base;
    }
    for n in 2usize.. {
        let candidate = format!("{base}_{n}");
        if used.insert(candidate.clone()) {
            return candidate;
        }
    }
    unreachable!("the suffix search terminates — some n is always free")
}

/// `TBD_RadioPlan.CapLabel` in the compiler, on a char boundary.
fn cap_net_label(label: &str) -> String {
    label.chars().take(MOD_MAX_LABEL_CHARS).collect()
}

/// Derive `radioPlan.nets[]` from the ORBAT this compile just built (T-203).
///
/// ── Where this comes from, since nothing authors it ──────────────────────────────────
/// The editor has no radio UI: there is no `radioPlan` anywhere in the editor payload, in
/// `mission-editor-payload.schema.json`, or in the document core. So this is DERIVED, and
/// the only honest thing to derive it from is the structure the compile already knows —
/// factions and their squads. That is not a shape invented here: it is the shape every
/// committed golden mission authors by hand (`bridgehead-at-levie.json`,
/// `last-stand-at-montfort.json`, `slot-loadout-coverage.json` — one command net per side
/// plus one net per squad), and the shape `docs/mod/tbd-reforger-platform-build-plan.md`
/// §C2 describes ("a squad leader spawns already tuned to `cmd` + own squad net"). When the
/// editor learns to author nets, an authored plan replaces this whole function; the seam is
/// the single `derive_radio_plan(...)` call in [`flatten_to_mod_document`].
///
/// ── The frequencies are an ALLOCATION, not a doctrine ────────────────────────────────
/// Nothing in this repo can tell the compiler what frequency a net should be on, so it does
/// not pretend to know. Nets are numbered off the schema's own floor (30 MHz) in 0.5 MHz
/// steps, in emission order. Two properties are all that is claimed for them: they are
/// DETERMINISTIC (`/missions/:id/compiled` is re-fetched by the game server and must not
/// change under it) and they are DISTINCT (two sides on one frequency would hear each
/// other, which is the one way a frequency choice can be actively wrong).
///
/// The goldens' numbers were deliberately NOT copied. They read like doctrine — 41.0 for
/// BLUFOR command, 51.0 for OPFOR — and reproducing them on every compiled mission would
/// publish a frequency plan this program never agreed to. The community's own written
/// practice is the opposite of a fixed plan: the doctrine wiki's `comms-dynamic` entry
/// ("Operating With Looted Radios") says frequencies are randomized each match and treated
/// as throwaway. A mechanical allocation is the honest reading of that; a fixed table is not.
///
/// ── Document order is load-bearing, because the mod truncates on it ──────────────────
/// `TBD_RadioPlan.Parse` accepts the first [`MOD_MAX_NETS`] nets **across all factions**
/// and warns about the rest. So a naive faction-major emission on a mission where the first
/// side has 32+ squads would hand the second side ZERO nets — including its command net —
/// and the only trace would be one truncation line in the server log. Two rules prevent it:
///   1. every side's command net is emitted BEFORE any squad net, so no side can be
///      silenced by another side's squad count;
///   2. squad nets are then taken round-robin across sides, so the cut falls evenly.
///
/// The cut is also made HERE rather than left to the mod, so the compiled document a human
/// reads is the plan the server actually runs.
///
/// ── `range` ─────────────────────────────────────────────────────────────────────────
/// The schema advertises `short | long | any`, but only `long` does anything today:
/// `TBD_RadioService.LongRangeFlag` (`TBD_RadioService.c:214-220`) returns 1 for `"long"`
/// and 0 for everything else, so `short`, `any` and absent are one behaviour — pick the
/// handheld. Command nets are therefore marked `long` (the one value that changes what the
/// tuner does: it asks for the backpack set) and squad nets omit `range` entirely rather
/// than say `"short"`, which would look like a distinction the mod does not make. If the
/// mod ever starts honouring `short`, this is the line to revisit — not a comment to soften.
///
/// ── Every net is side-scoped ────────────────────────────────────────────────────────
/// `faction` is always set. The schema makes it optional and the mod reads an empty faction
/// as "shared with everybody" (`TBD_RadioPlan.GetNetsForFaction`), but nothing in the editor
/// expresses "common channel", so emitting one would be handing both sides a frequency on a
/// guess. Every faction key here is one the document declares, which is also what
/// `TBD_RadioPlan.Fault` cross-checks before serving a net to anyone.
fn derive_radio_plan(sources: &[RadioNetSource]) -> Option<ModRadioPlan> {
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

    // Rule 1 — every side's command net first. `display_name` is never empty (flatten falls
    // back to the faction key), so the label can never be the bare " Command" the mod would
    // still accept but a player would read as a blank channel.
    for src in sources.iter().take(MOD_MAX_NETS) {
        let label = format!("{} Command", src.display_name);
        push(&mut nets, src, "cmd", &label, true);
    }

    // Rule 2 — squad nets, round-robin by rank across sides.
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

fn normalize_heading(rotation: f64) -> f64 {
    if rotation.is_nan() || rotation.is_infinite() {
        return 0.0;
    }
    (rotation % 360.0 + 360.0) % 360.0
}

/// Build the compiled mod mission document. Fields the editor never authors (zones,
/// flow, winConditions, templateId, playerRange, presetId) are synthesized with the
/// same defaults as `flattenModDocument.ts`. Returns [`CompileError::NoSlots`] when
/// the editor graph holds no placed slots.
pub fn flatten_to_mod_document(
    mission: &MissionMeta,
    payload: &[u8],
) -> Result<ModMissionDocument, CompileError> {
    let aliases = load_kit_aliases();
    let parsed: EditorPayload =
        serde_json::from_slice(payload).map_err(|e| CompileError::Parse(e.to_string()))?;
    let ed = parsed.editor;

    let squads_by_id: HashMap<&str, &SquadIn> =
        ed.squads.iter().map(|s| (s.id.as_str(), s)).collect();
    let slots_by_id: HashMap<&str, &SlotIn> = ed.slots.iter().map(|s| (s.id.as_str(), s)).collect();

    let mut factions: Vec<ModFaction> = Vec::new();
    let mut orbat: BTreeMap<String, ModOrbatFaction> = BTreeMap::new();
    let mut doc_slots: Vec<ModSlot> = Vec::new();
    let mut centroids: HashMap<String, (f64, f64, i64)> = HashMap::new();
    let mut centroid_order: Vec<String> = Vec::new();
    let mut radio_sources: Vec<RadioNetSource> = Vec::new();
    let mut substitutions = SubstitutionAcc::default();
    let mut any_y = false;

    for f in &ed.factions {
        let faction_key = slug_key(&f.key, "faction");
        let (default_kit, preset) = aliases.faction_default(&faction_key);
        let mut groups: Vec<ModOrbatGroup> = Vec::new();

        for squad_id in &f.squad_ids {
            let Some(sq) = squads_by_id.get(squad_id.as_str()) else {
                continue;
            };
            let mut rows: Vec<&SlotIn> = sq
                .slot_ids
                .iter()
                .filter_map(|id| slots_by_id.get(id.as_str()).copied())
                .collect();
            if rows.is_empty() {
                continue;
            }
            rows.sort_by_key(|s| s.index); // stable

            // callsign → name → squad id → literal. The id rung keeps two unnamed
            // squads distinct so their derived slot ids stay unique.
            let callsign = if sq.callsign.is_empty() {
                or_fallback(or_fallback(&sq.name, &sq.id), CALLSIGN_FALLBACK).to_string()
            } else {
                sq.callsign.clone()
            };

            let mut role_counters: HashMap<&str, i64> = HashMap::new();
            let mut role_index: HashMap<&str, usize> = HashMap::new();
            let mut roles: Vec<ModOrbatRole> = Vec::new();

            for sl in &rows {
                let role = or_fallback(&sl.role, ROLE_FALLBACK);
                let occurrence = *role_counters.get(role).unwrap_or(&0);
                role_counters.insert(role, occurrence + 1);

                // T-200 — the substitution that used to happen in silence. `map_or_else` here read
                // as a tidy default; what it actually did was throw away the author's choice of
                // character and tell nobody. It still substitutes (see `KitSubstitutionReport` for
                // why erroring is not an option with 342 of 354 characters unaliased) — it just
                // records what it substituted.
                //
                // An EMPTY `assetId` is deliberately not recorded. A slot with no asset expressed
                // no preference: ORBAT templates and the `+` button both mint slots that way, and
                // for those the faction default is the correct answer, not a swap. Reporting them
                // would bury the real finding under one line per templated seat. This is the same
                // rule `wire_safety` applies to a blank callsign — a value the compile would have
                // substituted anyway is not a finding.
                let kit = match aliases.kit_for_resource(&sl.asset_id) {
                    Some(alias) => alias.to_string(),
                    None => {
                        if !sl.asset_id.is_empty() {
                            substitutions.record(
                                &sl.asset_id,
                                &faction_key,
                                default_kit,
                                &sl.id,
                                || format!("{faction_key}:{callsign}:{role}:{occurrence}"),
                            );
                        }
                        default_kit.to_string()
                    }
                };

                if let Some(&idx) = role_index.get(role) {
                    roles[idx].count += 1;
                } else {
                    role_index.insert(role, roles.len());
                    roles.push(ModOrbatRole {
                        slot: role.to_string(),
                        kit: kit.clone(),
                        count: 1,
                    });
                }

                let x = sl.position.x;
                let z = sl.position.y; // editor y (map north) → mod z
                let elev = sl.position.z; // editor z (elevation) → mod y (optional)
                let y = if elev != 0.0 && !elev.is_nan() && !elev.is_infinite() {
                    any_y = true;
                    Some(elev)
                } else {
                    None
                };

                doc_slots.push(ModSlot {
                    id: format!("{faction_key}:{callsign}:{role}:{occurrence}"),
                    uid: sl.id.clone(),
                    faction: faction_key.clone(),
                    group_callsign: callsign.clone(),
                    role: role.to_string(),
                    kit,
                    x,
                    z,
                    y,
                    heading_deg: normalize_heading(sl.position.rotation),
                    loadout: sl.loadout.as_ref().and_then(mod_slot_loadout),
                });

                if !centroids.contains_key(&faction_key) {
                    centroids.insert(faction_key.clone(), (0.0, 0.0, 0));
                    centroid_order.push(faction_key.clone());
                }
                let c = centroids.get_mut(&faction_key).expect("inserted");
                c.0 += x;
                c.1 += z;
                c.2 += 1;
            }

            groups.push(ModOrbatGroup {
                callsign,
                kind: "rifle_squad".to_string(),
                roles,
            });
        }

        let display_name = if f.name.is_empty() {
            faction_key.clone()
        } else {
            f.name.clone()
        };

        if !groups.is_empty() {
            // T-203 — harvested BEFORE `groups` moves into the orbat, and only for a faction
            // that actually holds seats: the stub faction padded in below has no squads and no
            // players, so giving it frequencies would put nets in the document that nobody can
            // ever be served.
            radio_sources.push(RadioNetSource {
                faction_key: faction_key.clone(),
                display_name: display_name.clone(),
                callsigns: groups.iter().map(|g| g.callsign.clone()).collect(),
            });
            orbat.insert(faction_key.clone(), ModOrbatFaction { groups });
        }
        factions.push(ModFaction {
            key: faction_key,
            display_name,
            preset_id: preset.to_string(),
            tickets: 0,
        });
    }

    if doc_slots.is_empty() {
        return Err(CompileError::NoSlots);
    }

    let schema_version = if any_y { "1.2" } else { "1.1" }.to_string();

    // Schema requires ≥ 2 factions; pad a stub opposing faction for single-faction drafts.
    if factions.len() < 2 {
        let mut stub = "opfor";
        for f in &factions {
            if f.key == "opfor" {
                stub = "blufor";
            }
        }
        let (_, preset) = aliases.faction_default(stub);
        factions.push(ModFaction {
            key: stub.to_string(),
            display_name: stub.to_uppercase(),
            preset_id: preset.to_string(),
            tickets: 0,
        });
    }

    let mut zones: Vec<ModZone> = Vec::new();
    for faction_key in &centroid_order {
        let (sx, sz, n) = centroids[faction_key];
        let nf = n as f64;
        zones.push(ModZone {
            id: format!("z_spawn_{faction_key}"),
            kind: "spawn".to_string(),
            faction: faction_key.clone(),
            shape: ModZoneShape {
                circle: ModCircle {
                    x: (sx / nf * 10.0).round() / 10.0,
                    z: (sz / nf * 10.0).round() / 10.0,
                    r: SPAWN_ZONE_RADIUS_M,
                },
            },
        });
    }

    // `faction_eliminated` is only declared when at least two factions actually HOLD SLOTS. The
    // mod's validator rejects the document outright otherwise ("declares faction_eliminated but
    // only 1 faction(s) actually have slots — no second side can ever be eliminated"), and since
    // the editor never authors winConditions, an unconditional default made EVERY single-faction
    // mission unloadable with no way for the author to fix it. Counted over the FLATTENED SLOTS
    // rather than `factions`, because a faction can be declared with no seats — which is exactly
    // the case that triggered this (an operator's live mission declared opfor with zero slots).
    // Computed here rather than inline below because the struct literal moves `doc_slots`.
    let end_on = {
        let mut sides: Vec<&str> = doc_slots.iter().map(|s| s.faction.as_str()).collect();
        sides.sort_unstable();
        sides.dedup();
        let mut triggers = vec!["time_limit".to_string()];
        if sides.len() >= 2 {
            triggers.push("faction_eliminated".to_string());
        }
        triggers
    };

    let max_players = if mission.max_players < 1 {
        (doc_slots.len() as i64).max(1)
    } else {
        mission.max_players
    };

    let terrain = mission_terrain_key(&mission.terrain, &mission.custom_terrain_name);

    let meta = ModMeta {
        id: mission_doc_id(&mission.id),
        // maxLength is counted in characters, not bytes — truncate on a char boundary.
        name: if mission.title.is_empty() {
            "Untitled Mission".to_string()
        } else {
            mission.title.chars().take(META_NAME_MAX_CHARS).collect()
        },
        author: mission.author.clone(),
        terrain,
        template_id: "editor_v1".to_string(),
        player_range: [1, max_players],
    };

    let mut environment = ModEnvironment {
        date_time: String::new(),
        weather_preset: mission.weather_preset.clone(),
    };
    if !mission.time_of_day.is_empty() {
        // time_of_day may be HH:MM or HH:MM:SS — keep exactly HH:MM.
        let t = if mission.time_of_day.len() > 5 {
            &mission.time_of_day[..5]
        } else {
            &mission.time_of_day
        };
        environment.date_time = format!("{COMPILE_DATE_ANCHOR}T{t}:00Z");
    }

    Ok(ModMissionDocument {
        schema_version,
        meta,
        environment: Some(environment),
        factions,
        orbat,
        slots: doc_slots,
        radio_plan: derive_radio_plan(&radio_sources),
        zones,
        flow: ModFlow {
            briefing_seconds: 600,
            safe_start_seconds: 300,
            time_limit_seconds: 5400,
            jip: "until_safestart_end".to_string(),
        },
        win_conditions: ModWinConditions {
            mode: "attrition".to_string(),
            // `faction_eliminated` is only declared when at least two factions actually HOLD
            // SLOTS. The mod's validator rejects the document outright otherwise ("declares
            // faction_eliminated but only 1 faction(s) actually have slots — no second side can
            // ever be eliminated"), and since the editor never authors winConditions, an
            // unconditional default made EVERY single-faction mission unloadable with no way for
            // the author to fix it. Counted over the flattened slots rather than `factions`,
            // because a faction can be declared with no seats — which is exactly the case that
            // triggered this.
            end_on,
        },
        briefings: derive_briefings(&ed.factions),
        kit_substitutions: substitutions.finish(),
    })
}

/// JSON-in / JSON-out flatten for the wasm client: `meta_json` (camelCase [`MissionMeta`]) + the
/// stored version `payload` → the compiled mod-document JSON bytes. Keeps serde_json on the core
/// side so the wasm shim stays dependency-thin.
///
/// These bytes are the editor's **Export** download and must satisfy `mission.schema.json` on
/// their own, so [`ModMissionDocument::kit_substitutions`] does NOT appear in them — it is
/// `#[serde(skip)]` and this function returns only the serialized document. A caller that wants
/// the substitutions in the browser needs [`flatten_to_mod_document`] and a shim export of its
/// own; adding a second key here would put a non-schema field in a file the mod loads.
///
/// # Errors
/// Returns a message on meta/payload parse failure or a compile error (e.g. no slots).
pub fn flatten_mod_document_json(meta_json: &[u8], payload: &[u8]) -> Result<Vec<u8>, String> {
    let meta: MissionMeta = serde_json::from_slice(meta_json).map_err(|e| e.to_string())?;
    let doc = flatten_to_mod_document(&meta, payload).map_err(|e| e.to_string())?;
    serde_json::to_vec(&doc).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Two factions, callsigned squads, a duplicate role (TL x2), one slot with real elevation.
    const FIXTURE: &str = r#"{
      "schemaVersion": 1,
      "map": {"terrain": "everon", "bounds": [0, 0, 12800, 12800]},
      "editor": {
        "factions": [
          {"id": "f1", "key": "BLUFOR", "name": "US Army", "squadIds": ["sq1"]},
          {"id": "f2", "key": "OPFOR", "name": "Soviet VDV", "squadIds": ["sq2"]}
        ],
        "squads": [
          {"id": "sq1", "factionId": "f1", "callsign": "Alpha", "name": "Alpha 1-1", "slotIds": ["s1", "s2", "s3"]},
          {"id": "sq2", "factionId": "f2", "name": "Grom", "slotIds": ["s4"]}
        ],
        "slots": [
          {"id": "s1", "squadId": "sq1", "index": 0, "role": "SL", "assetId": "{84029128FA6F6BB9}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_GL.et", "position": {"x": 4839.2, "y": 6620.8, "z": 0, "rotation": 270},
           "loadout": {"version": 2,
             "wear": {"headCover": "res://helmet", "jacket": "res://bdu_blouse", "vest": "res://chest_rig", "armoredVest": "res://pasgt", "pants": "res://bdu_pants", "boots": null},
             "weapons": [{"slotIndex": 0, "slotType": "primary", "weapon": "res://m16", "optic": "res://acog", "magazine": "res://stanag", "attachments": []},
                         {"slotIndex": 1, "slotType": "primary", "weapon": "res://m72", "attachments": []},
                         {"slotIndex": 2, "slotType": "secondary", "weapon": "res://m9", "attachments": []},
                         {"slotIndex": 3, "slotType": "grenade", "weapon": "res://m67", "attachments": []}],
             "cargo": [{"container": "vest", "item": "res://stanag", "qty": 4},
                       {"container": "pants", "item": "res://bandage", "qty": 2},
                       {"container": "", "item": "res://dropped", "qty": 1}]}},
          {"id": "s2", "squadId": "sq1", "index": 1, "role": "TL", "position": {"x": 4836.9, "y": 6626.5, "z": 142.5, "rotation": 450}},
          {"id": "s3", "squadId": "sq1", "index": 2, "role": "TL", "position": {"x": 4831.2, "y": 6628.8, "z": 0, "rotation": 0},
           "loadout": {"version": 2, "wear": {"jacket": ""}, "weapons": [], "cargo": []}},
          {"id": "s4", "squadId": "sq2", "index": 0, "role": "RFL", "assetId": "{DCB41B3746FDD1BE}Prefabs/Characters/Factions/OPFOR/USSR_Army/Character_USSR_Rifleman.et", "position": {"x": 6010, "y": 7211.5, "z": 0, "rotation": 90},
           "loadout": {"version": 2, "wear": {}, "weapons": [],
             "cargo": [{"container": "backpack", "item": "res://ak_mag", "qty": 40}]}}
        ],
        "editorLayers": []
      }
    }"#;

    fn meta() -> MissionMeta {
        MissionMeta {
            id: "11112222333344445555666677778888".into(),
            title: "Compiled Fixture".into(),
            author: "maker".into(),
            terrain: "everon".into(),
            custom_terrain_name: String::new(),
            max_players: 64,
            time_of_day: "05:30".into(),
            weather_preset: "clear".into(),
        }
    }

    #[test]
    fn flatten_matches_locked_contract() {
        let doc = flatten_to_mod_document(&meta(), FIXTURE.as_bytes()).expect("compiles");
        // One slot carries y → schemaVersion bumps to 1.2.
        assert_eq!(doc.schema_version, "1.2");
        // Deterministic slot ids (faction:callsign:role:occurrence).
        let ids: Vec<&str> = doc.slots.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "blufor:Alpha:SL:0",
                "blufor:Alpha:TL:0",
                "blufor:Alpha:TL:1",
                "opfor:Grom:RFL:0"
            ]
        );
        // Locked mapping: x→x, y→z, z→y (optional), rotation→headingDeg (mod 360).
        let s0 = &doc.slots[0];
        assert!((s0.x - 4839.2).abs() < 1e-9 && (s0.z - 6620.8).abs() < 1e-9);
        assert!(s0.y.is_none() && (s0.heading_deg - 270.0).abs() < 1e-9);
        assert_eq!(doc.slots[1].y, Some(142.5));
        assert!((doc.slots[1].heading_deg - 90.0).abs() < 1e-9); // 450 % 360
        // Kit aliases: mapped assetId → kit; unmapped → faction default.
        assert_eq!(s0.kit, "kit:us_sl");
        assert_eq!(doc.slots[1].kit, "kit:us_rifleman");
        assert_eq!(doc.slots[3].kit, "kit:sov_rifleman");
        // Orbat instance count == slots length (loader parity gate).
        let orbat_count: i64 = doc
            .orbat
            .values()
            .flat_map(|f| &f.groups)
            .flat_map(|g| &g.roles)
            .map(|r| r.count)
            .sum();
        assert_eq!(orbat_count, doc.slots.len() as i64);
        assert_eq!(doc.meta.player_range, [1, 64]);

        // B1 — uid carries the editor slot id verbatim (identity thread).
        let uids: Vec<&str> = doc.slots.iter().map(|s| s.uid.as_str()).collect();
        assert_eq!(uids, ["s1", "s2", "s3", "s4"]);

        // T-068.11/A3 — s1: full gear + cargo. armoredVest wins over vest; jacket→uniform;
        // headCover→helmet; pants copied (A3), null boots omitted; weapons[0] triple;
        // malformed cargo row (empty container) drops.
        let lo = doc.slots[0].loadout.as_ref().expect("s1 loadout");
        let g = lo.gear.as_ref().expect("s1 gear");
        assert_eq!(
            (
                g.primary.as_deref(),
                g.optic.as_deref(),
                g.magazine.as_deref(),
                g.uniform.as_deref(),
                g.vest.as_deref(),
                g.helmet.as_deref(),
                g.pants.as_deref(),
                g.boots.as_deref()
            ),
            (
                Some("res://m16"),
                Some("res://acog"),
                Some("res://stanag"),
                Some("res://bdu_blouse"),
                Some("res://pasgt"),
                Some("res://helmet"),
                Some("res://bdu_pants"),
                None
            )
        );
        // T-182 — the other three authored weapon slots reach the wire under the editor's own
        // key names. Asserted on the SERIALIZED document, not just the struct, because the whole
        // point of the ticket is what the game server is handed.
        assert_eq!(
            (
                g.launcher.as_deref(),
                g.handgun.as_deref(),
                g.throwable.as_deref()
            ),
            (Some("res://m72"), Some("res://m9"), Some("res://m67"))
        );
        assert_eq!(lo.cargo.len(), 2);
        assert_eq!(
            (lo.cargo[0].container.as_str(), lo.cargo[0].qty),
            ("vest", 4)
        );
        // s2 (no loadout) + s3 (all-empty loadout) omit the key entirely on the wire.
        assert!(doc.slots[1].loadout.is_none() && doc.slots[2].loadout.is_none());
        let wire = serde_json::to_value(&doc).unwrap();
        assert!(wire["slots"][1].get("loadout").is_none());
        assert!(wire["slots"][2].get("loadout").is_none());
        // s4: cargo-only loadout → gear key omitted, cargo verbatim (qty 40 preserved).
        let lo4 = doc.slots[3].loadout.as_ref().expect("s4 loadout");
        assert!(lo4.gear.is_none());
        assert_eq!(
            (lo4.cargo[0].item.as_str(), lo4.cargo[0].qty),
            ("res://ak_mag", 40)
        );
        assert!(wire["slots"][3]["loadout"].get("gear").is_none());
        assert_eq!(wire["slots"][3]["loadout"]["cargo"][0]["qty"], 40);

        // T-182 — the three new keys on the actual wire, spelled exactly as the Arsenal UI and
        // mission.schema.json spell them. A rename here is a silent contract break: the mod reads
        // this block by field NAME via JsonLoadContext, which ignores keys it does not recognise.
        let s1_gear = &wire["slots"][0]["loadout"]["gear"];
        assert_eq!(s1_gear["launcher"], "res://m72");
        assert_eq!(s1_gear["handgun"], "res://m9");
        assert_eq!(s1_gear["throwable"], "res://m67");
    }

    #[test]
    fn slot_loadout_mapper_edge_cases() {
        // vest falls back when armoredVest is absent/empty.
        let lo = serde_json::json!({"wear": {"vest": "res://rig", "armoredVest": ""}});
        let m = mod_slot_loadout(&lo).expect("gear");
        assert_eq!(m.gear.unwrap().vest.as_deref(), Some("res://rig"));
        // T-182 — INVERTED. This assertion used to read `is_none()`, pinning the bug: an RPG
        // authored at slotIndex 1 produced no loadout at all, which is precisely how the silent
        // discard survived a green test suite. A launcher is authored content and now stands on
        // its own — the whole loadout survives on the strength of it, even though the jacket is
        // an empty string and the cargo row is dropped for qty<1.
        let lo = serde_json::json!({
            "wear": {"jacket": ""},
            "weapons": [{"slotIndex": 1, "slotType": "primary", "weapon": "res://rpg"}],
            "cargo": [{"container": "vest", "item": "res://mag", "qty": 0}]
        });
        let m = mod_slot_loadout(&lo).expect("launcher-only loadout must survive");
        let g = m.gear.expect("launcher-only gear");
        assert_eq!(g.launcher.as_deref(), Some("res://rpg"));
        // It must NOT be mistaken for the rifle — that would be the same loss wearing a new name.
        assert!(g.primary.is_none() && g.handgun.is_none() && g.throwable.is_none());
        assert!(m.cargo.is_empty());

        // All four slots at once, each landing in its own key and none stealing another's.
        let lo = serde_json::json!({
            "weapons": [
                {"slotIndex": 0, "slotType": "primary",   "weapon": "res://m4", "optic": "res://acog", "magazine": "res://stanag"},
                {"slotIndex": 1, "slotType": "primary",   "weapon": "res://rpg"},
                {"slotIndex": 2, "slotType": "secondary", "weapon": "res://m9"},
                {"slotIndex": 3, "slotType": "grenade",   "weapon": "res://m67"}
            ]
        });
        let g = mod_slot_loadout(&lo)
            .expect("four weapons")
            .gear
            .expect("gear");
        assert_eq!(
            (
                g.primary.as_deref(),
                g.launcher.as_deref(),
                g.handgun.as_deref(),
                g.throwable.as_deref(),
                g.optic.as_deref(),
                g.magazine.as_deref()
            ),
            (
                Some("res://m4"),
                Some("res://rpg"),
                Some("res://m9"),
                Some("res://m67"),
                Some("res://acog"),
                Some("res://stanag")
            )
        );

        // The PAIR is the selector, not the index: a row at the right index with the wrong
        // slotType is not silently promoted into the key it half-matches.
        let lo = serde_json::json!({
            "weapons": [{"slotIndex": 2, "slotType": "primary", "weapon": "res://bogus"}]
        });
        assert!(mod_slot_loadout(&lo).is_none());

        // A weapon row with an empty ResourceName drops rather than emitting an empty string
        // (the schema's minLength: 1 would reject it at /compiled).
        let lo = serde_json::json!({
            "weapons": [{"slotIndex": 3, "slotType": "grenade", "weapon": ""}]
        });
        assert!(mod_slot_loadout(&lo).is_none());
        // Cargo-only survives without gear.
        let lo =
            serde_json::json!({"cargo": [{"container": "pants", "item": "res://b", "qty": 1}]});
        let m = mod_slot_loadout(&lo).expect("cargo-only");
        assert!(m.gear.is_none());
        assert_eq!(m.cargo.len(), 1);
    }

    #[test]
    fn empty_editor_is_no_slots() {
        let payload = br#"{"editor":{"factions":[],"squads":[],"slots":[],"editorLayers":[]}}"#;
        assert!(matches!(
            flatten_to_mod_document(&meta(), payload),
            Err(CompileError::NoSlots)
        ));
    }

    // ── T-203 radioPlan ──────────────────────────────────────────────────────────────────

    /// Minimal editor payload: `(faction key, faction name, squad callsigns)`, one slot per
    /// squad so every squad reaches the ORBAT and therefore the radio plan.
    fn payload_with(factions: &[(&str, &str, &[&str])]) -> Vec<u8> {
        let (mut fs, mut squads, mut slots) = (Vec::new(), Vec::new(), Vec::new());
        for (fi, f) in factions.iter().enumerate() {
            let squad_ids: Vec<String> = (0..f.2.len()).map(|i| format!("f{fi}s{i}")).collect();
            for (i, callsign) in f.2.iter().enumerate() {
                let slot_id = format!("f{fi}s{i}p0");
                squads.push(serde_json::json!({
                    "id": format!("f{fi}s{i}"), "callsign": callsign, "slotIds": [slot_id]
                }));
                slots.push(serde_json::json!({
                    "id": slot_id, "index": 0, "role": "RFL",
                    "position": {"x": 1.0, "y": 2.0, "z": 0.0, "rotation": 0.0}
                }));
            }
            fs.push(serde_json::json!({"key": f.0, "name": f.1, "squadIds": squad_ids}));
        }
        serde_json::to_vec(
            &serde_json::json!({"editor": {"factions": fs, "squads": squads, "slots": slots}}),
        )
        .expect("fixture serializes")
    }

    /// The whole derivation on the locked fixture: what is emitted, from what, and in what
    /// order. Asserted on the SERIALIZED document as well as the struct, because the mod binds
    /// this block by field NAME through `JsonLoadContext` and silently ignores keys it does not
    /// recognise — a rename here is not a compile error anywhere, it is an empty radio plan.
    #[test]
    fn radio_plan_is_derived_from_the_orbat() {
        let doc = flatten_to_mod_document(&meta(), FIXTURE.as_bytes()).expect("compiles");
        let plan = doc.radio_plan.as_ref().expect("radioPlan emitted");

        // Command nets FIRST (both sides), then squad nets round-robin. Labels come from the
        // faction display name / the squad callsign the ORBAT already carries — `sq2` has no
        // callsign, so its `name` ("Grom") is what reached the ORBAT and it is what reaches here.
        let seen: Vec<String> = plan
            .nets
            .iter()
            .map(|n| {
                format!(
                    "{} | {} | {:.1} | {} | {}",
                    n.id,
                    n.label,
                    n.freq_mhz,
                    n.faction,
                    n.range.as_deref().unwrap_or("-")
                )
            })
            .collect();
        assert_eq!(
            seen,
            [
                "net:blufor_cmd | US Army Command | 30.0 | blufor | long",
                "net:opfor_cmd | Soviet VDV Command | 30.5 | opfor | long",
                "net:blufor_alpha | Alpha | 31.0 | blufor | -",
                "net:opfor_grom | Grom | 31.5 | opfor | -",
            ]
        );

        let wire = serde_json::to_value(&doc).unwrap();
        let nets = &wire["radioPlan"]["nets"];
        // `freqMHz`, NOT the `freqMhz` that `rename_all = "camelCase"` would have produced.
        // `TBD_RadioPlan.Fault` reads a missing frequency as 0, which is outside the schema
        // band, so every net would be rejected and the plan would arrive empty.
        assert_eq!(nets[0]["freqMHz"], 30.0);
        assert!(nets[0].get("freqMhz").is_none());
        // `range` is present ONLY where it does something. `TBD_RadioService.LongRangeFlag`
        // (TBD_RadioService.c:214-220) returns 1 for "long" and 0 for everything else, so
        // "short" and "any" are the same behaviour as absent — the squad nets do not claim a
        // distinction the mod does not make.
        assert_eq!(nets[0]["range"], "long");
        assert!(nets[2].get("range").is_none() && nets[3].get("range").is_none());

        // Deterministic: the game server re-fetches `/compiled` and the plan must not move
        // under it. Frequencies are allocated by position, so this also pins the allocation.
        let again = flatten_to_mod_document(&meta(), FIXTURE.as_bytes()).expect("compiles");
        assert_eq!(serde_json::to_value(&again).unwrap(), wire);
    }

    /// The cap the schema does NOT state. `TBD_RadioPlan.Parse` takes the first
    /// `MAX_NETS = 32` in document order across ALL factions and warns the rest away — so a
    /// faction-major emission with a big first side would leave the second side with no nets
    /// at all, command net included. This is the assertion that pins the ordering rules.
    #[test]
    fn radio_plan_never_silences_a_side_at_the_net_cap() {
        let big: Vec<String> = (0..40).map(|i| format!("Sq{i}")).collect();
        let refs: Vec<&str> = big.iter().map(String::as_str).collect();
        let payload = payload_with(&[("blufor", "US Army", &refs), ("opfor", "Soviet VDV", &refs)]);
        let doc = flatten_to_mod_document(&meta(), &payload).expect("compiles");
        let nets = &doc.radio_plan.as_ref().expect("radioPlan emitted").nets;

        // Cut here, so the mod never has to: 80 squads authored, 32 nets emitted.
        assert_eq!(nets.len(), MOD_MAX_NETS);
        // Both command nets survive, and they are the first two entries.
        assert_eq!(
            (nets[0].id.as_str(), nets[1].id.as_str()),
            ("net:blufor_cmd", "net:opfor_cmd")
        );
        // The remaining budget splits evenly rather than falling entirely on one side.
        let blufor = nets.iter().filter(|n| n.faction == "blufor").count();
        assert_eq!((blufor, nets.len() - blufor), (16, 16));
    }

    /// Ids and frequencies are the two values that must not repeat: the mod and the VOIP
    /// bridge key channels on `net.id`, and two sides sharing a frequency can hear each other.
    /// The callsigns here all reduce to the same slug through the netId alphabet, and one of
    /// them is spelled exactly like the disambiguator for another.
    #[test]
    fn radio_plan_ids_and_frequencies_are_unique() {
        let blufor: &[&str] = &["Alpha 1", "Alpha-1", "Alpha 1 2", "cmd"];
        let payload = payload_with(&[
            ("blufor", "US Army", blufor),
            ("opfor", "Soviet VDV", &["Alpha 1"]),
        ]);
        let doc = flatten_to_mod_document(&meta(), &payload).expect("compiles");
        let nets = &doc.radio_plan.as_ref().expect("radioPlan emitted").nets;

        let ids: HashSet<&str> = nets.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(ids.len(), nets.len(), "duplicate net id in {nets:?}");
        // The faction command net is emitted first, so it keeps the plain `net:<faction>_cmd`
        // and the squad literally called "cmd" is the one that gets suffixed.
        assert!(ids.contains("net:blufor_cmd") && ids.contains("net:blufor_cmd_2"));

        let freqs: HashSet<u64> = nets.iter().map(|n| (n.freq_mhz * 1000.0) as u64).collect();
        assert_eq!(freqs.len(), nets.len(), "duplicate frequency in {nets:?}");
        // Inside `mission.schema.json#/$defs/net/freqMHz` (30..=512) — a frequency outside it
        // is rejected net-by-net by `TBD_RadioPlan.Fault`.
        assert!(nets.iter().all(|n| (30.0..=512.0).contains(&n.freq_mhz)));
        // Every net names a faction the document declares; nothing is emitted unscoped.
        let declared: HashSet<&str> = doc.factions.iter().map(|f| f.key.as_str()).collect();
        assert!(nets.iter().all(|n| declared.contains(n.faction.as_str())));
    }

    /// The other limit the schema does not state. `TBD_RadioPlan.CapLabel` truncates past
    /// `MAX_LABEL_CHARS = 48` without telling anyone, so the compiler does it where the
    /// compiled document shows the string the player will actually see. An empty label is a
    /// hard rejection in `TBD_RadioPlan.Fault`, so the floor matters as much as the ceiling.
    #[test]
    fn radio_plan_label_is_capped_at_the_mod_limit() {
        let long_name = "N".repeat(200);
        let long_callsign = "C".repeat(200);
        let payload = payload_with(&[
            ("blufor", &long_name, &[long_callsign.as_str()]),
            ("opfor", "", &["Grom"]),
        ]);
        let doc = flatten_to_mod_document(&meta(), &payload).expect("compiles");
        let nets = &doc.radio_plan.as_ref().expect("radioPlan emitted").nets;

        assert!(
            nets.iter()
                .all(|n| (1..=MOD_MAX_LABEL_CHARS).contains(&n.label.chars().count()))
        );
        assert_eq!(nets[0].label, "N".repeat(MOD_MAX_LABEL_CHARS));
        // An unnamed faction falls back to its key, so the label is never a bare " Command".
        assert_eq!(nets[1].label, "opfor Command");
    }

    // ── T-200 kit substitutions ──────────────────────────────────────────────────────────
    //
    // The three ResourceNames below are REAL rows of
    // `packages/tbd-schema/registry/registry-items.workbench.json` — two of the 342 characters
    // the palette offers and `kit-aliases.json` has no row for, and one of the 12 it does. Real
    // ones on purpose: a made-up GUID would prove the code reports SOMETHING, not that it reports
    // the case an author actually hits. Placing a sniper and spawning a rifleman is the harm.

    const US_SNIPER: &str =
        "{0F6689B491641155}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Sniper.et";
    const USSR_MEDIC: &str =
        "{AB9726163EC1BD81}Prefabs/Characters/Factions/OPFOR/USSR_Army/Character_USSR_Medic.et";
    const US_RIFLEMAN_ALIASED: &str =
        "{26A9756790131354}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Rifleman.et";

    /// `(faction key, faction name, squad callsign, [assetId per slot])` — one squad per faction,
    /// one slot per assetId, all role `RFL`, so the derived slot id of the nth slot of faction i
    /// is `<key>:<callsign>:RFL:<n>`.
    fn payload_with_assets(factions: &[(&str, &str, &str, &[&str])]) -> Vec<u8> {
        let (mut fs, mut squads, mut slots) = (Vec::new(), Vec::new(), Vec::new());
        for (fi, (key, name, callsign, assets)) in factions.iter().enumerate() {
            let squad_id = format!("f{fi}sq");
            let slot_ids: Vec<String> = (0..assets.len()).map(|i| format!("f{fi}s{i}")).collect();
            for (i, asset) in assets.iter().enumerate() {
                slots.push(serde_json::json!({
                    "id": slot_ids[i], "index": i as i64, "role": "RFL", "assetId": asset,
                    "position": {"x": 1.0, "y": 2.0, "z": 0.0, "rotation": 0.0}
                }));
            }
            squads.push(
                serde_json::json!({"id": squad_id, "callsign": callsign, "slotIds": slot_ids}),
            );
            fs.push(serde_json::json!({"key": key, "name": name, "squadIds": [squad_id]}));
        }
        serde_json::to_vec(
            &serde_json::json!({"editor": {"factions": fs, "squads": squads, "slots": slots}}),
        )
        .expect("fixture serializes")
    }

    /// The whole point of the ticket: the substitution still happens (it must — see
    /// `KitSubstitutionReport`), but it is now on the record with enough detail to act on.
    #[test]
    fn unaliased_character_is_recorded_not_swallowed() {
        let payload = payload_with_assets(&[
            (
                "BLUFOR",
                "US Army",
                "Alpha",
                &[US_SNIPER, US_RIFLEMAN_ALIASED, US_SNIPER],
            ),
            ("OPFOR", "Soviet VDV", "Grom", &[USSR_MEDIC]),
        ]);
        let doc = flatten_to_mod_document(&meta(), &payload).expect("compiles");

        // BEHAVIOUR IS UNCHANGED — this is a report, not a repair. The sniper and the medic still
        // compile to their faction's generic rifleman, because that is the only value on hand that
        // satisfies `mission.schema.json`'s `^kit:[a-z0-9_]+$` and the alias table has no row.
        let kits: Vec<&str> = doc.slots.iter().map(|s| s.kit.as_str()).collect();
        assert_eq!(
            kits,
            [
                "kit:us_rifleman",
                "kit:us_rifleman",
                "kit:us_rifleman",
                "kit:sov_rifleman"
            ]
        );

        let rep = &doc.kit_substitutions;
        assert!(!rep.is_empty());
        // Three seats will spawn as somebody else; the aliased rifleman is not one of them.
        assert_eq!(rep.slots(), 3);
        assert_eq!(rep.rows().len(), 2, "{:?}", rep.rows());

        let sniper = &rep.rows()[0];
        assert_eq!(sniper.asset_id, US_SNIPER);
        assert_eq!(sniper.faction, "blufor");
        assert_eq!(sniper.kit, "kit:us_rifleman");
        // The FIRST slot that hit the pair, named the two ways the document names a slot: the
        // derived id a reader can grep the compiled JSON for, and the editor uid that survives a
        // role rename. The third slot carries the same asset and only bumps the count.
        assert_eq!(sniper.example_slot_id, "blufor:Alpha:RFL:0");
        assert_eq!(sniper.example_slot_uid, "f0s0");
        assert_eq!(sniper.occurrences, 2);

        let medic = &rep.rows()[1];
        assert_eq!(
            (
                medic.asset_id.as_str(),
                medic.faction.as_str(),
                medic.kit.as_str(),
                medic.occurrences
            ),
            (USSR_MEDIC, "opfor", "kit:sov_rifleman", 1)
        );

        // The rendered line has to carry all three answers, or it is a warning that tells an
        // operator something happened without telling them what.
        let lines = rep.details();
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert!(lines[0].starts_with("blufor:Alpha:RFL:0:"), "{lines:?}");
        assert!(lines[0].contains("Character_US_Sniper.et"), "{lines:?}");
        assert!(lines[0].contains("kit:us_rifleman"), "{lines:?}");
        assert!(lines[0].contains("and 1 more slot(s)"), "{lines:?}");
        // No tail line while every substituted slot is accounted for by a named row.
        assert!(
            !lines.iter().any(|l| l.starts_with('+')),
            "nothing was dropped: {lines:?}"
        );
    }

    /// A slot with no `assetId` expressed no preference, so the faction default is the answer and
    /// not a swap. This is the assertion that keeps the report readable: ORBAT templates and the
    /// `+` button both mint slots with no asset, and reporting those would bury the real findings
    /// under one line per templated seat.
    #[test]
    fn a_slot_that_named_no_character_is_not_a_substitution() {
        // The locked fixture: s1/s4 carry aliased assets, s2/s3 carry none at all. Nothing to say.
        let doc = flatten_to_mod_document(&meta(), FIXTURE.as_bytes()).expect("compiles");
        assert!(
            doc.kit_substitutions.is_empty(),
            "{:?}",
            doc.kit_substitutions.rows()
        );
        assert!(doc.kit_substitutions.details().is_empty());
        // …and those nameless slots did still take the faction default, which is the behaviour
        // that makes them uninteresting rather than unreported.
        assert_eq!(doc.slots[1].kit, "kit:us_rifleman");

        // An explicitly EMPTY assetId is the same case as an absent one.
        let payload = payload_with_assets(&[("BLUFOR", "US Army", "Alpha", &["", US_SNIPER])]);
        let doc = flatten_to_mod_document(&meta(), &payload).expect("compiles");
        assert_eq!(doc.kit_substitutions.slots(), 1, "only the sniper counts");
        assert_eq!(
            doc.kit_substitutions.rows()[0].example_slot_id,
            "blufor:Alpha:RFL:1"
        );
    }

    /// The `/compiled` contract. `mission.schema.json` sets top-level
    /// `additionalProperties: false` and `validated_compiled_body` answers **500** on any finding,
    /// so a report that reached the wire would not degrade the endpoint — it would break it
    /// outright, on every mission. `#[serde(skip)]` is what stops that, and a `#[serde(skip)]` is
    /// one keystroke from a `#[serde(rename)]`, so it is pinned here rather than trusted.
    #[test]
    fn substitutions_never_reach_the_compiled_wire() {
        let payload = payload_with_assets(&[
            ("BLUFOR", "US Army", "Alpha", &[US_SNIPER]),
            ("OPFOR", "Soviet VDV", "Grom", &[USSR_MEDIC]),
        ]);
        let doc = flatten_to_mod_document(&meta(), &payload).expect("compiles");
        assert!(!doc.kit_substitutions.is_empty(), "fixture must substitute");

        let wire = serde_json::to_value(&doc).unwrap();
        // `serde_json::Map` is ordered by key here (no `preserve_order` feature), so this is a SET
        // assertion — which is the right shape anyway: the mod binds this block by field NAME
        // through `JsonLoadContext` and emission order means nothing to it. What matters is that
        // the membership is exactly the schema's `properties`.
        let keys: Vec<&str> = wire
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            keys,
            [
                "environment",
                "factions",
                "flow",
                "meta",
                "orbat",
                "radioPlan",
                "schemaVersion",
                "slots",
                "winConditions",
                "zones"
            ],
            "top-level key set is the document contract — nothing new may appear here"
        );

        // Nothing anywhere in the body, at any depth, under either casing.
        let text = serde_json::to_string(&doc).unwrap();
        assert!(!text.contains("ubstitution"), "report leaked into the wire");
        // And the placed ResourceName is still absent from the compiled document — which is
        // exactly why this substitution was undetectable downstream: the document keeps no trace
        // of what the author asked for, only of what it decided.
        assert!(!text.contains("Character_US_Sniper"), "{text}");
    }

    /// Dedup is on the PAIR, not the asset. The same prefab dropped on two sides resolves to two
    /// different faction defaults, so collapsing them onto one row would report one side's
    /// substitution and hide the other's behind a kit that was never used for it.
    #[test]
    fn the_same_character_on_two_sides_is_two_substitutions() {
        let payload = payload_with_assets(&[
            ("BLUFOR", "US Army", "Alpha", &[USSR_MEDIC]),
            ("OPFOR", "Soviet VDV", "Grom", &[USSR_MEDIC]),
        ]);
        let doc = flatten_to_mod_document(&meta(), &payload).expect("compiles");
        let rows = doc.kit_substitutions.rows();
        assert_eq!(rows.len(), 2, "{rows:?}");
        assert_eq!(
            (rows[0].faction.as_str(), rows[0].kit.as_str()),
            ("blufor", "kit:us_rifleman")
        );
        assert_eq!(
            (rows[1].faction.as_str(), rows[1].kit.as_str()),
            ("opfor", "kit:sov_rifleman")
        );
        assert!(rows.iter().all(|r| r.asset_id == USSR_MEDIC));
    }

    /// The two shapes that would make this report useless: a bulk paste of one unaliased asset
    /// producing thousands of identical lines, and a mission wide enough to blow past the cap
    /// reporting a number that no longer adds up.
    #[test]
    fn repeats_collapse_and_the_cap_keeps_the_count_honest() {
        // 500 slots, one asset: exactly ONE row, with the count on it.
        let bulk: Vec<&str> = std::iter::repeat_n(US_SNIPER, 500).collect();
        let payload = payload_with_assets(&[("BLUFOR", "US Army", "Alpha", &bulk)]);
        let rep = flatten_to_mod_document(&meta(), &payload)
            .expect("compiles")
            .kit_substitutions;
        assert_eq!(rep.rows().len(), 1, "a bulk paste is one finding");
        assert_eq!((rep.rows()[0].occurrences, rep.slots()), (500, 500));
        assert_eq!(rep.details().len(), 1);

        // MAX + 5 DISTINCT assets, one of them placed twice. The list stops at the cap; the slot
        // count does not, and the tail line is the difference — so the total is still recoverable
        // from what is printed.
        let owned: Vec<String> = (0..MAX_REPORTED_SUBSTITUTIONS + 5)
            .map(|i| format!("{{DEADBEEF{i:08X}}}Prefabs/Characters/Made/Up_{i}.et"))
            .collect();
        let mut many: Vec<&str> = owned.iter().map(String::as_str).collect();
        many.push(owned[0].as_str());
        let payload = payload_with_assets(&[("BLUFOR", "US Army", "Alpha", &many)]);
        let rep = flatten_to_mod_document(&meta(), &payload)
            .expect("compiles")
            .kit_substitutions;
        assert_eq!(rep.rows().len(), MAX_REPORTED_SUBSTITUTIONS);
        assert_eq!(rep.slots(), MAX_REPORTED_SUBSTITUTIONS + 6);
        // The repeat landed on a NAMED row, so it is carried by that row's count and not by the
        // tail — the tail is exactly the 5 assets past the cap.
        let lines = rep.details();
        assert_eq!(lines.len(), MAX_REPORTED_SUBSTITUTIONS + 1);
        assert!(
            lines[MAX_REPORTED_SUBSTITUTIONS].starts_with("+ 5 further slot(s)"),
            "{:?}",
            lines[MAX_REPORTED_SUBSTITUTIONS]
        );
    }

    /// `assetId` is the one authored string the compile never copies into the document, so
    /// `wire_safety` does not scan it and a control character in an imported payload arrives here
    /// intact. It must not be able to garble the line it lands in — and it must not cost the
    /// reader the character name, which is why this is not `wire_safety::quote_value`.
    #[test]
    fn a_control_character_in_an_asset_id_is_escaped_but_nothing_is_truncated() {
        assert_eq!(
            escape_resource_name(US_SNIPER),
            US_SNIPER,
            "clean is verbatim"
        );
        // 84 chars — comfortably past quote_value's 60-char elision, and the name is at the end.
        assert!(US_SNIPER.chars().count() > 60);

        let dirty = format!("{US_SNIPER}\t");
        let out = escape_resource_name(&dirty);
        assert!(!out.contains('\t'), "{out}");
        assert!(out.contains("\\u{09}"), "{out}");
        assert!(
            out.contains("Character_US_Sniper.et"),
            "the name must survive: {out}"
        );
        assert!(!out.contains('…'), "nothing is elided: {out}");
    }

    // ── T-208 second pass — the compiler-shaped golden regenerates from HERE ─────────────
    //
    // `packages/tbd-schema/golden-missions/compiler-shaped-two-faction.json` is the only
    // committed golden whose entire claimed value is being BYTE-FAITHFUL to this emitter: the
    // other four are hand-authored documents that exercise the mod's parser, and they are
    // allowed to say things the compiler cannot. That claim was prose only. Nothing in any
    // `.rs`, `.sh`, `.yml` or `Makefile` regenerated the file and diffed it, so when T-203
    // taught `flatten_to_mod_document` to emit `radioPlan`, the golden simply stopped being
    // the compiler's output and no gate anywhere noticed:
    //
    //   * `xtask schema validate` cannot notice. `radioPlan` is deliberately NOT in
    //     `mission.schema.json`'s top-level `required` — the emitter legitimately omits it for
    //     a plan with no nets, and `TBD_RadioPlan.Parse` treats an absent plan as legal. A
    //     document missing the block is a VALID document; it is just not this compiler's.
    //   * `world-boot.sh` cannot notice. It boots what is committed and the stale file booted
    //     clean (`errors=0 warnings=0`) — being out of date is not a parse fault.
    //
    // So the only thing that can catch this drift is a re-run of the emitter against the
    // committed bytes, which is what lives below. The fixture is the editor graph the golden
    // was compiled from; it is inline rather than a second committed file so the payload and
    // the assertion can never drift apart on their own.
    //
    // WHEN THIS TEST FAILS after an intentional emitter change, the fix is to REGENERATE the
    // golden (`serde_json::to_string_pretty` + a trailing newline — the two lines the test
    // prints), never to hand-patch the delta into the JSON. A hand-patched file passes the
    // diff while ceasing to be an emitter output, which is exactly the property being pinned.

    /// The editor graph `compiler-shaped-two-faction.json` is the compiled output of.
    ///
    /// Deliberately holds no unaliased character: every `assetId` is either a `kit-aliases.json`
    /// row or absent. The AT seat carries no `assetId` at all — the "+ button" case, which
    /// `flatten_to_mod_document` resolves to the faction default WITHOUT recording a
    /// substitution. That keeps this fixture a pure emitter-shape fixture and leaves the
    /// substitution behaviour to T-200's own tests, which own it.
    const COMPILER_SHAPED_PAYLOAD: &str = r#"{
      "schemaVersion": 1,
      "map": {"terrain": "everon", "bounds": [0, 0, 12800, 12800]},
      "editor": {
        "factions": [
          {"id": "f_blu", "key": "BLUFOR", "name": "US Army", "squadIds": ["sq_ranger"]},
          {"id": "f_opf", "key": "OPFOR", "name": "Soviet VDV", "squadIds": ["sq_grom"]}
        ],
        "squads": [
          {"id": "sq_ranger", "factionId": "f_blu", "callsign": "Ranger", "name": "Ranger 1-1",
           "slotIds": ["n0", "n1", "n2", "n3", "n4"]},
          {"id": "sq_grom", "factionId": "f_opf", "callsign": "Grom", "name": "Grom 1",
           "slotIds": ["n5", "n6", "n7"]}
        ],
        "slots": [
          {"id": "n0", "squadId": "sq_ranger", "index": 0, "role": "SL",
           "assetId": "{84029128FA6F6BB9}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_GL.et",
           "position": {"x": 4837.6, "y": 7710.8, "z": 0, "rotation": 45},
           "loadout": {"version": 2,
             "wear": {
               "headCover": "{B74A4FF0DD8BB116}Prefabs/Characters/HeadGear/Helmet_PASGT_01/Helmet_PASGT_01.et",
               "jacket": "{293F577C298061E3}Prefabs/Characters/Uniforms/Jacket_US_BDU_02.et",
               "vest": "{477A190AF2A17B8A}Prefabs/Characters/Vests/Vest_ALICE/Variants/Vest_ALICE_MG.et",
               "pants": "{604BB72BE8E023C2}Prefabs/Characters/Uniforms/Pants_US_BDU.et",
               "boots": "{DAAFD15478BDE1C3}Prefabs/Characters/Footwear/CombatBoots_US_01.et",
               "handwear": "{8266820FFDE17477}Prefabs/Characters/Handwear/Gloves_Wool_01/Gloves_Wool_01.et",
               "backpack": "{06B68C58B72EAAC6}Prefabs/Items/Equipment/Backpacks/Backpack_ALICE_Medium.et"},
             "weapons": [
               {"slotIndex": 0, "slotType": "primary",
                "weapon": "{3E413771E1834D2F}Prefabs/Weapons/Rifles/M16/Rifle_M16A2.et",
                "optic": "{F358F46ADA42A197}Prefabs/Weapons/Attachments/Optics/Optic_4x20/Optic_4x20_base.et",
                "magazine": "{2EBF60EF24B108FC}Prefabs/Weapons/Magazines/Magazine_556x45_STANAG_30rnd_M855_Ball.et",
                "attachments": []},
               {"slotIndex": 2, "slotType": "secondary",
                "weapon": "{1353C6EAD1DCFE43}Prefabs/Weapons/Handguns/M9/Handgun_M9.et", "attachments": []},
               {"slotIndex": 3, "slotType": "grenade",
                "weapon": "{E8F00BF730225B00}Prefabs/Weapons/Grenades/Grenade_M67.et", "attachments": []}],
             "cargo": [
               {"container": "vest",
                "item": "{2EBF60EF24B108FC}Prefabs/Weapons/Magazines/Magazine_556x45_STANAG_30rnd_M855_Ball.et",
                "qty": 6},
               {"container": "jacket",
                "item": "{D70216B1B2889129}Prefabs/Items/Medicine/Tourniquet_01/Tourniquet_US_01.et", "qty": 1},
               {"container": "backpack",
                "item": "{13772C903CB5E4F7}Prefabs/Items/Equipment/Maps/Map_Paper_01/PaperMap_01_folded.et",
                "qty": 1}]}},
          {"id": "n1", "squadId": "sq_ranger", "index": 1, "role": "AR",
           "assetId": "{5B1996C05B1E51A4}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_AR.et",
           "position": {"x": 4844.9, "y": 7716.4, "z": 0, "rotation": 45}},
          {"id": "n2", "squadId": "sq_ranger", "index": 2, "role": "AT",
           "position": {"x": 4833.2, "y": 7721.5, "z": 0, "rotation": 40},
           "loadout": {"version": 2, "wear": {},
             "weapons": [
               {"slotIndex": 0, "slotType": "primary",
                "weapon": "{3E413771E1834D2F}Prefabs/Weapons/Rifles/M16/Rifle_M16A2.et",
                "magazine": "{2EBF60EF24B108FC}Prefabs/Weapons/Magazines/Magazine_556x45_STANAG_30rnd_M855_Ball.et",
                "attachments": []},
               {"slotIndex": 1, "slotType": "primary",
                "weapon": "{9C5C20FB0E01E64F}Prefabs/Weapons/Launchers/M72/Launcher_M72A3.et", "attachments": []}],
             "cargo": []}},
          {"id": "n3", "squadId": "sq_ranger", "index": 3, "role": "RFL",
           "assetId": "{26A9756790131354}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Rifleman.et",
           "position": {"x": 4850.1, "y": 7727.2, "z": 0, "rotation": 50}},
          {"id": "n4", "squadId": "sq_ranger", "index": 4, "role": "RFL",
           "assetId": "{26A9756790131354}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Rifleman.et",
           "position": {"x": 4844.2, "y": 7719.1, "z": 0, "rotation": 45},
           "loadout": {"version": 2, "wear": {}, "weapons": [],
             "cargo": [
               {"container": "vest",
                "item": "{2EBF60EF24B108FC}Prefabs/Weapons/Magazines/Magazine_556x45_STANAG_30rnd_M855_Ball.et",
                "qty": 4}]}},
          {"id": "n5", "squadId": "sq_grom", "index": 0, "role": "SL",
           "assetId": "{5436629450D8387A}Prefabs/Characters/Factions/OPFOR/USSR_Army/Character_USSR_SL.et",
           "position": {"x": 5182.7, "y": 7982.4, "z": 0, "rotation": 225},
           "loadout": {"version": 2,
             "wear": {
               "headCover": "{E49D9EE7E2B3016C}Prefabs/Characters/HeadGear/Helmet_ZSh5_01/Helmet_ZSh5_01.et",
               "jacket": "{9F546CCA2582D16F}Prefabs/Characters/Uniforms/Jacket_M88.et",
               "vest": "{ADE19B33DCBB9005}Prefabs/Characters/Vests/Vest_6B2/Vest_6B2.et",
               "pants": "{DCF980831E880F6A}Prefabs/Characters/Uniforms/Pants_M88.et",
               "boots": "{4C6029AB8BF5C044}Prefabs/Characters/Footwear/CombatBoots_Soviet_01_Dirty.et"},
             "weapons": [
               {"slotIndex": 0, "slotType": "primary",
                "weapon": "{43497A18DD888667}Prefabs/Weapons/Rifles/AK74/Rifle_AK74_base.et",
                "optic": "{ACDF49FACD0701A8}Prefabs/Weapons/Attachments/Optics/Optic_1P29/Optic_1P29.et",
                "magazine": "{63C1E699345B24F9}Prefabs/Weapons/Magazines/Magazine_545x39_AK_30rnd_Base.et",
                "attachments": []},
               {"slotIndex": 3, "slotType": "grenade",
                "weapon": "{645C73791ECA1698}Prefabs/Weapons/Grenades/Grenade_RGD5.et", "attachments": []}],
             "cargo": []}},
          {"id": "n6", "squadId": "sq_grom", "index": 1, "role": "AR",
           "assetId": "{23ADBBC31B6A3DC6}Prefabs/Characters/Factions/OPFOR/USSR_Army/Character_USSR_AR.et",
           "position": {"x": 5190.1, "y": 7988.6, "z": 0, "rotation": 230}},
          {"id": "n7", "squadId": "sq_grom", "index": 2, "role": "RFL",
           "assetId": "{DCB41B3746FDD1BE}Prefabs/Characters/Factions/OPFOR/USSR_Army/Character_USSR_Rifleman.et",
           "position": {"x": 5179.2, "y": 7993.0, "z": 0, "rotation": 220}}
        ],
        "editorLayers": []
      }
    }"#;

    /// The mission row `COMPILER_SHAPED_PAYLOAD` was compiled under.
    fn compiler_shaped_meta() -> MissionMeta {
        MissionMeta {
            id: "4c7e1b08-9a35-4d62-b1f7-e30d5a86c941".into(),
            title: "Grid Sweep at Montignac".into(),
            author: "184472930165846017".into(),
            terrain: "everon".into(),
            custom_terrain_name: String::new(),
            max_players: 12,
            time_of_day: "06:15".into(),
            weather_preset: "overcast".into(),
        }
    }

    /// The committed golden, pulled in by `include_str!` so the file is a COMPILE-TIME input:
    /// editing the JSON rebuilds this test. A `std::fs::read_to_string` would have made the
    /// golden a runtime path that a stale build could skip past.
    const COMPILER_SHAPED_GOLDEN: &str = include_str!(
        "../../../../packages/tbd-schema/golden-missions/compiler-shaped-two-faction.json"
    );

    /// Serialize a compiled document exactly the way the golden file is written: pretty,
    /// two-space, one trailing newline. The ONE definition — both the guard and the
    /// drift-detection proof below go through it, so they cannot disagree about what
    /// "the emitter's output" means.
    fn golden_text(doc: &ModMissionDocument) -> String {
        let mut s = serde_json::to_string_pretty(doc).expect("serialize compiled document");
        s.push('\n');
        s
    }

    /// First line where `expected` and `actual` differ, as `(1-based line, expected, actual)`.
    /// A missing or extra block shows up as the first line of the block rather than as a
    /// whole-file dump — the T-203 `radioPlan` drift is 4 nets deep inside a 280-line file.
    fn first_line_difference(expected: &str, actual: &str) -> Option<(usize, String, String)> {
        let (mut e, mut a) = (expected.lines(), actual.lines());
        let mut n = 0usize;
        loop {
            n += 1;
            match (e.next(), a.next()) {
                (None, None) => return None,
                (le, la) if le == la => continue,
                (le, la) => {
                    let show = |l: Option<&str>| l.unwrap_or("<end of file>").to_string();
                    return Some((n, show(le), show(la)));
                }
            }
        }
    }

    /// The guard. `compiler-shaped-two-faction.json` must be what this emitter produces TODAY,
    /// not what it produced the day the file was written.
    #[test]
    fn compiler_shaped_golden_is_a_fresh_emitter_output() {
        let doc =
            flatten_to_mod_document(&compiler_shaped_meta(), COMPILER_SHAPED_PAYLOAD.as_bytes())
                .expect("the compiler-shaped fixture compiles");

        // Nothing was substituted, so the golden's kits are the author's own choices and the
        // file is readable as written. Asserted, not assumed: a kit-aliases row deleted out from
        // under this fixture would otherwise quietly turn a named character into a default and
        // still round-trip, because the report is `#[serde(skip)]` and never reaches the bytes.
        assert!(
            doc.kit_substitutions.rows().is_empty(),
            "fixture must name only aliased characters: {:?}",
            doc.kit_substitutions.details()
        );

        let regenerated = golden_text(&doc);
        if let Some((line, expected, actual)) =
            first_line_difference(COMPILER_SHAPED_GOLDEN, &regenerated)
        {
            panic!(
                "packages/tbd-schema/golden-missions/compiler-shaped-two-faction.json is no \
                 longer this emitter's output.\n\
                 First difference at line {line}:\n  \
                 committed:   {expected}\n  \
                 regenerated: {actual}\n\n\
                 If the emitter change was intentional, REGENERATE the file — do not hand-patch \
                 the delta in, or it stops being a byte-faithful compiler output and this gate \
                 stops meaning anything:\n  \
                 let mut s = serde_json::to_string_pretty(&doc).unwrap();\n  \
                 s.push('\\n');\n\n\
                 Then re-run `scripts/mod/world-boot.sh \
                 --mission=compiler-shaped-two-faction`, because a regenerated document is a \
                 new document as far as the mod's parser is concerned."
            );
        }
    }

    /// Proof the guard above is not vacuous — it must FAIL on exactly the drift that got past
    /// everything: T-203 added `radioPlan` and the committed file kept the pre-T-203 shape.
    ///
    /// Replayed by deleting the block back out of the committed bytes rather than by asserting
    /// on a hand-written string, so this stays honest if the block's contents ever change.
    #[test]
    fn the_guard_catches_the_t203_radio_plan_drift() {
        let open = COMPILER_SHAPED_GOLDEN
            .find("  \"radioPlan\": {\n")
            .expect("the regenerated golden carries a radioPlan block");
        let close = COMPILER_SHAPED_GOLDEN[open..]
            .find("\n  },\n")
            .expect("radioPlan block is closed")
            + open
            + "\n  },\n".len();
        let pre_t203: String = format!(
            "{}{}",
            &COMPILER_SHAPED_GOLDEN[..open],
            &COMPILER_SHAPED_GOLDEN[close..]
        );

        // Sanity: the replay really is the old file — same document minus the one block.
        assert!(!pre_t203.contains("radioPlan"));
        assert!(pre_t203.contains("\"zones\"") && pre_t203.contains("\"slots\""));
        assert!(pre_t203.lines().count() < COMPILER_SHAPED_GOLDEN.lines().count());

        let (line, expected, actual) = first_line_difference(&pre_t203, COMPILER_SHAPED_GOLDEN)
            .expect(
                "dropping radioPlan MUST register as a difference — otherwise the guard is a \
                     no-op and the golden can rot again exactly the way it just did",
            );
        assert!(
            actual.contains("radioPlan"),
            "the diff must point AT the dropped block, not somewhere downstream of it \
             (line {line}: {expected:?} vs {actual:?})"
        );
    }

    // ---- T-202: the briefings block ----

    /// A hand-authored golden that carries a full two-sided `briefings` block. Pulled in at COMPILE
    /// time so editing the fixture rebuilds these tests, matching [`COMPILER_SHAPED_GOLDEN`].
    ///
    /// This file is the reason these tests can prove schema-validity without a `jsonschema`
    /// dev-dependency: `xtask`'s `make schema-validate` already validates every golden against
    /// `mission.schema.json`, and `scripts/mod/world-boot.sh` already boots this one through the real
    /// Enfusion parser. So reproducing its `briefings` block EXACTLY inherits both proofs.
    const BRIDGEHEAD_GOLDEN: &str =
        include_str!("../../../../packages/tbd-schema/golden-missions/bridgehead-at-levie.json");

    /// The two-faction editor graph with an authored `briefing` hung on each named faction ROW —
    /// T-214's shape (`editor.factions[].briefing`), which is what the emitter reads.
    ///
    /// Keys are matched through [`slug_key`], so `"blufor"` finds `FIXTURE`'s `"BLUFOR"` row; a key
    /// matching no row appends one, so a test can author for a side the graph does not have.
    fn payload_with_briefings(briefings: serde_json::Value) -> Vec<u8> {
        let mut p: serde_json::Value = serde_json::from_str(FIXTURE).expect("fixture parses");
        let factions = p["editor"]["factions"]
            .as_array_mut()
            .expect("fixture has faction rows");

        for (key, briefing) in briefings.as_object().expect("briefings is an object") {
            let slug = slug_key(key, "faction");
            match factions.iter_mut().find(|f| {
                f.get("key")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|k| slug_key(k, "faction") == slug)
            }) {
                Some(row) => {
                    row.as_object_mut()
                        .expect("faction row is an object")
                        .insert("briefing".to_string(), briefing.clone());
                }
                None => factions.push(serde_json::json!({
                    "id": format!("f_{slug}"), "key": key, "name": key,
                    "squadIds": [], "briefing": briefing,
                })),
            }
        }

        serde_json::to_vec(&p).expect("payload serialises")
    }

    /// Rewrite every number in a tree to its `f64` form, so a comparison is about VALUES and not
    /// about how a JSON integer happened to be spelled.
    ///
    /// Needed because the hand-authored goldens write a whole-number coordinate as `5402` while the
    /// compiler emits `5402.0` — its coordinate type is `f64` throughout (`slots[].x`, `zones[]`
    /// circle centres, and now `marker.x`/`z`), and `serde_json` renders an integral `f64` with the
    /// `.0`. Both are `{"type":"number"}` to the schema and both bind to `TBD_MissionMarkerStruct`'s
    /// `float x` / `float z`, so the difference is presentational only. Keeping `f64` is the right
    /// call rather than special-casing markers to integers: `marker.x`/`z` are `number`, not
    /// `integer`, and a marker at a fractional coordinate is legal and would be silently moved.
    fn numbers_as_f64(v: &serde_json::Value) -> serde_json::Value {
        match v {
            serde_json::Value::Number(n) => serde_json::json!(n.as_f64().unwrap_or(f64::NAN)),
            serde_json::Value::Array(a) => {
                serde_json::Value::Array(a.iter().map(numbers_as_f64).collect())
            }
            serde_json::Value::Object(o) => serde_json::Value::Object(
                o.iter()
                    .map(|(k, x)| (k.clone(), numbers_as_f64(x)))
                    .collect(),
            ),
            other => other.clone(),
        }
    }

    /// The compiled `briefings` block as a `serde_json::Value`, or `Null` when it was omitted.
    fn compiled_briefings(payload: &[u8]) -> serde_json::Value {
        let doc = flatten_to_mod_document(&meta(), payload).expect("compiles");
        serde_json::to_value(&doc)
            .expect("document serialises")
            .get("briefings")
            .cloned()
            .unwrap_or(serde_json::Value::Null)
    }

    /// The no-churn property, and the reason the T-208 drift guard above stays green without
    /// `compiler-shaped-two-faction.json` being regenerated for this ticket.
    ///
    /// Nothing authors a `briefings` block today, so the emitter must add NOTHING to the document.
    /// An empty-but-present `{}` would be schema-legal and behave identically in the mod, and would
    /// still be wrong here: it would change the compiled bytes of every mission on the platform to
    /// carry a block with no information in it.
    #[test]
    fn briefings_is_omitted_entirely_when_nothing_authors_one() {
        let doc = flatten_to_mod_document(&meta(), FIXTURE.as_bytes()).expect("compiles");
        assert!(doc.briefings.is_empty());

        let text = serde_json::to_string(&doc).expect("serialises");
        assert!(
            !text.contains("briefings"),
            "an unauthored briefings block must not reach the bytes at all: {text}"
        );

        // An explicit `"briefing": null` on the row is the same as no key at all — `Option` +
        // `#[serde(default)]`. This is the shape a UI that clears a briefing would most likely write.
        assert_eq!(
            compiled_briefings(&payload_with_briefings(
                serde_json::json!({"BLUFOR": serde_json::Value::Null})
            )),
            serde_json::Value::Null
        );

        // But a briefing that is PRESENT and empty is a different fact, and it is emitted: the author
        // opened this side's orders. `{}` is exactly `empty-warning-fields.json`'s `opfor` shape.
        let out = compiled_briefings(&payload_with_briefings(serde_json::json!({"BLUFOR": {}})));
        assert_eq!(out["blufor"], serde_json::json!({}));
    }

    /// The contract test. Feed the committed golden's OWN `briefings` block through the emitter and
    /// require the output to be that block again, unchanged.
    ///
    /// This is the strongest statement available about the shape: `bridgehead-at-levie.json` is
    /// schema-validated by `make schema-validate` and parsed by real Enfusion in
    /// `scripts/mod/world-boot.sh`, so an emitter that reproduces it byte-for-byte emits a document
    /// both gates already accept. It covers prose on both sides, per-side markers at the SAME
    /// coordinates (the side-discipline case), and lowercase keys that survive slugging untouched.
    #[test]
    fn authored_briefings_reproduce_the_committed_golden_block() {
        let golden: serde_json::Value =
            serde_json::from_str(BRIDGEHEAD_GOLDEN).expect("golden parses");
        let expected = golden
            .get("briefings")
            .expect("bridgehead-at-levie.json carries a briefings block")
            .clone();

        // Sanity: the fixture really is the rich two-sided case, not something that degenerated.
        assert!(expected.get("blufor").is_some() && expected.get("opfor").is_some());

        let actual = compiled_briefings(&payload_with_briefings(expected.clone()));
        assert_eq!(
            numbers_as_f64(&actual),
            numbers_as_f64(&expected),
            "the emitter must pass an authored briefings block through unchanged"
        );

        // The ONLY licensed difference is how a whole-number coordinate is spelled — the golden was
        // typed by hand as `5402`, the compiler emits its `f64` as `5402.0`. Pinned explicitly so
        // that `numbers_as_f64` above can never quietly paper over a real drift in some other field.
        assert_eq!(
            actual["blufor"]["markers"][0]["x"],
            serde_json::json!(5402.0)
        );
        assert_eq!(
            expected["blufor"]["markers"][0]["x"],
            serde_json::json!(5402)
        );
        assert_eq!(
            actual["blufor"]["markers"][0]["label"], expected["blufor"]["markers"][0]["label"],
            "every non-numeric field must be EXACTLY equal, no normalisation"
        );

        // Key ORDER too, not just key set: the schema declares situation/mission/execution/markers
        // and x/z/icon/label in that order, the goldens are written that way, and the compiled
        // document is read by humans.
        //
        // Asserted on the REAL emitted bytes, not on a `serde_json::Value` round-trip:
        // `Value::Object` is a `BTreeMap` without the `preserve_order` feature, so re-serialising a
        // `Value` sorts keys alphabetically and would make this check pass on any order at all. The
        // struct's own `Serialize` is what emits declaration order, so that is what gets tested.
        let doc =
            flatten_to_mod_document(&meta(), &payload_with_briefings(expected)).expect("compiles");
        let bytes = serde_json::to_string_pretty(&doc).expect("serialises");
        let block = &bytes[bytes.find("\"briefings\"").expect("briefings in the bytes")..];
        for keys in [
            ["situation", "mission", "execution", "markers"],
            ["x", "z", "icon", "label"],
        ] {
            let mut last = 0;
            for key in keys {
                let at = block
                    .find(&format!("\"{key}\""))
                    .unwrap_or_else(|| panic!("{key} present in {block}"));
                assert!(at > last, "{key} out of schema order in {block}");
                last = at;
            }
        }
    }

    /// The load-bearing key property: the mod resolves orders with
    /// `GetBriefingForFaction(slot.faction)`, so a briefings key that does not match the COMPILED
    /// faction key is a side that silently receives nothing.
    ///
    /// `FIXTURE` authors `editor.factions[].key` as `BLUFOR`/`OPFOR` and the compiler slugs those to
    /// `blufor`/`opfor`. An authored briefing keyed the way the author typed the faction must land on
    /// the slugged key, or the block is decoration.
    #[test]
    fn briefing_keys_are_slugged_onto_the_faction_keys_the_mod_looks_up() {
        let doc = flatten_to_mod_document(
            &meta(),
            &payload_with_briefings(serde_json::json!({
                "BLUFOR": {"situation": "west bank"},
                "OPFOR":  {"situation": "east bank"},
            })),
        )
        .expect("compiles");

        let keys: Vec<&str> = doc.briefings.keys().map(String::as_str).collect();
        assert_eq!(keys, ["blufor", "opfor"]);

        // The whole point: every briefings key is a key the mod will actually ask for.
        let faction_keys: Vec<&str> = doc.factions.iter().map(|f| f.key.as_str()).collect();
        for k in doc.briefings.keys() {
            assert!(
                faction_keys.contains(&k.as_str()),
                "briefings key {k:?} matches no compiled faction {faction_keys:?} — \
                 GetBriefingForFaction would miss and the side would get no orders"
            );
            assert!(
                doc.orbat.contains_key(k),
                "briefings key {k:?} is not an orbat key — the two must agree on faction identity"
            );
        }
    }

    /// Briefing prose is deliberately EXEMPT from the control-character ban
    /// (`mission.schema.json#/$defs/wireSafeString`, final paragraph): it does not ride a delimited
    /// wire — `TBD_BriefingService` ships it as parallel `array<string>` RPC parameters — and
    /// `TBD_BriefingData.AppendParagraphs` SPLITS on newlines to build display paragraphs.
    ///
    /// So a multi-paragraph situation report is the feature. Stripping newlines here would merge an
    /// author's paragraphs into one wall of text, and it would do it silently.
    #[test]
    fn prose_newlines_survive_because_briefing_prose_is_wire_exempt() {
        let prose = "Soviet airborne hold the crossing.\n\nSecond paragraph.\nThird line.";
        let out = compiled_briefings(&payload_with_briefings(serde_json::json!({
            "blufor": {"situation": prose, "mission": "Seize it.", "execution": "Advance."},
        })));

        assert_eq!(
            out["blufor"]["situation"].as_str().expect("string"),
            prose,
            "prose must reach the mod exactly as authored, newlines included"
        );
    }

    /// The question the ticket asks explicitly: does `marker.label` ride a delimited wire?
    ///
    /// **It does not.** `TBD_MarkerController.TBD_RpcDo_Markers` takes
    /// `(array<int> xs, array<int> zs, array<string> icons, array<string> labels, …)` — four PARALLEL
    /// arrays, chosen precisely so that an empty or delimiter-bearing label means exactly one thing
    /// (`TBD_MarkerData.c` header). Nothing in the five-file pipeline calls `Join`, `Split`,
    /// `Sanitise` or `FIELD_MARK`, and the schema types `marker.label` as a plain string rather than
    /// `$ref`-ing `wireSafeString`.
    ///
    /// So the T-181.42 roster treatment must NOT be applied: a TAB in a marker caption cannot shift a
    /// column because there are no columns. Rewriting it would corrupt an authored caption to defend
    /// against a hazard that does not exist on this lane.
    #[test]
    fn marker_labels_are_not_sanitised_because_markers_are_not_a_delimited_wire() {
        // The exact shape that broke the roster wire, on the lane that has no wire to break.
        let label = "AL\tPHA\nBRAVO";
        let out = compiled_briefings(&payload_with_briefings(serde_json::json!({
            "blufor": {"markers": [{"x": 1.0, "z": 2.0, "icon": "objective", "label": label}]},
        })));

        assert_eq!(
            out["blufor"]["markers"][0]["label"].as_str().expect("str"),
            label,
            "a marker label must pass through verbatim — see this test's doc comment for why"
        );
    }

    /// `#/$defs/marker` puts all four of `x`/`z`/`icon`/`label` in `required` AND sets
    /// `additionalProperties: false`. So the emitter may neither drop a key nor add one, and an empty
    /// string is CONTENT rather than absence — `golden-missions/empty-warning-fields.json` ships a
    /// committed marker with `icon` and `label` both `""`, and `TBD_MarkerIcons.ReportUnknown` treats
    /// that as information rather than as a mistake.
    ///
    /// An emitter that skipped `label` when it was empty would produce an invalid document, and the
    /// failure would land as a 500 on `/compiled` rather than as anything the author could read.
    #[test]
    fn every_marker_key_is_emitted_even_when_empty() {
        let out = compiled_briefings(&payload_with_briefings(serde_json::json!({
            "blufor": {"markers": [{"x": 7615.0, "z": 4350.0, "icon": "", "label": ""}]},
        })));

        let marker = out["blufor"]["markers"][0]
            .as_object()
            .expect("marker object");
        let mut keys: Vec<&str> = marker.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            ["icon", "label", "x", "z"],
            "exactly the four required keys — no omissions (required) and no extras \
             (additionalProperties: false)"
        );
        assert_eq!(marker["icon"], "");
        assert_eq!(marker["label"], "");

        // A briefing that carries only markers is legal: `briefing` declares no `required`, so the
        // three prose keys are simply absent rather than emitted blank.
        let entry = out["blufor"].as_object().expect("briefing object");
        assert_eq!(
            entry.keys().map(String::as_str).collect::<Vec<_>>(),
            ["markers"]
        );
    }

    /// The cap is applied in the compiler, not left to `TBD_MarkerService.CapLabel`, for the same
    /// reason `net.label` is: the mod truncates without telling anyone, so the compiled document a
    /// human reads should already show the caption the player will see.
    ///
    /// On a CHAR boundary — the mod's own cut is by bytes, so it may shorten a multi-byte caption
    /// slightly further, but a char-boundary cut here can never hand it invalid UTF-8.
    #[test]
    fn marker_labels_are_capped_at_the_mods_budget() {
        let long = "M".repeat(MOD_MAX_MARKER_LABEL_CHARS + 40);
        let out = compiled_briefings(&payload_with_briefings(serde_json::json!({
            "blufor": {"markers": [{"x": 1.0, "z": 2.0, "icon": "dot", "label": long}]},
        })));
        assert_eq!(
            out["blufor"]["markers"][0]
                .get("label")
                .and_then(serde_json::Value::as_str)
                .expect("str")
                .chars()
                .count(),
            MOD_MAX_MARKER_LABEL_CHARS
        );

        // Multi-byte: the cut must land on a char boundary, never mid-sequence.
        let wide = "Ω".repeat(MOD_MAX_MARKER_LABEL_CHARS + 5);
        let out = compiled_briefings(&payload_with_briefings(serde_json::json!({
            "blufor": {"markers": [{"x": 1.0, "z": 2.0, "icon": "dot", "label": wide}]},
        })));
        let got = out["blufor"]["markers"][0]["label"]
            .as_str()
            .expect("str")
            .to_string();
        assert_eq!(got.chars().count(), MOD_MAX_MARKER_LABEL_CHARS);
        assert!(got.chars().all(|c| c == 'Ω'), "cut mid-sequence: {got:?}");
    }

    /// Two authored keys can slug onto one faction (`BLUFOR` and `blufor`). They are the same side as
    /// far as the mod is concerned, so the emitter merges instead of letting document order decide
    /// which half of the author's orders survives — silently discarding authored prose is the failure
    /// mode `wire_safety`'s "no silent repair" rule exists to prevent.
    ///
    /// Prose joins on a BLANK LINE because that is what the mod reads as a paragraph break, so a
    /// merge renders as two paragraphs rather than two sentences run together.
    #[test]
    fn slug_colliding_briefing_keys_merge_rather_than_overwrite() {
        // Built by hand rather than through `payload_with_briefings`, which slug-matches and would
        // collapse these two onto one row before the emitter ever saw the collision.
        let mut p: serde_json::Value = serde_json::from_str(FIXTURE).expect("fixture parses");
        p["editor"]["factions"] = serde_json::json!([
            {"id": "f1", "key": "BLUFOR", "name": "US Army", "squadIds": ["sq1"],
             "briefing": {"situation": "first",
                          "markers": [{"x": 1.0, "z": 1.0, "icon": "dot", "label": "A"}]}},
            {"id": "f1b", "key": "blufor", "name": "US Army (dup)", "squadIds": [],
             "briefing": {"situation": "second",
                          "markers": [{"x": 2.0, "z": 2.0, "icon": "dot", "label": "B"}]}},
        ]);
        let out = compiled_briefings(&serde_json::to_vec(&p).expect("serialises"));

        let blufor = &out["blufor"];
        assert_eq!(blufor["situation"], "first\n\nsecond");
        assert_eq!(blufor["markers"].as_array().expect("array").len(), 2);
        assert_eq!(blufor["markers"][0]["label"], "A");
        assert_eq!(blufor["markers"][1]["label"], "B");

        // One merged side, not two colliding ones.
        assert_eq!(out.as_object().expect("object").len(), 1);
    }

    /// An authored empty string and an absent key are different authorial acts, and the emitted bytes
    /// must keep them apart. The mod collapses both to "render nothing" (`AppendParagraphs` does a
    /// CONTENT test), but `empty-warning-fields.json` ships both shapes side by side on purpose —
    /// `blufor` with three blank strings, `opfor` as `{}` — and a compiled document that turned one
    /// into the other would misreport what the author did.
    #[test]
    fn authored_blank_prose_is_distinguishable_from_an_absent_key() {
        let out = compiled_briefings(&payload_with_briefings(serde_json::json!({
            "blufor": {"situation": "", "mission": "", "execution": ""},
            "opfor": {},
        })));

        let blufor = out["blufor"].as_object().expect("object");
        assert_eq!(blufor["situation"], "");
        assert_eq!(blufor["mission"], "");
        assert_eq!(blufor["execution"], "");

        // `opfor` authored nothing at all → `{}`, exactly the golden's shape. Present as a KEY
        // (the author named the side) but empty as a VALUE.
        assert!(out.get("opfor").is_some());
        assert_eq!(out["opfor"].as_object().expect("object").len(), 0);
    }
}
