/**
 * @file TBD_SlotBodyMaterializer.c
 * @brief Slot bodies: one dressed, AI-disabled body per mission slot at its authored transform.
 *
 * Role: spawns the numbered slot lineup at mission load and a fresh body for every new life, and
 * remembers which body stands on which slot and whom it was handed to.  Position: owned by
 * TBD_SpawnManager; MaterializeSlotBodies runs from TBD_FrameworkManager's roster settle; the deploy
 * path takes and rematerializes bodies; TBD_MissionVehicleRoster seats crews into them.
 * State: slot key to standing body, slot key to the bind key it was handed to, and the
 * materialized flag; server only.
 * Invariants: materialized becomes true only through TBD_SlotLoadoutSettle; a body is reused only
 * alive and by the identity it was handed to; an abandoned body stays in the world (corpses stay).
 */

//! Slot body spawning and bookkeeping of the spawn manager.
class TBD_SlotBodyMaterializer : Managed
{
	static const float CAPSULE_GROUND_OFFSET_M = 0.0; //!< height (m) added so the capsule stands feet on ground; measured on a human character spawn

	protected const float MAX_Y_DELTA_M = 2.0; //!< warn threshold (m) between an authored y and the live terrain surface

	protected TBD_SpawnManager m_Spawn; //!< owning manager
	protected ref map<string, IEntity> m_mSlotBodies; //!< slot key to the body standing on it
	protected ref map<string, string> m_mBodyBoundTo; //!< slot key to the bind key its body was handed to
	protected bool m_bSlotBodiesMaterialized; //!< true once the lineup passed the loadout settle; default false

	//! Bind the materializer to its manager.
	void TBD_SlotBodyMaterializer(TBD_SpawnManager spawn)
	{
		m_Spawn = spawn;
		m_mSlotBodies = new map<string, IEntity>();
		m_mBodyBoundTo = new map<string, string>();
	}

	//! Cancel the pending AI re-checks.
	void CancelCallbacks()
	{
		GetGame().GetCallqueue().Remove(DisableBodyAIRecheck);
	}

	//! True once the lineup stands and passed the loadout settle.
	bool AreMaterialized()
	{
		return m_bSlotBodiesMaterialized;
	}

	//! Open spawning: called by TBD_SlotLoadoutSettle when the lineup is playable.
	void MarkMaterialized()
	{
		m_bSlotBodiesMaterialized = true;
	}

	//! The body standing on `slotKey`, or null.
	IEntity GetSlotBody(string slotKey)
	{
		return m_mSlotBodies.Get(slotKey);
	}

	//! Record `body` as the one standing on `slotKey`.
	void SetSlotBody(string slotKey, IEntity body)
	{
		m_mSlotBodies.Set(slotKey, body);
	}

	//! How many slot bodies are recorded.
	int CountBodies()
	{
		return m_mSlotBodies.Count();
	}

	//! True when the body on `slotKey` was handed to a key other than `bindKey`.
	bool IsBoundToAnother(string slotKey, string bindKey)
	{
		string boundTo;
		return m_mBodyBoundTo.Find(slotKey, boundTo) && boundTo != bindKey;
	}

	//! Record that the body on `slotKey` now belongs to `bindKey`.
	void BindBody(string slotKey, string bindKey)
	{
		m_mBodyBoundTo.Set(slotKey, bindKey);
	}

	//! Stop trusting the body on `slotKey`: the next deploy there rematerializes one.
	void ForgetBody(string slotKey)
	{
		m_mSlotBodies.Remove(slotKey);
		m_mBodyBoundTo.Remove(slotKey);
	}

