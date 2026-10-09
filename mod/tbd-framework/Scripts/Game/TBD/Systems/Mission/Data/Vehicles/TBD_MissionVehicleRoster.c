/**
 * @file TBD_MissionVehicleRoster.c
 * @brief Puts exactly one world vehicle behind each `vehicles[]` roster row.
 *
 * Role: the `entities[]` placement index, the claim that joins a roster row to its placed twin,
 * the fallback spawn, and the world census that proves one authored vehicle is one entity.
 * Position: fed by `TBD_MissionWorldApplier` (`ResetIndex`, `RecordEntitySpawn`); driven by
 * `TBD_SlotBodyMaterializer` through `SeatAuthoredCrews`; hands crew seating to
 * `TBD_MissionVehicleCrewSeating`.
 * State: the static twin index, rebuilt on every mission load, server only.  Invariants: each
 * twin is claimed at most once; a census other than 1 is logged at ERROR, never corrected.
 */

//! One `entities[]` row that ACTUALLY reached the world, kept so a `vehicles[]` roster row can claim
//! it instead of spawning a second copy. `body` is a plain (non-owning) handle, the same way
//! `TBD_SlotBodyMaterializer.m_mSlotBodies` holds spawned bodies.
class TBD_MissionVehicleTwin
{
	string uid;         //!< `$defs/entity.uid`, or empty when the row carried none.
	string fingerprint; //!< `alias|x|z`, always present.
	IEntity body;       //!< The spawned world entity.
	bool claimed;       //!< True once a roster row has taken it. One-shot, so two rows cannot share one entity.
}

//! The roster reader: the `entities[]` index, the anti-double-spawn join, and the census that
//! proves one authored vehicle produced exactly one world vehicle. Crew seating is
//! `TBD_MissionVehicleCrewSeating`.
class TBD_MissionVehicleRoster
{
	//! Half-extent of the census box around a roster position, metres. Small enough that two
	//! DIFFERENT authored vehicles are not counted together (a vehicle is metres wide and the editor
	//! cannot place two on the same metre by accident), large enough to cover a spawn that settled
	//! onto the surface a little off the authored point.
	protected static const float CENSUS_XZ_M = 3.0; //!< metres, census box half-extent in X and Z
	//! Vertical half-extent of the census box. Generous: the census asks "how many of this prefab
	//! stand HERE in XZ", and terrain height is not the question.
	protected static const float CENSUS_Y_M = 300.0; //!< metres, census box half-extent in Y

	protected static ref array<ref TBD_MissionVehicleTwin> s_aTwins; //!< every placed `entities[]` row in wire order; rebuilt on each mission load

	//! Drop the entities[] -> world index.
	//!
	//! Called at the top of the `entities[]` placement pass, before its own early return, so a
	//! reload whose new mission authors no `entities[]` cannot inherit the previous mission's rows
	//! and let a roster row "join" a vehicle that is not in the world.
	static void ResetIndex()
	{
		s_aTwins = new array<ref TBD_MissionVehicleTwin>();
	}

	//! Record one `entities[]` row that reached the world. Skipped rows are deliberately NOT
	//! recorded: a roster row must then spawn its own vehicle, because nothing exists to claim.
	//! @param uid the row's `uid`, or empty
	//! @param alias the row's registry alias
	//! @param x world X, metres
	//! @param z world Z, metres
	//! @param body the placed entity; null records nothing
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

	//! THE ANTI-DOUBLE-SPAWN GUARD. Claim the world entity this roster row's `entities[]` twin
	//! already produced, or null when there is none to claim.
	//!
	//! Two keys, tried in order, and both claim ONCE:
	//!
	//!  1. `uid` -- the durable join key, carried on both rows. Exact match only.
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

	//! Read the compiled `vehicles[]` roster, put exactly one vehicle in the world per authored row,
	//! and seat the slots each row names.
	//!
	//! Called once from `TBD_SlotBodyMaterializer` after every slot body exists and
	//! before the loadout settle is armed. Slot bodies must already stand in the world, because a
	//! seat is a body being moved INTO a vehicle -- there is nothing to move before then.
	//!
	//! A rosterless mission returns on the first line: no index walk, no query, not one line of log.
	//! @param spawner the spawn manager that owns the slot bodies
	//! @authority server
	static void SeatAuthoredCrews(TBD_SpawnManager spawner)
	{
		array<ref TBD_MissionVehicleStruct> roster = TBD_MissionLoader.GetVehicles();
		if (!roster || roster.Count() == 0)
			return;

		TBD_MissionVehicleCrewSeating.BeginPass();

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
				// honest move: an aliasless vehicle cannot be resolved, and guessing one would be a
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
			seatsAccepted += TBD_MissionVehicleCrewSeating.SeatCrew(spawner, veh, body, rowSkips);
			seatSkips += rowSkips;
		}

		Print(string.Format("[TBD][Vehicles] roster done rows=%1 joined=%2 spawned=%3 skipped=%4 seatsAccepted=%5 seatSkips=%6 censusFailures=%7",
			roster.Count(), joined, spawned, skipped, seatsAccepted, seatSkips, doubles));

		if (doubles > 0)
			Print(string.Format("[TBD][Vehicles] %1 roster row(s) FAILED the world census -- an authored vehicle reached the world more than once",
				doubles), LogLevel.ERROR);

		// The line above reports what was REQUESTED. The verdict on what actually happened comes
		// from the engine a second later.
		TBD_MissionVehicleCrewSeating.ArmVerification();
	}

	//! The world entity for one roster row: the twin it claims, else a fresh spawn.
	//! @param spawnedHere set true when this call spawned the vehicle, false when it claimed a twin
	//! @return the vehicle entity, or null when the spawn failed (already logged)
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

	//! Spawn one roster vehicle at its authored transform.
	//!
	//! This CAN fire and is not a dead path: the profile artifact cache (TBD_MissionArtifactCache)
	//! can hold a hand-staged mission, which may carry a `vehicles[]` row with no `entities[]`
	//! twin, and `TBD_MissionWorldApplier.SpawnMissionEntities` skips rows whose prefab fails to load -- in both cases
	//! nothing exists to claim and the roster row is the only thing that can put the vehicle there.
	//! @return the spawned vehicle, or null with an ERROR line when the prefab fails to load or spawn
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

	//! How many entities of `prefab` the WORLD holds at this roster position.
	//!
	//! This is the census that proves the double-spawn guard, and it deliberately does not consult
	//! our own bookkeeping: the engine query asks the world itself. One authored vehicle must
	//! answer 1. A guard that failed would answer 2, because the roster row and its `entities[]` twin
	//! carry the same position by construction.
	//! @return the entity count; 0 when there is no world
	protected static int CountWorldVehiclesAt(ResourceName prefab, float x, float z)
	{
		vector mins = Vector(x - CENSUS_XZ_M, -CENSUS_Y_M, z - CENSUS_XZ_M);
		vector maxs = Vector(x + CENSUS_XZ_M, CENSUS_Y_M, z + CENSUS_XZ_M);
		array<IEntity> hits = {};
		return TBD_EntityQuery.CollectPrefabInBox(prefab, mins, maxs, hits);
	}
}
