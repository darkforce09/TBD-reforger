/**
 * @file TBD_MissionBriefingStruct.c
 * @brief One faction's written orders and the map markers authored on them.
 *
 * Role: the typed form of the `briefings` map values.  Position: filled by `TBD_MissionLoader`'s
 * parse; read through `TBD_MissionLoader.GetBriefingForFaction` by the briefing service.
 * State: none; plain data.  Invariants: every briefing field is optional, so an empty string means
 * "not authored" and renders nothing; a marker's four core keys are schema-required, so a marker
 * that exists is complete.
 */

//! One map marker authored on a faction's briefing. The styling keys are optional; -1 or an empty
//! string marks an absent key (the schema forbids -1 for `size` and allows 0 for `rotationDeg` and
//! `alpha`).
//! @contract mission.schema.json#/$defs/marker
class TBD_MissionMarkerStruct
{
	float x;              //!< JSON `x`: world X, metres.
	float z;              //!< JSON `z`: world Z, metres.
	string icon;          //!< JSON `icon`: icon key ("objective", "defend", "destroy").
	string label;         //!< JSON `label`: marker caption ("OBJ BRIDGE").
	float size = -1;      //!< JSON `size`: marker size; -1 = absent.
	float rotationDeg = -1; //!< JSON `rotationDeg`: rotation, degrees; -1 = absent.
	string shape;         //!< JSON `shape`: marker shape; empty = absent.
	string brush;         //!< JSON `brush`: fill brush; empty = absent.
	string color;         //!< JSON `color`: colour; empty = absent.
	float alpha = -1;     //!< JSON `alpha`: opacity, 0 is a legal invisible marker; -1 = absent.
}

//! One faction's written orders. Every field is optional; callers treat an empty string as not
//! authored.
//! @contract mission.schema.json#/$defs/briefing
class TBD_MissionBriefingStruct
{
	string situation;                               //!< JSON `situation`: what is happening.
	string mission;                                 //!< JSON `mission`: what this side must achieve.
	string execution;                               //!< JSON `execution`: how they are to do it.
	ref array<ref TBD_MissionMarkerStruct> markers; //!< JSON `markers`: optional map markers; null when absent.
}