	//! Spawn the whole lineup once, seat the authored vehicle crews, apply the vehicle and entity
	//! defaults, then arm the loadout settle. No-op once materialized, settling or refused.
	//! @authority server
	void MaterializeSlotBodies()
	{
		TBD_SlotLoadoutSettle settle = m_Spawn.GetLoadoutSettle();
		if (m_bSlotBodiesMaterialized || settle.IsPending() || settle.IsRefused())
			return;

		array<ref TBD_MissionSlotStruct> slots = TBD_MissionLoader.GetSlots();
		if (!slots || slots.IsEmpty())
		{
			Print("[TBD] SpawnManager: no mission slots -- cannot materialize bodies.", LogLevel.ERROR);
			return;
		}

		int built = 0;
		int loadouts = 0;
		int kitOnly = 0;
		int failed = 0;
		int number = 0;
		foreach (TBD_MissionSlotStruct slot : slots)
		{
			if (!slot)
				continue;
			number++;

			IEntity body = SpawnSlotBody(slot, number);
			if (!body)
			{
				failed++;
				continue;
			}

			m_mSlotBodies.Set(slot.Key(), body);
			built++;
			if (HasAuthoredLoadout(slot.loadout))
				loadouts++;
			else
				kitOnly++;
		}
		Print(string.Format("[TBD][Slots] materialized %1/%2 bodies -- %3 with a JSON loadout, %4 kit-only, %5 failed",
			built, number, loadouts, kitOnly, failed));
		if (failed > 0)
			Print(string.Format("[TBD][Slots] %1 of %2 slot bodies FAILED to materialize -- see the kit resolve / prefab errors above",
				failed, number), LogLevel.ERROR);

		TBD_MissionVehicleRoster.SeatAuthoredCrews(m_Spawn);
		// Cargo before the authored vehicle states so authored ammo scales the inserted magazines;
		// default fuel after, so authored fuel wins.
		TBD_VehicleSpawnDefaults.ApplyCargo();
		TBD_VehicleState.ApplySpawned();
		TBD_VehicleSpawnDefaults.ApplyDefaultFuel();
		TBD_EntityState.ApplySpawned();

		if (built <= 0)
			return;

		settle.Arm();
	}

	//! Spawn one body for `slot`: kit prefab at the scattered authored transform, AI parked unless
	//! the waypoint runtime needs it, identity applied, then the loadout pass (authored equip, or the
	//! kit worn audit) tracked by the settle.
	//! @param number the lineup number for the log; 0 for a rematerialized body
	//! @return the body, or null on a kit or prefab failure (logged ERROR)
	//! @authority server
	IEntity SpawnSlotBody(TBD_MissionSlotStruct slot, int number)
	{
		bool kitOk;
		ResourceName prefab = TBD_Registry.Resolve(slot.kit, kitOk);
		if (!kitOk || prefab.IsEmpty())
		{
			Print("[TBD] SpawnManager: kit resolve failed: " + slot.kit, LogLevel.ERROR);
			return null;
		}

		Resource resource = Resource.Load(prefab);
		if (!resource || !resource.IsValid())
		{
			Print("[TBD] SpawnManager: kit prefab failed to load: " + prefab, LogLevel.ERROR);
			return null;
		}

		float x = slot.x;
		float z = slot.z;
		vector scattered = TBD_PlacementScatter.ForSlot(slot.Key(), slot.id, slot.faction, slot.groupCallsign, x, z);
		x = scattered[0];
		z = scattered[2];

		// An authored y wins, else the live terrain surface; both get the capsule offset.
		float surfaceY = GetGame().GetWorld().GetSurfaceY(x, z);
		float spawnY = surfaceY;
		float delta = 0;
		string jsonYLabel = "-";
		if (slot.HasJsonY())
		{
			spawnY = slot.y;
			delta = Math.AbsFloat(slot.y - surfaceY);
			jsonYLabel = slot.y.ToString();
			if (delta > MAX_Y_DELTA_M)
				Print(string.Format("[TBD][Spawn] slot=%1 jsonY=%2 deviates %3 m from surfaceY=%4 (> %5 m) -- stale DEM or mis-authored slot?",
					slot.id, slot.y, delta, surfaceY, MAX_Y_DELTA_M), LogLevel.WARNING);
		}
		spawnY += CAPSULE_GROUND_OFFSET_M;

		vector pos = Vector(x, spawnY, z);

		EntitySpawnParams params = new EntitySpawnParams();
		params.TransformMode = ETransformMode.WORLD;
		Math3D.MatrixIdentity4(params.Transform);
		params.Transform[3] = pos;

		float yawRad = slot.headingDeg * Math.DEG2RAD;
		params.Transform[0] = Vector(Math.Cos(yawRad), 0, Math.Sin(yawRad));
		params.Transform[2] = Vector(-Math.Sin(yawRad), 0, Math.Cos(yawRad));

		IEntity body = GetGame().SpawnEntityPrefab(resource, GetGame().GetWorld(), params);
		if (!body)
		{
			Print("[TBD] SpawnManager: failed to spawn slot body for " + slot.id, LogLevel.ERROR);
			return null;
		}

		// Deactivate once plus a next-frame re-check; a waypointed seat respawned in LIVE keeps
		// its AI so the waypoint runtime has a subject.
		if (!TBD_WaypointRuntime.ShouldEnableAIAtSpawn(slot))
			DisableBodyAI(body);

		TBD_SlotBodyDressing.ApplySlotIdentity(body, slot);

		Print(string.Format("[TBD][Slots] Slot-%1 %2 (%3) kit %4 at %5",
			number, slot.Key(), slot.id, slot.kit, pos.ToString()));
		Print(string.Format("[TBD][Spawn] slot=%1 Y=%2 jsonY=%3 surfaceY=%4 delta=%5 heading=%6",
			slot.id, spawnY, jsonYLabel, surfaceY, delta, slot.headingDeg));

		TBD_SlotLoadoutSettle settle = m_Spawn.GetLoadoutSettle();
		settle.PruneDoneLoadoutApps();
		TBD_LoadoutApplication app = new TBD_LoadoutApplication(body, slot.loadout, "[TBD][Loadout][Slot]", slot.id);
		settle.Track(app);
		if (HasAuthoredLoadout(slot.loadout))
			app.Run();
		else
			app.RunKitWornAudit(slot.kit);

		return body;
	}

