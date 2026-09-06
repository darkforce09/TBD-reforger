//! T-675.2 -- the compiled `vehicles[]` ROSTER: the mission-placed vehicles and the crew plan
//! T-076 authors, read in the mod and turned into seated crew.
//!
//! == THE DOUBLE-SPAWN TRAP, WHICH IS THE WHOLE DIFFICULTY OF THIS FILE ==
//! ONE authored vehicle emits TWO rows on the wire, deliberately (`flatten.rs` ModVehicle):
//!
//!   * an `entities[]` alias row -- `{alias, uid?, x, z, headingDeg?, faction?, inventory}` -- which
//!     `TBD_MissionLoader.SpawnMissionEntities()` ALREADY places in the world, and has since T-254;
//!   * a `vehicles[]` roster row -- the same placement PLUS `seats[]`, because `$defs/entity` is
//!     `additionalProperties: false` and has nowhere to put a crew plan.
//!
//! So a roster reader that simply spawns every row it reads spawns every crewed vehicle TWICE. The
//! guard is a JOIN, not a heuristic: `uid` is carried on BOTH rows for exactly this purpose (T-946.18),
//! so a roster row first tries to CLAIM the world entity its `entities[]` twin already produced and
//! only spawns when there is nothing to claim. `TBD_MissionVehicleRoster` owns that index, the claim,
//! and the census that proves the result -- see `SeatAuthoredCrews`.
//!
//! == PRESENCE ==
//! `JsonLoadContext.ReadValue` ALLOCATES a nested `ref` field even when the JSON key is ABSENT
//! (measured; TBD_MissionLoader.c:31-42). So `if (v.seats)` is ALWAYS TRUE and is NOT a presence
//! test. Presence here is `seats.Count()` for the container and an empty string / the `INDEX_ABSENT`
//! scalar sentinel for the scalars, exactly as `TBD_MissionSlotStruct.Y_ABSENT` does it.
//!
//! Field names must equal the JSON keys -- `JsonLoadContext` binds by CLASS MEMBER NAME.
//! @contract mission.schema.json#/$defs/vehicle

//! One `$defs/vehicle.seats[]` entry: which authored slot rides which crew station.
//! @contract mission.schema.json#/$defs/vehicle (seats[])
class TBD_MissionVehicleSeatStruct
{
	//! "index absent from JSON". A presence flag, not a magic default, on the same rule as
	//! `TBD_MissionSlotStruct.Y_ABSENT`: JsonLoadContext leaves a missing key at the field
	//! initializer, and no real compartment ordinal approaches -1e6.
	static const int INDEX_ABSENT = -1000000;

	//! References `slots[].uid` -- the DURABLE identity -- and NEVER the derived `slots[].id`,
	//! which shifts under role renames, reorders and deletes. Resolve it through
	//! `TBD_MissionLoader.GetSlotById`, which is uid-aware; do not string-compare it against `id`.
	//! Schema-required and always emitted: every seat came from a crew entry, so it always names
	//! an occupant.
	string slotId;

	//! One of the schema's closed enum of seven: driver, commander, gunner, cargo, pilot, copilot,
	//! turret. Enum-gated by the emitter, so an off-list token can only reach here in a hand-edited
	//! `$profile` document -- which this build parses and therefore does not trust.
	string role;

	//! Disambiguates several stations of the same role (`cargo` 0, 1, 2 ...). OPTIONAL: absent for a
	//! station the author named without an ordinal, which is the common case for `driver`.
	//! INDEX_ABSENT when the key was not in the JSON -- test with `HasIndex()`, never against 0,
	//! because 0 is a REAL authored ordinal.
	int index = INDEX_ABSENT;

