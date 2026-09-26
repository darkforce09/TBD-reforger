/**
 * @file TBD_ObjectiveDestroyTargets.c
 * @brief Finds a destroy objective's targets at LIVE and counts how many are left.
 *
 * Role: resolves `rules.targetAlias` through `TBD_Registry`, counts the matching prefabs inside
 * the objective's zone, explains an empty search, and completes the objective once enough are
 * destroyed.  Position: `TBD_ObjectivesComponent` arms each destroy objective on its first LIVE
 * tick and `TBD_ObjectiveProgression` evaluates it every tick; queries the world through
 * `TBD_EntityQuery`.
 * State: none; writes only the objective it is given and the registry's usable counts.
 * Invariants: targets are re-queried every evaluation rather than cached as handles, so a deleted
 * target and one in `EDamageState.DESTROYED` read the same; a target without a damage manager
 * counts as alive; a target that moves out of the zone reads as destroyed.
 */

//! Destroy-objective target search and destruction count.
class TBD_ObjectiveDestroyTargets
{
	static const float QUERY_Y_EXTENT_M = 5000.0; //!< metres above and below Y = 0 of the zone query box; zones are footprints, so the box spans any terrain height

	//! Find this objective's targets. Runs once, on the first LIVE evaluation, because a target
	//! placed by another subsystem may not exist in LOBBY. `TBD_MissionLoader.SpawnMissionEntities`
	//! places the authored `entities[]` rows; terrain-placed prefabs matching the alias count too.
	//! An unresolved alias or an empty search makes the objective inert, with the reason logged,
	//! and recounts the registry's usable objectives.
	static void ArmDestroyTargets(notnull TBD_Objective objective)
	{
		objective.m_bArmed = true;

		bool resolved = false;
		ResourceName resource = TBD_Registry.Resolve(objective.m_sTargetAlias, resolved);
		if (!resolved)
		{
			objective.m_bUsable = false;
			objective.m_sInertReason = string.Format("rules.targetAlias '%1' is not in the registry, so there is no prefab to look for", objective.m_sTargetAlias);
			TBD_Log.Warn(TBD_ObjectiveRegistry.CH, string.Format("objective '%1' INERT: %2", objective.m_sId, objective.m_sInertReason));
			TBD_ObjectiveRegistry.RecountUsable();
			return;
		}

		objective.m_TargetResource = resource;
		objective.m_iTargetsFound = CountLiveTargets(objective, true);
		objective.m_iTargetsDestroyed = 0;

		if (objective.m_iTargetsFound == 0)
		{
			objective.m_bUsable = false;
			objective.m_sInertReason = DiagnoseEmptyDestroyTargets(objective);
			TBD_Log.Warn(TBD_ObjectiveRegistry.CH, string.Format("objective '%1' INERT: %2", objective.m_sId, objective.m_sInertReason));
			TBD_ObjectiveRegistry.RecountUsable();
			return;
		}

		TBD_Log.Kv(TBD_ObjectiveRegistry.CH, "armed", string.Format("id=%1 alias='%2' targets=%3 required=%4",
			objective.m_sId, objective.m_sTargetAlias, objective.m_iTargetsFound, objective.RequiredKills()));
	}

	//! Why the zone query found no match for a resolved `targetAlias`: no `entities[]` row with
	//! the alias, rows placed outside the zone, or rows inside the zone whose spawn was skipped or
	//! failed.
	//! @return the operator-facing inert reason
	protected static string DiagnoseEmptyDestroyTargets(notnull TBD_Objective objective)
	{
		string alias = objective.m_sTargetAlias;
		array<ref TBD_MissionEntityStruct> entities = TBD_MissionLoader.GetEntities();

		int authoredMatching = 0;
		int authoredInsideZone = 0;
		if (entities)
		{
			foreach (TBD_MissionEntityStruct ent : entities)
			{
				if (!ent || ent.alias != alias)
					continue;

				authoredMatching++;
				if (objective.m_Zone && objective.m_Zone.Contains(ent.x, ent.z))
					authoredInsideZone++;
			}
		}

		if (authoredMatching == 0)
		{
			return string.Format("no entity matching alias '%1' was found inside the zone at LIVE. No `entities[]` row with that alias was authored (and no terrain prefab matched) -- SpawnMissionEntities only places authored rows whose alias resolves in the registry.", alias);
		}

		if (authoredInsideZone == 0)
		{
			return string.Format("no entity matching alias '%1' was found inside the zone at LIVE. %2 `entities[]` row(s) with that alias were authored, but none sit inside this objective's zone (out-of-zone placement).", alias, authoredMatching);
		}

		return string.Format("no entity matching alias '%1' was found inside the zone at LIVE. %2 `entities[]` row(s) with that alias are authored inside the zone, so spawn likely skipped or failed for this alias -- check `[TBD][Entities]` warnings (unknown registry alias / Resource.Load / SpawnEntityPrefab).", alias, authoredInsideZone);
	}

	//! How many matching targets are inside the zone right now, by origin and height band.
	//! @param countAll true counts every match; false counts only those not destroyed
	//! @return the count; 0 when there is no world
	protected static int CountLiveTargets(notnull TBD_Objective objective, bool countAll)
	{
		array<IEntity> hits = {};
		int matched = TBD_EntityQuery.CollectPrefabInZone(objective.m_TargetResource, objective.m_Zone, QUERY_Y_EXTENT_M, true, hits);
		if (countAll)
			return matched;

		int alive = 0;
		foreach (IEntity entity : hits)
		{
			// No damage manager means nothing can destroy it, so it stays alive and the objective
			// reads incomplete.
			DamageManagerComponent damage = DamageManagerComponent.Cast(entity.FindComponent(DamageManagerComponent));
			if (!damage || damage.GetState() != EDamageState.DESTROYED)
				alive++;
		}

		return alive;
	}

	//! Count this objective's destroyed targets and complete it once `RequiredKills()` are gone.
	//! @return true only on the tick it completes, so the caller announces once
	static bool EvaluateDestroy(notnull TBD_Objective objective)
	{
		if (objective.m_bComplete || !objective.m_bUsable || !objective.m_bArmed)
			return false;

		int alive = CountLiveTargets(objective, false);
		int destroyed = objective.m_iTargetsFound - alive;
		if (destroyed < 0)
			destroyed = 0;

		objective.m_iTargetsDestroyed = destroyed;

		if (destroyed < objective.RequiredKills())
			return false;

		objective.m_bComplete = true;
		return true;
	}
}
