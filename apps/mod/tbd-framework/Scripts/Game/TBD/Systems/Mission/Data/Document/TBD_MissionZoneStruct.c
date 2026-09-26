/**
 * @file TBD_MissionZoneStruct.c
 * @brief One mission zone with its shape (circle or polygon) and its rules block.
 *
 * Role: the typed form of `zones[]` rows.  Position: filled by `TBD_MissionLoader`'s parse; read
 * by `TBD_ZoneRegistry`, the spawn-zone lookup, the objective registry and the validator.
 * State: none; plain data.  Invariants: `shape.circle` and `rules` are allocated even when their
 * keys are absent (a polygon-only zone reads `circle` as x=0 z=0 r=0), so a circle is present only
 * when `r > 0`, a polygon only when `Count() > 0`, and a rule only when it differs from its
 * `ABSENT` sentinel or empty string. Nothing here validates; `TBD_ZoneRegistry` reports a
 * malformed shape or rule by zone id.
 */

//! Circle shape in world metres: centre on X/Z and radius.
//! @contract mission.schema.json#/$defs/circle
class TBD_MissionCircleStruct
{
	float x; //!< JSON `x`: centre X, metres.
	float z; //!< JSON `z`: centre Z, metres.
	float r; //!< JSON `r`: radius, metres; 0 when the key is absent, which marks no circle.
}

//! Zone shape: the schema's `oneOf` circle or polygon, so a well-formed zone authors exactly one.
//! `ref array<ref array<float>>` is a legal field type and `JsonLoadContext` fills it.
//! @contract mission.schema.json#/$defs/shape
class TBD_MissionShapeStruct
{
	ref TBD_MissionCircleStruct circle;  //!< JSON `circle`: always allocated; present only when `r > 0`.
	ref array<ref array<float>> polygon; //!< JSON `polygon`: vertices as [x, z] pairs in world metres; null when a circle is authored.
}

//! The zone `rules` object, as far as a typed parser can see it. The schema closes the vocabulary;
//! a key without a field here is invisible at runtime. Objective keys (`captureSeconds`,
//! `holdSeconds`, ...) are read by the second pass in `TBD_ObjectiveRulesReader`. `TBD_ZoneRegistry`
//! reports a rules object with no legible key as a WARNING and an out-of-range value or unknown
//! `penalty` as an ERROR with the default applied.
//! @contract mission.schema.json#/$defs/zoneRules
class TBD_MissionZoneRulesStruct
{
	static const float ABSENT = -1000000; //!< Float sentinel for an absent key; no authored value approaches it and JSON carries no NaN.
	static const int ABSENT_INT = -1;     //!< Integer sentinel for an absent count.

	float graceSeconds = ABSENT;     //!< JSON `graceSeconds`: seconds a player may stay in violation before the penalty; >= 0.
	float warnEverySeconds = ABSENT; //!< JSON `warnEverySeconds`: seconds between warnings while in violation; > 0.
	string penalty;                  //!< JSON `penalty`: "none", "warn" or "kill"; empty = absent (see `TBD_EZonePenalty`).
	int attackerCount = ABSENT_INT;  //!< JSON `attackerCount`: attackers needed in the volume; `ABSENT_INT` when absent.
	int defenderCount = ABSENT_INT;  //!< JSON `defenderCount`: defenders needed in the volume; `ABSENT_INT` when absent.
	float advantagePercent = ABSENT; //!< JSON `advantagePercent`: numeric advantage to flip the zone, percent.
	float minHeight = ABSENT;        //!< JSON `minHeight`: lower bound of the volume, metres; may be negative.
	float maxHeight = ABSENT;        //!< JSON `maxHeight`: upper bound of the volume, metres.
	string startingOwner;            //!< JSON `startingOwner`: faction key owning the zone at start; empty = absent.
	ref array<string> vehicleClasses; //!< JSON `vehicleClasses`: play-area penalty limited to these classes; `Count() == 0` applies to all (`TBD_PlayAreaVehicleAxis`).
}

//! One entry of the mission `zones[]` array (spawn, objective, boundary, ...).
//! @contract mission.schema.json#/$defs/zone
class TBD_MissionZoneStruct
{
	string id;                         //!< JSON `id`: the zone id findings and objectives name.
	string type;                       //!< JSON `type`: zone type ("spawn", "objective_capture", ...).
	string label;                      //!< JSON `label`: optional editor name; callers fall back to `type` and `id` when empty.
	string faction;                    //!< JSON `faction`: owning faction key; empty when absent.
	string variantId;                  //!< JSON `variantId`: variant gate; empty = unconditional row.
	ref TBD_MissionShapeStruct shape;  //!< JSON `shape`: circle or polygon.
	ref TBD_MissionZoneRulesStruct rules; //!< JSON `rules`: optional; every field at its sentinel means nothing legible was authored.
}