	//------------------------------------------------------------------------------------------------
	//! True when the mission JSON carried an explicit `index` for this seat.
	//!
	//! The distinction is load-bearing, not cosmetic: an AUTHORED ordinal is exact and a seat that
	//! cannot have it is skipped, whereas an ABSENT one means "a station of this role" and may fall
	//! forward to the next free one. See `TBD_MissionVehicleRoster.ResolveCompartment`.
	bool HasIndex()
	{
		return index != INDEX_ABSENT;
	}
}

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
	ref array<ref TBD_MissionVehicleSeatStruct> seats;

	//------------------------------------------------------------------------------------------------
	//! How many crew stations this row authors. 0 for an uncrewed vehicle AND for an absent key --
	//! the two are indistinguishable on the wire by design (flatten omits an empty `seats`).
	int CrewCount()
	{
		if (!seats)
			return 0;
		return seats.Count();
	}

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	//! The best identifying label for a log line: the authored uid when there is one, else the
	//! fingerprint, which always exists. Never a bare index into `vehicles[]` -- that shifts.
	string Label()
	{
		if (!uid.IsEmpty())
			return uid;
		return Fingerprint();
	}
}

//! One `entities[]` row that ACTUALLY reached the world, kept so a `vehicles[]` roster row can claim
//! it instead of spawning a second copy. `body` is a plain (non-owning) handle, the same way
//! `TBD_SpawnManager.m_mSlotBodies` holds spawned bodies.
class TBD_MissionVehicleTwin
{
	string uid;         //!< `$defs/entity.uid`, or empty when the row carried none.
	string fingerprint; //!< `alias|x|z`, always present.
	IEntity body;       //!< The spawned world entity.
	bool claimed;       //!< True once a roster row has taken it. One-shot, so two rows cannot share one entity.
}

//! One seat whose `GetInVehicle` was accepted, held until the deferred pass can ask the engine
//! whether the body actually ended up inside. See `TBD_MissionVehicleRoster.VerifySeatedCrews`.
class TBD_MissionVehiclePendingSeat
{
	IEntity body;      //!< The slot body that was told to get in.
	IEntity vehicle;   //!< The vehicle it was told to get into.
	string slotKey;    //!< The slot's durable key, for the log line.
	string vehicleLabel; //!< The roster row's label, for the log line.
	string role;       //!< The authored seat role, for the log line.
}

//! T-675.2 -- the roster reader: the entities[] index, the anti-double-spawn join, crew seating, and
//! the census that proves one authored vehicle produced exactly one world vehicle.
class TBD_MissionVehicleRoster
{
	//! Half-extent of the census box around a roster position, metres. Small enough that two
	//! DIFFERENT authored vehicles are not counted together (a vehicle is metres wide and the editor
	//! cannot place two on the same metre by accident), large enough to cover a spawn that settled
	//! onto the surface a little off the authored point.
	protected static const float CENSUS_XZ_M = 3.0;
	//! Vertical half-extent of the census box. Generous: the census asks "how many of this prefab
	//! stand HERE in XZ", and terrain height is not the question.
	protected static const float CENSUS_Y_M = 300.0;

	//! How long to wait before asking the engine whether the accepted seats actually took.
	//!
	//! MEASURED, not guessed: `GetInVehicle` returns true in the requesting frame but
	//! `GetVehicleIn` still answers null there, so a same-frame check reports every seat as
	//! deferred (four of four on the roster fixture). One second is ~4x `TBD_SpawnManager`'s own
	//! 250 ms settle tick and well inside the headless boot's settle window, so the verdict lands in
	//! the same boot log as the request.
	protected static const int SEAT_VERIFY_DELAY_MS = 1000;

	//! Every `entities[]` row that spawned, in wire order. Rebuilt on each `SpawnMissionEntities`.
	protected static ref array<ref TBD_MissionVehicleTwin> s_aTwins;

	//! Seats accepted by this pass, awaiting the deferred verification. Rebuilt on each pass.
	protected static ref array<ref TBD_MissionVehiclePendingSeat> s_aPendingSeats;

	// ---- census scratch. Static because `QueryEntitiesByAABB` takes a plain function pointer, the
	// same reason `TBD_ObjectiveRegistry.s_QueryResource` is static. Never read outside a query. ----
	protected static ResourceName s_CensusPrefab;
	protected static int s_iCensusHits;

	//------------------------------------------------------------------------------------------------
	//! Drop the entities[] -> world index.
	//!
	//! Called at the TOP of `SpawnMissionEntities`, before its own early return, so a reload whose
	//! new mission authors no `entities[]` cannot inherit the previous mission's rows and let a
	//! roster row "join" a vehicle that no longer exists.
	static void ResetIndex()
	{
		s_aTwins = new array<ref TBD_MissionVehicleTwin>();
	}

