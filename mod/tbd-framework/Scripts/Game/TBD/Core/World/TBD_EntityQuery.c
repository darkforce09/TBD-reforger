/**
 * @file TBD_EntityQuery.c
 * @brief World box queries: the first vehicle near a point, and every entity of one prefab.
 *
 * Role: wraps `BaseWorld.QueryEntitiesByAABB`, whose callback is a plain function, behind calls
 * that return their answer.  Position: called by the vehicle state, spawn manager, waypoint
 * runtime, mission vehicle census, objective registry and trigger runtime on the server.
 * State: the static scratch the callbacks write into, set and cleared inside each call; queries
 * are synchronous, so no two calls overlap.  Invariants: the scratch is empty between calls;
 * characters never count as vehicles; a zone query keeps only origins inside the zone's shape.
 */

//! Synchronous world queries over axis-aligned boxes.
class TBD_EntityQuery
{
	protected static IEntity s_FirstVehicle; //!< first vehicle hit of the running query; null between calls
	protected static ResourceName s_WantedPrefab; //!< prefab the running prefab query keeps; empty between calls
	protected static ref array<IEntity> s_aPrefabHits; //!< hits of the running prefab query; null between calls

	//! The first vehicle whose entity lies in the box centred on (`x`, `z`).
	//! @param halfWidthM half the box's X and Z extent, metres
	//! @param halfHeightM half the box's Y extent, metres, centred on Y = 0
	//! @return the first non-character `Vehicle` the engine reports, or null for none or no world
	static IEntity FirstVehicleNear(float x, float z, float halfWidthM, float halfHeightM)
	{
		BaseWorld world = GetGame().GetWorld();
		if (!world)
			return null;

		s_FirstVehicle = null;
		vector mins = Vector(x - halfWidthM, -halfHeightM, z - halfWidthM);
		vector maxs = Vector(x + halfWidthM, halfHeightM, z + halfWidthM);
		world.QueryEntitiesByAABB(mins, maxs, OnVehicleCandidate);

		IEntity hit = s_FirstVehicle;
		s_FirstVehicle = null;
		return hit;
	}

	//! Collect every entity spawned from `prefab` inside the box.
	//! @param outHits receives the hits, in engine order
	//! @return how many hits were added; 0 when there is no world
	static int CollectPrefabInBox(ResourceName prefab, vector mins, vector maxs, notnull array<IEntity> outHits)
	{
		BaseWorld world = GetGame().GetWorld();
		if (!world)
			return 0;

		int before = outHits.Count();
		s_WantedPrefab = prefab;
		s_aPrefabHits = outHits;
		world.QueryEntitiesByAABB(mins, maxs, OnPrefabCandidate);
		s_aPrefabHits = null;
		s_WantedPrefab = ResourceName.Empty;
		return outHits.Count() - before;
	}

	//! Collect every entity spawned from `prefab` whose origin lies inside `zone`'s shape. The
	//! query box is the zone's XZ bounds, `halfHeightM` above and below Y = 0.
	//! @param checkHeightBand true also applies the zone's authored height band
	//! (`TBD_ZoneVolume.ContainsOrigin`); false tests the XZ shape only (`TBD_Zone.Contains`)
	//! @param outHits receives the hits, in engine order
	//! @return how many hits were added
	static int CollectPrefabInZone(ResourceName prefab, notnull TBD_Zone zone, float halfHeightM, bool checkHeightBand, notnull array<IEntity> outHits)
	{
		array<IEntity> boxHits = {};
		vector mins = Vector(zone.m_fMinX, -halfHeightM, zone.m_fMinZ);
		vector maxs = Vector(zone.m_fMaxX, halfHeightM, zone.m_fMaxZ);
		CollectPrefabInBox(prefab, mins, maxs, boxHits);

		int added = 0;
		foreach (IEntity hit : boxHits)
		{
			vector origin = hit.GetOrigin();
			bool inside;
			if (checkHeightBand)
				inside = TBD_ZoneVolume.ContainsOrigin(zone, origin);
			else
				inside = zone.Contains(origin[0], origin[2]);

			if (!inside)
				continue;

			outHits.Insert(hit);
			added++;
		}

		return added;
	}

	//! Query callback for `FirstVehicleNear`: keeps the first vehicle and stops the query.
	//! @return false to stop once a vehicle is kept
	protected static bool OnVehicleCandidate(IEntity entity)
	{
		if (!entity)
			return true;
		if (ChimeraCharacter.Cast(entity))
			return true;
		if (!Vehicle.Cast(entity))
			return true;

		s_FirstVehicle = entity;
		return false;
	}

	//! Query callback for `CollectPrefabInBox`: keeps entities spawned from the wanted prefab.
	//! @return true, so the query visits every entity
	protected static bool OnPrefabCandidate(IEntity entity)
	{
		if (!entity || !s_aPrefabHits)
			return true;

		EntityPrefabData prefabData = entity.GetPrefabData();
		if (!prefabData)
			return true;

		if (prefabData.GetPrefabName() != s_WantedPrefab)
			return true;

		s_aPrefabHits.Insert(entity);
		return true;
	}
}
