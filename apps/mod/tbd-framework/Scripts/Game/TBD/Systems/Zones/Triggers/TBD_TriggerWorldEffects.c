/**
 * @file TBD_TriggerWorldEffects.c
 * @brief The trigger effects that change the world: `spawn` and `delete`.
 *
 * Role: places copies of a registry alias at a point, and removes every entity of an alias inside
 * the trigger's zone.  Position: called by `TBD_TriggerEffects.Run`; resolves aliases through
 * `TBD_Registry` and finds entities through `TBD_EntityQuery`.
 * State: none.  Invariants: a spawn lands where `TBD_MissionLoader` places authored entities, on
 * the ground at one point; a delete never reaches past the zone's drawn shape and takes children
 * with the parent.
 */

//! World-changing trigger effects.
//! @authority server
class TBD_TriggerWorldEffects
{
	static const float QUERY_Y_EXTENT_M = 5000.0; //!< half-height in metres of the box a `delete` queries; tall enough for any terrain

	//! `spawn`: place `count` copies of the alias's prefab at one point, snapped to the ground,
	//! at `params.x`/`params.z` or the zone's centre, facing `headingDeg`. An unknown alias or a
	//! prefab that will not load spawns nothing; a short count is logged.
	//! @param trigger the firing trigger
	//! @param effect the prepared `spawn` effect
	static void Spawn(notnull TBD_Trigger trigger, notnull TBD_TriggerEffect effect)
	{
		bool ok;
		ResourceName prefab = TBD_Registry.Resolve(effect.m_sAlias, ok);
		if (!ok || prefab.IsEmpty())
		{
			TBD_Log.Error(TBD_TriggerRuntime.CH, string.Format("trigger '%1' spawn alias='%2' is not in the registry - nothing spawned",
				trigger.m_sId, effect.m_sAlias));
			return;
		}

		Resource resource = Resource.Load(prefab);
		if (!resource || !resource.IsValid())
		{
			TBD_Log.Error(TBD_TriggerRuntime.CH, string.Format("trigger '%1' spawn alias='%2' prefab=%3 failed Resource.Load - nothing spawned",
				trigger.m_sId, effect.m_sAlias, prefab));
			return;
		}

		BaseWorld world = GetGame().GetWorld();
		if (!world)
			return;

		float px = effect.m_fX;
		float pz = effect.m_fZ;
		if (px == TBD_TriggerParamsStruct.ABSENT || pz == TBD_TriggerParamsStruct.ABSENT)
		{
			// The validator refuses an effect with neither coordinates nor a zone.
			px = ZoneCentreX(trigger.m_Zone);
			pz = ZoneCentreZ(trigger.m_Zone);
		}

		float surfaceY = world.GetSurfaceY(px, pz);
		vector pos = Vector(px, surfaceY, pz);

		float yawRad = effect.m_fHeadingDeg * Math.DEG2RAD;

		int spawned = 0;
		for (int i = 0; i < effect.m_iCount; i++)
		{
			EntitySpawnParams params = new EntitySpawnParams();
			params.TransformMode = ETransformMode.WORLD;
			Math3D.MatrixIdentity4(params.Transform);
			params.Transform[3] = pos;
			params.Transform[0] = Vector(Math.Cos(yawRad), 0, Math.Sin(yawRad));
			params.Transform[2] = Vector(-Math.Sin(yawRad), 0, Math.Cos(yawRad));

			IEntity body = GetGame().SpawnEntityPrefab(resource, world, params);
			if (body)
				spawned++;
		}

		if (spawned < effect.m_iCount)
		{
			TBD_Log.Error(TBD_TriggerRuntime.CH, string.Format("trigger '%1' spawn alias='%2' produced %3 of %4 requested - SpawnEntityPrefab refused the rest",
				trigger.m_sId, effect.m_sAlias, spawned, effect.m_iCount));
		}

		TBD_Log.Kv(TBD_TriggerRuntime.CH, "spawn", string.Format("id=%1 alias='%2' at=%3 heading=%4 count=%5 spawned=%6",
			trigger.m_sId, effect.m_sAlias, pos.ToString(), effect.m_fHeadingDeg, effect.m_iCount, spawned));
	}

	//! `delete`: remove every entity of the alias's prefab whose origin is inside the trigger's
	//! zone shape, with its children. Hits are collected first and deleted after the query.
	//! @param trigger the firing trigger; its zone bounds the deletion
	//! @param effect the prepared `delete` effect
	static void Delete(notnull TBD_Trigger trigger, notnull TBD_TriggerEffect effect)
	{
		bool ok;
		ResourceName prefab = TBD_Registry.Resolve(effect.m_sAlias, ok);
		if (!ok || prefab.IsEmpty())
		{
			TBD_Log.Error(TBD_TriggerRuntime.CH, string.Format("trigger '%1' delete alias='%2' is not in the registry - nothing deleted",
				trigger.m_sId, effect.m_sAlias));
			return;
		}

		BaseWorld world = GetGame().GetWorld();
		if (!world || !trigger.m_Zone)
			return;

		array<IEntity> hits = {};
		TBD_EntityQuery.CollectPrefabInZone(prefab, trigger.m_Zone, QUERY_Y_EXTENT_M, false, hits);

		int deleted = 0;
		foreach (IEntity hit : hits)
		{
			if (!hit)
				continue;

			SCR_EntityHelper.DeleteEntityAndChildren(hit);
			deleted++;
		}

		TBD_Log.Kv(TBD_TriggerRuntime.CH, "delete", string.Format("id=%1 alias='%2' zone=%3 matched=%4 deleted=%5",
			trigger.m_sId, effect.m_sAlias, trigger.m_Zone.LogKey(), hits.Count(), deleted));
	}

	//! A zone's centre in X: the circle's centre, or the midpoint of a polygon's bounds.
	//! @param zone the zone; null returns 0 (the validator keeps a zone-less spawn from asking)
	//! @return world X in metres
	protected static float ZoneCentreX(TBD_Zone zone)
	{
		if (!zone)
			return 0;

		if (zone.m_eShape == TBD_EZoneShapeKind.CIRCLE)
			return zone.m_fCx;

		return (zone.m_fMinX + zone.m_fMaxX) * 0.5;
	}

	//! A zone's centre in Z; see `ZoneCentreX`.
	//! @param zone the zone; null returns 0
	//! @return world Z in metres
	protected static float ZoneCentreZ(TBD_Zone zone)
	{
		if (!zone)
			return 0;

		if (zone.m_eShape == TBD_EZoneShapeKind.CIRCLE)
			return zone.m_fCz;

		return (zone.m_fMinZ + zone.m_fMaxZ) * 0.5;
	}
}