	//------------------------------------------------------------------------------------------------
	//! Record one `entities[]` row that reached the world. Skipped rows are deliberately NOT
	//! recorded: a roster row must then spawn its own vehicle, because nothing exists to claim.
	static void RecordEntitySpawn(string uid, string alias, float x, float z, IEntity body)
	{
		if (!s_aTwins)
			ResetIndex();
		if (!body)
			return;

		TBD_MissionVehicleTwin twin = new TBD_MissionVehicleTwin();
		twin.uid = uid;
		twin.fingerprint = string.Format("%1|%2|%3", alias, x, z);
		twin.body = body;
		twin.claimed = false;
		s_aTwins.Insert(twin);
	}

	//------------------------------------------------------------------------------------------------
	//! THE ANTI-DOUBLE-SPAWN GUARD. Claim the world entity this roster row's `entities[]` twin
	//! already produced, or null when there is none to claim.
	//!
	//! Two keys, tried in order, and both claim ONCE:
	//!
	//!  1. `uid` -- the durable join key, carried on both rows since T-946.18. Exact match only.
	//!  2. the `alias|x|z` fingerprint -- for the documented residual hole where the authored id is
	//!     blank and NEITHER row carries a uid. A twin that carries a DIFFERENT non-empty uid is
	//!     never matched positionally: two distinct authored vehicles stacked on one metre must not
	//!     be cross-joined just because they happen to share a position.
	//!
	//! Returning null is a real answer, not a failure: a hand-edited `$profile` mission may carry a
	//! `vehicles[]` row with no `entities[]` twin at all, and that vehicle genuinely has to be
	//! spawned here. See `ResolveVehicleBody`.
	protected static IEntity ClaimTwin(TBD_MissionVehicleStruct veh)
	{
		if (!s_aTwins)
			return null;

		if (!veh.uid.IsEmpty())
		{
			foreach (TBD_MissionVehicleTwin byUid : s_aTwins)
			{
				if (!byUid || byUid.claimed || byUid.uid != veh.uid)
					continue;

				byUid.claimed = true;
				return byUid.body;
			}
		}

		string fingerprint = veh.Fingerprint();
		foreach (TBD_MissionVehicleTwin byPos : s_aTwins)
		{
			if (!byPos || byPos.claimed || byPos.fingerprint != fingerprint)
				continue;
			// A twin with its own, different identity is not this row's twin.
			if (!byPos.uid.IsEmpty() && byPos.uid != veh.uid)
				continue;

			byPos.claimed = true;
			return byPos.body;
		}

		return null;
	}

