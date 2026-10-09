/**
 * @file TBD_MissionVehicleSeatStruct.c
 * @brief One crew station of a `vehicles[]` roster row: which slot rides which station.
 *
 * Role: binds one `$defs/vehicle.seats[]` entry.  Position: bound by `JsonLoadContext` inside
 * `TBD_MissionVehicleStruct.seats`; read by `TBD_MissionVehicleCrewSeating`.
 * State: none.  Invariants: field names equal the JSON keys; an absent `index` reads as
 * `INDEX_ABSENT`, never as 0.
 */

//! One `$defs/vehicle.seats[]` entry: which authored slot rides which crew station.
//! @contract mission.schema.json#/$defs/vehicle/properties/seats/items
class TBD_MissionVehicleSeatStruct
{
	//! A presence flag, not a magic default, on the same rule as `TBD_MissionSlotStruct.Y_ABSENT`:
	//! JsonLoadContext leaves a missing key at the field initializer, and no real compartment
	//! ordinal approaches -1e6.
	static const int INDEX_ABSENT = -1000000; //!< "`index` absent from JSON"

	//! References `slots[].uid` -- the DURABLE identity -- and NEVER the derived `slots[].id`,
	//! which shifts under role renames, reorders and deletes. Resolve it through
	//! `TBD_MissionLoader.GetSlotById`, which is uid-aware; do not string-compare it against `id`.
	//! Schema-required and always emitted: every seat came from a crew entry, so it always names
	//! an occupant.
	string slotId; //!< `slotId`: a `slots[].uid`

	//! One of the schema's closed enum of seven: driver, commander, gunner, cargo, pilot, copilot,
	//! turret. Enum-gated by the emitter, so an off-list token can only reach here in a hand-edited
	//! `$profile` document -- which this build parses and therefore does not trust.
	string role; //!< `role`: driver, commander, gunner, cargo, pilot, copilot or turret

	//! Disambiguates several stations of the same role (`cargo` 0, 1, 2 ...). OPTIONAL: absent for a
	//! station the author named without an ordinal, which is the common case for `driver`.
	//! INDEX_ABSENT when the key was not in the JSON -- test with `HasIndex()`, never against 0,
	//! because 0 is a REAL authored ordinal.
	int index = INDEX_ABSENT; //!< `index`: station ordinal within the role; default `INDEX_ABSENT`

	//! True when the mission JSON carried an explicit `index` for this seat.
	//!
	//! The distinction is load-bearing, not cosmetic: an AUTHORED ordinal is exact and a seat that
	//! cannot have it is skipped, whereas an ABSENT one means "a station of this role" and may fall
	//! forward to the next free one. See `TBD_MissionVehicleCrewSeating.ResolveCompartment`.
	bool HasIndex()
	{
		return index != INDEX_ABSENT;
	}
}