	//! True when `loadout` authors any content. JsonLoadContext allocates nested `ref` members even
	//! for an absent key, so presence is a non-empty scalar or a non-empty container, never non-null.
	//! Walks the same gear fields as TBD_LoadoutInventoryUtil.CountGear; a new gear field joins both.
	static bool HasAuthoredLoadout(TBD_SlotLoadoutStruct loadout)
	{
		if (!loadout)
			return false;

		if (loadout.cargo && !loadout.cargo.IsEmpty())
			return true;

		TBD_SlotGearStruct gear = loadout.gear;
		if (!gear)
			return false;

		if (!gear.primary.IsEmpty())  return true;
		if (!gear.optic.IsEmpty())    return true;
		if (!gear.magazine.IsEmpty()) return true;
		if (!gear.launcher.IsEmpty())   return true;
		if (!gear.handgun.IsEmpty())    return true;
		if (!gear.throwable.IsEmpty())  return true;
		if (!gear.uniform.IsEmpty())  return true;
		if (!gear.vest.IsEmpty())     return true;
		if (!gear.helmet.IsEmpty())   return true;
		if (!gear.pants.IsEmpty())    return true;
		if (!gear.boots.IsEmpty())    return true;
		if (!gear.handwear.IsEmpty()) return true;
		if (!gear.backpack.IsEmpty()) return true;

		return false;
	}

	//! The engine faction key a body was built with (its kit's default affiliation), or empty.
	string BodyFactionKey(IEntity body)
	{
		if (!body)
			return string.Empty;

		FactionAffiliationComponent affiliation = FactionAffiliationComponent.Cast(
			body.FindComponent(FactionAffiliationComponent));
		if (!affiliation)
			return string.Empty;

		Faction faction = affiliation.GetDefaultAffiliatedFaction();
		if (!faction)
			return string.Empty;

		return faction.GetFactionKey();
	}

	//! The engine faction key for a mission faction key (blufor US, opfor USSR, indfor FIA, civ
	//! CIV), or empty for any other.
	static string EngineFactionKey(string missionFactionKey)
	{
		switch (missionFactionKey)
		{
			case "blufor": return "US";
			case "opfor": return "USSR";
			case "indfor": return "FIA";
			case "civ": return "CIV";
		}
		return string.Empty;
	}

	//! Deactivate the body's AI agent now and once more next frame.
	void DisableBodyAI(IEntity body)
	{
		AIControlComponent aiComponent = AIControlComponent.Cast(body.FindComponent(AIControlComponent));
		if (!aiComponent)
			return;

		AIAgent agent = aiComponent.GetAIAgent();
		if (agent)
			agent.DeactivateAI();

		GetGame().GetCallqueue().Call(DisableBodyAIRecheck, aiComponent);
	}

	//! The next-frame half of DisableBodyAI.
	protected void DisableBodyAIRecheck(AIControlComponent aiComponent)
	{
		if (!aiComponent)
			return;
		AIAgent agent = aiComponent.GetAIAgent();
		if (agent)
			agent.DeactivateAI();
	}
}
