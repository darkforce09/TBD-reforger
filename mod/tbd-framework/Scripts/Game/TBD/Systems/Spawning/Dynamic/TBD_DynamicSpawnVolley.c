/**
 * @file TBD_DynamicSpawnVolley.c
 * @brief One LIVE tick of one dynamic spawn module: gate, volley, origin and living-group count.
 *
 * Role: decides whether a prepared module spawns this tick and spawns its volley.
 * Position: called by TBD_DynamicSpawner.Tick for every module during LIVE; groups spawn through
 * TBD_AIGroupFactory.
 * State: none of its own; it updates the TBD_SpawnModuleRuntime it is given.
 * Invariants: a trigger-gated module stays idle until its trigger FIRED; a garrison spawns once; a
 * restocking module spawns again only after its interval (never without one) and only
 * below its living cap; dead groups are pruned before every decision.
 */

//! Volley logic of the dynamic spawner.
class TBD_DynamicSpawnVolley
{
	//! Prune, gate and spawn one module for this tick.
	//! @param nowMs the world time (ms)
	//! @authority server
	static void TickModule(notnull TBD_SpawnModuleRuntime module, int index, float nowMs)
	{
		PruneDead(module);

		if (!module.m_sTriggerId.IsEmpty() && !TriggerHasFired(module))
			return;

		if (module.m_sKind == "garrison")
		{
			if (module.m_bGarrisonDone)
				return;
			SpawnVolley(module, index);
			module.m_bGarrisonDone = true;
			module.m_fLastSpawnMs = nowMs;
			return;
		}

		int living = LivingCount(module);
		if (living >= module.m_iMaxAlive)
			return;

		if (module.m_fLastSpawnMs > 0 && module.m_bHasInterval)
		{
			float elapsedS = (nowMs - module.m_fLastSpawnMs) / 1000.0;
			if (elapsedS < module.m_fIntervalS)
				return;
		}
		else if (module.m_fLastSpawnMs > 0 && !module.m_bHasInterval)
		{
			return;
		}

		SpawnVolley(module, index);
		module.m_fLastSpawnMs = nowMs;
	}

	//! Spawn up to `count` groups, never above the living cap, at the module's origin.
	//! @authority server
	protected static void SpawnVolley(notnull TBD_SpawnModuleRuntime module, int index)
	{
		vector origin;
		if (!ResolveOrigin(module, origin))
			return;

		int living = LivingCount(module);
		int want = module.m_iCount;
		int room = module.m_iMaxAlive - living;
		if (room < want)
			want = room;
		if (want <= 0)
			return;

		for (int i = 0; i < want; i++)
		{
			SCR_AIGroup group = SpawnGroup(module, origin);
			if (!group)
				continue;
			module.m_aGroups.Insert(group);
		}

			TBD_Log.Kv(TBD_DynamicSpawner.CH, "spawn", string.Format("id='%1' kind='%2' spawned=%3 alive=%4 max=%5",
			module.m_sId, module.m_sKind, want, LivingCount(module), module.m_iMaxAlive));
	}

	//! The module's spawn point on the terrain: its x and z, or its zone's centre.
	//! @return false (warned) when the zone is missing or unusable
	protected static bool ResolveOrigin(notnull TBD_SpawnModuleRuntime module, out vector origin)
	{
		float x;
		float z;
		if (module.m_bHasZone)
		{
			TBD_Zone zone = TBD_ZoneRegistry.FindById(module.m_sZoneId);
			if (!zone || !zone.IsUsable())
			{
				TBD_Log.Warn(TBD_DynamicSpawner.CH, string.Format("module '%1' zoneId '%2' is not a usable zone - skipped",
					module.m_sId, module.m_sZoneId));
				return false;
			}
			if (zone.m_eShape == TBD_EZoneShapeKind.CIRCLE)
			{
				x = zone.m_fCx;
				z = zone.m_fCz;
			}
			else
			{
				x = (zone.m_fMinX + zone.m_fMaxX) * 0.5;
				z = (zone.m_fMinZ + zone.m_fMaxZ) * 0.5;
			}
		}
		else
		{
			x = module.m_fX;
			z = module.m_fZ;
		}

		float y = GetGame().GetWorld().GetSurfaceY(x, z);
		origin = Vector(x, y, z);
		return true;
	}