	//------------------------------------------------------------------------------------------------
	//! T-675.2 -- read the compiled `vehicles[]` roster, put exactly one vehicle in the world per
	//! authored row, and seat the slots each row names.
	//!
	//! Called once from `TBD_SpawnManager.MaterializeSlotBodies` after every slot body exists and
	//! before the loadout settle is armed. Slot bodies must already stand in the world, because a
	//! seat is a body being moved INTO a vehicle -- there is nothing to move before then.
	//!
	//! A rosterless mission returns on the first line and behaves exactly as it did before this
	//! slice: no index walk, no query, not one line of log.
	static void SeatAuthoredCrews(TBD_SpawnManager spawner)
	{
		array<ref TBD_MissionVehicleStruct> roster = TBD_MissionLoader.GetVehicles();
		if (!roster || roster.Count() == 0)
			return;

		s_aPendingSeats = new array<ref TBD_MissionVehiclePendingSeat>();

		int joined = 0;
		int spawned = 0;
		int skipped = 0;
		int seatsAccepted = 0;
		int seatSkips = 0;
		int doubles = 0;

		foreach (TBD_MissionVehicleStruct veh : roster)
		{
			if (!veh || veh.alias.IsEmpty())
			{
				// Schema-required, so this is a hand-edited document. Skipping the row is the only
				// honest move: an aliasless vehicle cannot be resolved and guessing one is the T-200
				// silent substitution with ten tonnes in place of a rifleman.
				Print("[TBD][Vehicles] skip roster row with no alias -- cannot resolve a prefab for it", LogLevel.WARNING);
				skipped++;
				continue;
			}

			bool ok;
			ResourceName prefab = TBD_Registry.Resolve(veh.alias, ok);
			if (!ok || prefab.IsEmpty())
			{
				Print(string.Format("[TBD][Vehicles] skip %1 alias='%2' -- not in registry", veh.Label(), veh.alias), LogLevel.WARNING);
				skipped++;
				continue;
			}

			bool spawnedHere;
			IEntity body = ResolveVehicleBody(veh, prefab, spawnedHere);
			if (!body)
			{
				skipped++;
				continue;
			}

			if (spawnedHere)
				spawned++;
			else
				joined++;

			// THE PROOF, and it is deliberately not our own bookkeeping: ask the WORLD how many
			// entities of this prefab stand at this roster position. A double spawn puts two of them
			// on the same metre, so anything but 1 is reported -- 2 is the trap having fired.
			int census = CountWorldVehiclesAt(prefab, veh.x, veh.z);
			if (census != 1)
			{
				doubles++;
				Print(string.Format("[TBD][Vehicles] CENSUS %1 alias='%2' at %3,%4 -- world holds %5 entity(ies) of this prefab, expected exactly 1",
					veh.Label(), veh.alias, veh.x, veh.z, census), LogLevel.ERROR);
			}

			int rowSkips;
			seatsAccepted += SeatCrew(spawner, veh, body, rowSkips);
			seatSkips += rowSkips;
		}

		Print(string.Format("[TBD][Vehicles] roster done rows=%1 joined=%2 spawned=%3 skipped=%4 seatsAccepted=%5 seatSkips=%6 censusFailures=%7",
			roster.Count(), joined, spawned, skipped, seatsAccepted, seatSkips, doubles));

		if (doubles > 0)
			Print(string.Format("[TBD][Vehicles] %1 roster row(s) FAILED the world census -- an authored vehicle reached the world more than once",
				doubles), LogLevel.ERROR);

		// The line above reports what was REQUESTED. The verdict on what actually happened comes
		// from the engine a second later; arm it only when there is something to check.
		if (s_aPendingSeats.Count() > 0)
			GetGame().GetCallqueue().CallLater(VerifySeatedCrews, SEAT_VERIFY_DELAY_MS, false);
	}

	//------------------------------------------------------------------------------------------------
	//! The world entity for one roster row: the twin it claims, else a fresh spawn.
	//! `spawnedHere` says which, so the caller's census can separate the two.
	protected static IEntity ResolveVehicleBody(TBD_MissionVehicleStruct veh, ResourceName prefab, out bool spawnedHere)
	{
		spawnedHere = false;

		IEntity claimed = ClaimTwin(veh);
		if (claimed)
		{
			Print(string.Format("[TBD][Vehicles] %1 alias='%2' joined the entities[] row already in the world -- NOT spawned again",
				veh.Label(), veh.alias));
			return claimed;
		}

		IEntity body = SpawnRosterVehicle(veh, prefab);
		if (!body)
			return null;

		spawnedHere = true;
		// Record it as an already-claimed twin so a second roster row naming the same vehicle joins
		// this one instead of spawning a third copy.
		RecordEntitySpawn(veh.uid, veh.alias, veh.x, veh.z, body);
		if (s_aTwins && s_aTwins.Count() > 0)
			s_aTwins[s_aTwins.Count() - 1].claimed = true;

		return body;
	}

