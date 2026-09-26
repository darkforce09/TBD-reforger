/**
 * @file TBD_MissionWorldApplier.c
 * @brief Applies a freshly validated mission to the world: places `entities[]` and hands the
 * spectator policy to its seam.
 *
 * Role: the world-side effects of a successful load that belong to the mission document itself.
 * Position: called once per load by `TBD_MissionLoader.ParseMissionJson` after validation passes;
 * reads `TBD_MissionLoader.GetEntities` and `GetSettings`; writes to `TBD_MissionVehicleRoster`,
 * `TBD_EntityState` and `TBD_SpectatorTargets`.
 * State: none of its own; the spawned bodies are indexed by `TBD_MissionVehicleRoster` and
 * `TBD_EntityState`.  Invariants: both indexes reset on every call, before any early return, so a
 * reload never inherits the previous mission's world pointers; a row whose alias does not resolve
 * or whose prefab does not load is skipped with a log line, never retried.
 */

//! Static world application of the loaded mission.
class TBD_MissionWorldApplier
{
	//! Spawn every `entities[]` row of the valid mission so destroy-alias resolution
	//! (`TBD_ObjectiveDestroyTargets.ArmDestroyTargets`) finds the prefabs in its zone. Each alias resolves
	//! through `TBD_Registry` (loaded on demand); each spawned body is recorded under its uid and its
	//! alias|x|z fingerprint so the `vehicles[]` roster row of the same vehicle claims it instead of
	//! spawning a second copy.
	//! @authority server
	static void SpawnMissionEntities()
	{
		// Both indexes reset before the early return below, so a mission with no entities[] never
		// inherits the previous mission's world pointers.
		TBD_MissionVehicleRoster.ResetIndex();
		TBD_EntityState.ResetIndex();

		array<ref TBD_MissionEntityStruct> entities = TBD_MissionLoader.GetEntities();
		if (!entities || entities.Count() == 0)
			return;

		int spawned = 0;
		int skipped = 0;
		foreach (TBD_MissionEntityStruct ent : entities)
		{
			if (!ent || ent.alias.IsEmpty())
			{
				skipped++;
				continue;
			}

			bool ok;
			ResourceName prefab = TBD_Registry.Resolve(ent.alias, ok);
			if (!ok || prefab.IsEmpty())
			{
				Print(string.Format("[TBD][Entities] skip alias='%1' -- not in registry", ent.alias), LogLevel.WARNING);
				skipped++;
				continue;
			}

			Resource resource = Resource.Load(prefab);
			if (!resource || !resource.IsValid())
			{
				Print(string.Format("[TBD][Entities] Resource.Load failed for alias='%1' prefab=%2", ent.alias, prefab), LogLevel.ERROR);
				skipped++;
				continue;
			}

			float surfaceY = GetGame().GetWorld().GetSurfaceY(ent.x, ent.z);
			vector pos = Vector(ent.x, surfaceY, ent.z);

			EntitySpawnParams params = new EntitySpawnParams();
			params.TransformMode = ETransformMode.WORLD;
			Math3D.MatrixIdentity4(params.Transform);
			params.Transform[3] = pos;

			float yawRad = ent.headingDeg * Math.DEG2RAD;
			params.Transform[0] = Vector(Math.Cos(yawRad), 0, Math.Sin(yawRad));
			params.Transform[2] = Vector(-Math.Sin(yawRad), 0, Math.Cos(yawRad));

			IEntity body = GetGame().SpawnEntityPrefab(resource, GetGame().GetWorld(), params);
			if (!body)
			{
				Print(string.Format("[TBD][Entities] SpawnEntityPrefab failed for alias='%1'", ent.alias), LogLevel.ERROR);
				skipped++;
				continue;
			}

			spawned++;
			// Only bodies that reached the world are recorded: the roster claims them by uid or
			// fingerprint, and entity state applies health, damage, visibility and size to them.
			TBD_MissionVehicleRoster.RecordEntitySpawn(ent.uid, ent.alias, ent.x, ent.z, body);
			TBD_EntityState.RecordSpawn(ent.uid, ent.alias, ent.x, ent.z, body);
			Print(string.Format("[TBD][Entities] spawned alias='%1' at %2 heading=%3", ent.alias, pos.ToString(), ent.headingDeg));
		}

		Print(string.Format("[TBD][Entities] spawn done spawned=%1 skipped=%2", spawned, skipped));
	}

	//! Apply the authored `settings` through the seams reachable from here. `spectatorPolicy`
	//! "free" turns the spectator faction restriction off, "own_side_delayed_60s" turns it on (the
	//! delay belongs to the spectator controller), "none" and an unknown value leave the default
	//! with a WARNING. `respawn` and `nightVision: true` have no setter and are logged so the
	//! authored value is visible in the boot log.
	//! @authority server
	static void ApplyMissionSettings()
	{
		TBD_MissionSettingsStruct s = TBD_MissionLoader.GetSettings();
		if (!s)
			return;

		if (s.spectatorPolicy == "free")
		{
			TBD_SpectatorTargets.SetFactionRestricted(false);
			Print("[TBD][Settings] spectatorPolicy=free -> faction restriction OFF", LogLevel.NORMAL);
		}
		else if (s.spectatorPolicy == "own_side_delayed_60s")
		{
			TBD_SpectatorTargets.SetFactionRestricted(true);
			Print("[TBD][Settings] spectatorPolicy=own_side_delayed_60s -> faction restriction ON (delay owned by SpectatorController)", LogLevel.NORMAL);
		}
		else if (s.spectatorPolicy == "none")
		{
			Print("[TBD][Settings] spectatorPolicy=none -- no black-screen seam in MissionLoader owns; SpectatorTargets left at default", LogLevel.WARNING);
		}
		else if (!s.spectatorPolicy.IsEmpty())
		{
			Print(string.Format("[TBD][Settings] spectatorPolicy='%1' unrecognised -- SpectatorTargets left at default", s.spectatorPolicy), LogLevel.WARNING);
		}

		if (!s.respawn.IsEmpty())
			Print(string.Format("[TBD][Settings] respawn='%1' authored but no respawn-pool setter in MissionLoader owns", s.respawn), LogLevel.WARNING);

		// nightVision: bool defaults false, so only log the authored-true case as the interesting
		// one. Authored false and absent both read as false -- no NVG seam here either.
		if (s.nightVision)
			Print("[TBD][Settings] nightVision=true authored but no NVG setter in MissionLoader owns", LogLevel.WARNING);
	}
}