	//! Spawn one group of the module's template at `origin` with the module's faction.
	//! @return the group, or null (warned) when the template does not load or is not a group
	protected static SCR_AIGroup SpawnGroup(notnull TBD_SpawnModuleRuntime module, vector origin)
	{
		TBD_EAIGroupSpawnFailure failure;
		SCR_AIGroup group = TBD_AIGroupFactory.SpawnGroup(module.m_sGroupTemplate, origin, failure);
		if (failure == TBD_EAIGroupSpawnFailure.PREFAB_UNLOADABLE)
		{
			TBD_Log.Warn(TBD_DynamicSpawner.CH, string.Format("module '%1' groupTemplate failed to load", module.m_sId));
			return null;
		}

		if (!group)
		{
			TBD_Log.Warn(TBD_DynamicSpawner.CH, string.Format("module '%1' prefab is not an SCR_AIGroup", module.m_sId));
			return null;
		}

		ApplyFaction(group, module.m_sFactionKey);
		return group;
	}

	//! Give `group` the engine faction of a mission faction key; unknown keys leave it unchanged.
	protected static void ApplyFaction(notnull SCR_AIGroup group, string factionKey)
	{
		string engineKey = TBD_SlotBodyMaterializer.EngineFactionKey(factionKey);
		if (engineKey.IsEmpty())
			return;

		SCR_FactionManager fm = SCR_FactionManager.Cast(GetGame().GetFactionManager());
		if (!fm)
			return;
		Faction faction = fm.GetFactionByKey(engineKey);
		if (faction)
			group.SetFaction(faction);
	}

	//! Groups of the module with at least one controlled body.
	protected static int LivingCount(notnull TBD_SpawnModuleRuntime module)
	{
		if (!module.m_aGroups)
			return 0;
		int n = 0;
		foreach (SCR_AIGroup group : module.m_aGroups)
		{
			if (GroupIsLiving(group))
				n++;
		}
		return n;
	}

	//! True when `group` has an agent controlling a body.
	protected static bool GroupIsLiving(SCR_AIGroup group)
	{
		if (!group)
			return false;
		array<AIAgent> agents = {};
		group.GetAgents(agents);
		if (!agents)
			return false;
		foreach (AIAgent agent : agents)
		{
			if (!agent)
				continue;
			IEntity body = agent.GetControlledEntity();
			if (body)
				return true;
		}
		return false;
	}

	//! Drop the module's groups without a living member.
	protected static void PruneDead(notnull TBD_SpawnModuleRuntime module)
	{
		if (!module.m_aGroups)
			return;
		for (int i = module.m_aGroups.Count() - 1; i >= 0; i--)
		{
			if (!GroupIsLiving(module.m_aGroups[i]))
				module.m_aGroups.Remove(i);
		}
	}

	//! True when the module's trigger FIRED; warns once when no prepared trigger has that id.
	protected static bool TriggerHasFired(notnull TBD_SpawnModuleRuntime module)
	{
		bool unknownId;
		if (TBD_TriggerRuntime.HasFired(module.m_sTriggerId, unknownId))
			return true;

		if (unknownId && !module.m_bMissingTriggerLogged)
		{
			module.m_bMissingTriggerLogged = true;
			TBD_Log.Warn(TBD_DynamicSpawner.CH, string.Format("module '%1' triggerId '%2' is not a prepared trigger - stays idle",
				module.m_sId, module.m_sTriggerId));
		}
		return false;
	}
}
