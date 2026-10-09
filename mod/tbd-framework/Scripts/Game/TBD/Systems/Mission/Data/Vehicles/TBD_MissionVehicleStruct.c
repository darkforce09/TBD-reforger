/**
 * @file TBD_MissionVehicleStruct.c
 * @brief One `vehicles[]` roster row: a mission-placed vehicle and its crew plan.
 *
 * Role: binds `$defs/vehicle` placement and `seats[]`.  Position: bound by `JsonLoadContext` onto
 * `TBD_MissionDocumentStruct.vehicles`; read by `TBD_MissionVehicleRoster` and
 * `TBD_MissionVehicleCrewSeating`.
 * State: none.  Invariants: field names equal the JSON keys; one authored vehicle also arrives as
 * an `entities[]` row carrying the same `uid`, alias and position, which the roster joins on
 * instead of spawning a second copy; `seats` is always allocated after a parse, so presence is
 * `Count()`.
 */

//! One `vehicles[]` roster row -- a mission-placed vehicle with its crew plan.
//! @contract mission.schema.json#/$defs/vehicle
class TBD_MissionVehicleStruct
{
	string alias;      //!< `veh:` registry alias. Schema-required; resolved through `TBD_Registry`.
	string uid;        //!< The editor's own vehicle id, carried verbatim. OPTIONAL -- empty is NOT identity. THE JOIN KEY to the `entities[]` twin.
	float x;           //!< World X metres. Schema-required.
	float z;           //!< World Z metres. Schema-required.
	float headingDeg;  //!< Yaw degrees. Always emitted by flatten, so the two projections of one vehicle cannot disagree about which way it faces.
	string faction;    //!< Faction key. OPTIONAL -- empty when absent.

	//! The crew plan. ALWAYS non-null after a parse (JsonLoadContext allocates it regardless of the
	//! key), so presence is `Count()`, never a null test. An uncrewed vehicle is a legal roster row.
	ref array<ref TBD_MissionVehicleSeatStruct> seats; //!< `seats[]`, always allocated

	//! How many crew stations this row authors. 0 for an uncrewed vehicle AND for an absent key --
	//! the two are indistinguishable on the wire by design (flatten omits an empty `seats`).
	int CrewCount()
	{
		if (!seats)
			return 0;
		return seats.Count();
	}

	//! The SECONDARY join key to the `entities[]` twin: alias plus exact position.
	//!
	//! Why it is sound. flatten derives both rows of one authored vehicle from the same source in the
	//! same pass -- same `alias` (both go through `KitAliases.vehicle_for_resource`), same `x`/`z`
	//! (both `pos.x` / `pos.y`) -- so for a document THIS platform compiled the two fingerprints are
	//! produced from byte-identical JSON numbers and therefore from bit-identical floats.
	//!
	//! Why it is needed at all. `uid` is optional on both rows, so a vehicle whose editor id is blank
	//! emits two rows with NO uid and nothing to join on -- flatten says so in its own comment and
	//! calls it the residual hole. Without this key that vehicle spawns twice; with it, it does not.
	//! The claim is one-shot (see `ClaimTwin`), so even two authored vehicles stacked on the exact
	//! same metre resolve one-to-one rather than both claiming the same world entity.
	string Fingerprint()
	{
		return string.Format("%1|%2|%3", alias, x, z);
	}

	//! The best identifying label for a log line: the authored uid when there is one, else the
	//! fingerprint, which always exists. Never a bare index into `vehicles[]` -- that shifts.
	string Label()
	{
		if (!uid.IsEmpty())
			return uid;
		return Fingerprint();
	}
}