	//------------------------------------------------------------------------------------------------
	//! Spawn one roster vehicle at its authored transform.
	//!
	//! This CAN fire and is not a dead path: `TBD_MissionLoader.LoadFromProfileFile` parses a
	//! hand-editable `$profile` mission, which may carry a `vehicles[]` row with no `entities[]`
	//! twin, and `SpawnMissionEntities` skips rows whose prefab fails to load -- in both cases
	//! nothing exists to claim and the roster row is the only thing that can put the vehicle there.
	//! Proven at runtime: see the slice report's world-boot census.
	protected static IEntity SpawnRosterVehicle(TBD_MissionVehicleStruct veh, ResourceName prefab)
	{
		Resource resource = Resource.Load(prefab);
		if (!resource || !resource.IsValid())
		{
			Print(string.Format("[TBD][Vehicles] Resource.Load failed for %1 alias='%2' prefab=%3", veh.Label(), veh.alias, prefab), LogLevel.ERROR);
			return null;
		}

		float surfaceY = GetGame().GetWorld().GetSurfaceY(veh.x, veh.z);
		vector pos = Vector(veh.x, surfaceY, veh.z);

		EntitySpawnParams params = new EntitySpawnParams();
		params.TransformMode = ETransformMode.WORLD;
		Math3D.MatrixIdentity4(params.Transform);
		params.Transform[3] = pos;

		float yawRad = veh.headingDeg * Math.DEG2RAD;
		params.Transform[0] = Vector(Math.Cos(yawRad), 0, Math.Sin(yawRad));
		params.Transform[2] = Vector(-Math.Sin(yawRad), 0, Math.Cos(yawRad));

		IEntity body = GetGame().SpawnEntityPrefab(resource, GetGame().GetWorld(), params);
		if (!body)
		{
			Print(string.Format("[TBD][Vehicles] SpawnEntityPrefab failed for %1 alias='%2'", veh.Label(), veh.alias), LogLevel.ERROR);
			return null;
		}

		Print(string.Format("[TBD][Vehicles] %1 alias='%2' spawned at %3 heading=%4 (no entities[] twin to claim)",
			veh.Label(), veh.alias, pos.ToString(), veh.headingDeg));
		return body;
	}

	//------------------------------------------------------------------------------------------------
	//! Seat every slot this roster row names.
	//!
	//! Returns how many seats the engine ACCEPTED; `skipped` counts the ones that could not be
	//! requested at all, each of which has logged its own reason. Accepted is deliberately not
	//! called seated: only `VerifySeatedCrews`, a second later, can say a body is in the vehicle,
	//! and conflating the two is the signature defect this program was opened over.
	protected static int SeatCrew(TBD_SpawnManager spawner, TBD_MissionVehicleStruct veh, IEntity vehicle, out int skipped)
	{
		skipped = 0;
		int accepted = 0;
		if (veh.CrewCount() == 0)
			return 0;

		foreach (TBD_MissionVehicleSeatStruct seat : veh.seats)
		{
			if (!seat)
			{
				skipped++;
				continue;
			}

			if (SeatOne(spawner, veh, vehicle, seat))
				accepted++;
			else
				skipped++;
		}

		return accepted;
	}

	//------------------------------------------------------------------------------------------------
	//! Move one authored slot's body into one authored crew station.
	//!
	//! Every failure is REPORTED and skips only that seat: a seat that cannot be filled must never
	//! cost the rest of the crew their places, and a silently dropped crew plan is the T-216 failure
	//! this program exists to close.
	//!
	//! Returns whether the engine ACCEPTED the seat -- never whether the body ended up in the
	//! vehicle. Only the deferred `VerifySeatedCrews` pass can answer that.
	protected static bool SeatOne(TBD_SpawnManager spawner, TBD_MissionVehicleStruct veh, IEntity vehicle, TBD_MissionVehicleSeatStruct seat)
	{
		if (seat.slotId.IsEmpty())
		{
			Print(string.Format("[TBD][Vehicles] %1 seat role='%2' names no slot -- skipped", veh.Label(), seat.role), LogLevel.WARNING);
			return false;
		}

		// `seats[].slotId` carries a slot UID; GetSlotById is uid-aware, so this is the one correct
		// resolver. flatten refuses to emit a seat naming a slot it did not emit, but this build
		// parses documents it did not compile, so a dangling reference is reported rather than
		// resolved to whichever seat happens to answer.
		TBD_MissionSlotStruct slot = TBD_MissionLoader.GetSlotById(seat.slotId);
		if (!slot)
		{
			Print(string.Format("[TBD][Vehicles] %1 seat role='%2' slotId='%3' resolves to no slot -- skipped",
				veh.Label(), seat.role, seat.slotId), LogLevel.WARNING);
			return false;
		}

		IEntity body = spawner.GetSlotBody(slot.Key());
		if (!body)
		{
			Print(string.Format("[TBD][Vehicles] %1 seat role='%2' slot=%3 has no materialized body -- skipped",
				veh.Label(), seat.role, slot.Key()), LogLevel.WARNING);
			return false;
		}

		bool knownRole;
		ECompartmentType type = CompartmentTypeFor(seat.role, knownRole);
		if (!knownRole)
		{
			Print(string.Format("[TBD][Vehicles] %1 seat slot=%2 role='%3' is not one of the seven authored roles -- skipped",
				veh.Label(), slot.Key(), seat.role), LogLevel.WARNING);
			return false;
		}

		BaseCompartmentSlot compartment = ResolveCompartment(veh, vehicle, seat, type);
		if (!compartment)
			return false;

		ChimeraCharacter character = ChimeraCharacter.Cast(body);
		if (!character)
		{
			Print(string.Format("[TBD][Vehicles] %1 seat slot=%2 body is not a character -- skipped", veh.Label(), slot.Key()), LogLevel.WARNING);
			return false;
		}

		SCR_CompartmentAccessComponent access = SCR_CompartmentAccessComponent.Cast(character.GetCompartmentAccessComponent());
		if (!access)
		{
			Print(string.Format("[TBD][Vehicles] %1 seat slot=%2 body has no SCR_CompartmentAccessComponent -- NOT seated",
				veh.Label(), slot.Key()), LogLevel.WARNING);
			return false;
		}

		// FORCE TELEPORT, AND THIS WAS MEASURED, NOT ASSUMED.
		//
		// `MoveInVehicle` was the obvious call and it is the WRONG one here: it returns true for a
		// request it has merely accepted, and the character then has to WALK to the door and play the
		// get-in animation. Slot bodies have their AI disabled at spawn (`DisableBodyAI`), so that
		// walk never happens and the crew would stand beside the vehicle forever while the log said
		// "seated". Measured on a headless boot of the roster fixture: four seats accepted, zero of
		// them inside the vehicle -- see the slice report.
		//
		// `GetInVehicle(..., forceTeleport = true, ...)` is the immediate form and is what an
		// AUTHORED crew plan means: the crew START in the vehicle, they do not run to it at mission
		// load. `doorInfoIndex` is ignored under force teleport (engine doc), so -1 is passed to say
		// "no door", and the door is left as authored rather than animated shut behind a teleport.
		// The resolved compartment is passed explicitly, so the authored station is the one taken.
		if (!access.GetInVehicle(vehicle, compartment, true, -1, ECloseDoorAfterActions.LEAVE_OPEN, false))
		{
			Print(string.Format("[TBD][Vehicles] %1 seat slot=%2 role='%3' -- GetInVehicle refused the compartment",
				veh.Label(), slot.Key(), seat.role), LogLevel.WARNING);
			return false;
		}

		// A true from the call above says the REQUEST was ACCEPTED. It does not say the body is in the
		// vehicle, and reporting the first as the second is exactly how a dead mechanism reads as a
		// working one. Measured on the headless roster fixture: `GetVehicleIn` answers null in the
		// requesting frame for every one of four accepted seats, force teleport included -- the
		// engine attaches the occupant later. So the seat is queued for the deferred pass, which asks
		// the engine itself, and nothing calls it seated until the engine says so.
		QueuePendingSeat(body, vehicle, slot.Key(), veh.Label(), seat.role);

		Print(string.Format("[TBD][Vehicles] %1 seat accepted slot=%2 role='%3' index=%4 compartmentType=%5 -- awaiting engine confirmation",
			veh.Label(), slot.Key(), seat.role, seat.index, type));
		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! Hold one accepted seat for the deferred verification pass.
	protected static void QueuePendingSeat(IEntity body, IEntity vehicle, string slotKey, string vehicleLabel, string role)
	{
		if (!s_aPendingSeats)
			s_aPendingSeats = new array<ref TBD_MissionVehiclePendingSeat>();

		TBD_MissionVehiclePendingSeat pending = new TBD_MissionVehiclePendingSeat();
		pending.body = body;
		pending.vehicle = vehicle;
		pending.slotKey = slotKey;
		pending.vehicleLabel = vehicleLabel;
		pending.role = role;
		s_aPendingSeats.Insert(pending);
	}

	//------------------------------------------------------------------------------------------------
	//! THE SEATING PROOF. Ask the engine, one second after the requests, which bodies are actually in
	//! the vehicle they were told to get into.
	//!
	//! `GetVehicleIn` is the engine's own answer and it is compared against the SPECIFIC vehicle the
	//! seat named, not merely "some vehicle" -- a crewman in the wrong vehicle is the failure this is
	//! for, and "is he in a vehicle at all" could not see it. A seat that never confirms is reported
	//! at WARNING with its slot and role, because an authored crew plan that silently did nothing is
	//! the T-216 failure this program exists to close.
	static void VerifySeatedCrews()
	{
		if (!s_aPendingSeats || s_aPendingSeats.Count() == 0)
			return;

		int confirmed = 0;
		int unconfirmed = 0;
		foreach (TBD_MissionVehiclePendingSeat pending : s_aPendingSeats)
		{
			if (!pending || !pending.body || !pending.vehicle)
			{
				unconfirmed++;
				continue;
			}

			if (SCR_CompartmentAccessComponent.GetVehicleIn(pending.body) == pending.vehicle)
			{
				confirmed++;
				continue;
			}

			unconfirmed++;
			Print(string.Format("[TBD][Vehicles] %1 slot=%2 role='%3' -- accepted but the engine STILL does not report the body in this vehicle after %4 ms; the authored crew plan did not take effect for this seat",
				pending.vehicleLabel, pending.slotKey, pending.role, SEAT_VERIFY_DELAY_MS), LogLevel.WARNING);
		}

		LogLevel level = LogLevel.NORMAL;
		if (unconfirmed > 0)
			level = LogLevel.WARNING;

		Print(string.Format("[TBD][Vehicles] seating verified inVehicle=%1 of %2 accepted (%3 unconfirmed after %4 ms)",
			confirmed, s_aPendingSeats.Count(), unconfirmed, SEAT_VERIFY_DELAY_MS), level);

		s_aPendingSeats = null;
	}

	//------------------------------------------------------------------------------------------------
	//! Which compartment of the resolved type this seat takes.
	//!
	//! An AUTHORED `index` is EXACT: if that ordinal does not exist or is already taken, the seat is
	//! reported and skipped rather than moved somewhere the author did not choose. An ABSENT index
	//! means "a station of this role", so it starts at the role's natural ordinal and may fall
	//! forward to the next free station of the same type -- which is what lets a commander and a
	//! gunner, both TURRET stations, resolve to different turrets. Every fall-forward is logged;
	//! none of it is silent.
	protected static BaseCompartmentSlot ResolveCompartment(TBD_MissionVehicleStruct veh, IEntity vehicle, TBD_MissionVehicleSeatStruct seat, ECompartmentType type)
	{
		SCR_BaseCompartmentManagerComponent manager = SCR_BaseCompartmentManagerComponent.Cast(vehicle.FindComponent(SCR_BaseCompartmentManagerComponent));
		if (!manager)
		{
			Print(string.Format("[TBD][Vehicles] %1 alias='%2' has no compartment manager -- role='%3' NOT seated",
				veh.Label(), veh.alias, seat.role), LogLevel.WARNING);
			return null;
		}

		array<BaseCompartmentSlot> compartments = {};
		manager.GetCompartmentsOfType(compartments, type);
		if (compartments.IsEmpty())
		{
			Print(string.Format("[TBD][Vehicles] %1 alias='%2' has no compartment of the type role='%3' needs -- NOT seated",
				veh.Label(), veh.alias, seat.role), LogLevel.WARNING);
			return null;
		}

		int ordinal = DefaultOrdinalFor(seat.role);
		if (seat.HasIndex())
			ordinal = seat.index;

		if (ordinal >= 0 && ordinal < compartments.Count())
		{
			BaseCompartmentSlot exact = compartments[ordinal];
			if (exact && !exact.IsOccupied())
				return exact;
		}

		if (seat.HasIndex())
		{
			Print(string.Format("[TBD][Vehicles] %1 role='%2' authored index=%3 -- that station is missing or already occupied (%4 of this type) -- NOT seated",
				veh.Label(), seat.role, seat.index, compartments.Count()), LogLevel.WARNING);
			return null;
		}

		foreach (BaseCompartmentSlot free : compartments)
		{
			if (!free || free.IsOccupied())
				continue;

			Print(string.Format("[TBD][Vehicles] %1 role='%2' authored no index -- station %3 was taken, using the next free one of the same type",
				veh.Label(), seat.role, ordinal));
			return free;
		}

		Print(string.Format("[TBD][Vehicles] %1 role='%2' -- every station of this type (%3) is occupied -- NOT seated",
			veh.Label(), seat.role, compartments.Count()), LogLevel.WARNING);
		return null;
	}

	//------------------------------------------------------------------------------------------------
	//! `$defs/vehicle.seats[].role` to the engine's compartment type.
	//!
	//! The engine models crew stations as three types only (PILOT / TURRET / CARGO), so this is the
	//! whole of what a prefab-independent mapping can say. `commander`, `gunner` and `turret` all map
	//! to TURRET because Reforger models every crewed station that is not the driver as a turret
	//! compartment; a prefab that has none reports and skips the seat rather than dropping the crew
	//! member into cargo, which would look seated and be wrong.
	//!
	//! `known` is an out flag rather than a sentinel because ECompartmentType has no "unset" member
	//! to borrow -- all three of its values are real answers.
	protected static ECompartmentType CompartmentTypeFor(string role, out bool known)
	{
		known = true;

		if (role == "driver")
			return ECompartmentType.PILOT;
		if (role == "pilot")
			return ECompartmentType.PILOT;
		if (role == "copilot")
			return ECompartmentType.PILOT;
		if (role == "commander")
			return ECompartmentType.TURRET;
		if (role == "gunner")
			return ECompartmentType.TURRET;
		if (role == "turret")
			return ECompartmentType.TURRET;
		if (role == "cargo")
			return ECompartmentType.CARGO;

		known = false;
		return ECompartmentType.CARGO;
	}

	//------------------------------------------------------------------------------------------------
	//! The station ordinal a role takes when the author gave no `index`.
	//!
	//! Only `copilot` is not 0, and that is what the word means: the second PILOT station. A vehicle
	//! with one pilot station therefore reports and skips an authored copilot instead of putting them
	//! in the pilot's seat.
	protected static int DefaultOrdinalFor(string role)
	{
		if (role == "copilot")
			return 1;
		return 0;
	}

	//------------------------------------------------------------------------------------------------
	//! How many entities of `prefab` the WORLD holds at this roster position.
	//!
	//! This is the census that proves the double-spawn guard, and it deliberately does not consult
	//! our own bookkeeping: `QueryEntitiesByAABB` asks the world itself. One authored vehicle must
	//! answer 1. A guard that failed would answer 2, because the roster row and its `entities[]` twin
	//! carry the same position by construction.
	protected static int CountWorldVehiclesAt(ResourceName prefab, float x, float z)
	{
		BaseWorld world = GetGame().GetWorld();
		if (!world)
			return 0;

		s_CensusPrefab = prefab;
		s_iCensusHits = 0;

		vector mins = Vector(x - CENSUS_XZ_M, -CENSUS_Y_M, z - CENSUS_XZ_M);
		vector maxs = Vector(x + CENSUS_XZ_M, CENSUS_Y_M, z + CENSUS_XZ_M);
		world.QueryEntitiesByAABB(mins, maxs, OnCensusEntity);

		s_CensusPrefab = ResourceName.Empty;
		return s_iCensusHits;
	}

	//------------------------------------------------------------------------------------------------
	//! Census callback. Static because the query API takes a plain function; the scratch it writes
	//! into is documented on `s_CensusPrefab`.
	protected static bool OnCensusEntity(IEntity entity)
	{
		if (!entity)
			return true;

		EntityPrefabData prefabData = entity.GetPrefabData();
		if (!prefabData)
			return true;

		if (prefabData.GetPrefabName() != s_CensusPrefab)
			return true;

		s_iCensusHits++;
		return true;
	}
}
