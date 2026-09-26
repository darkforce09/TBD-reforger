/**
 * @file TBD_AIGroupFactory.c
 * @brief Spawns an empty `SCR_AIGroup` from a group prefab and sets its faction.
 *
 * Role: the one group-spawn path for authored AI.  Position: called by `TBD_DynamicSpawner` and
 * `TBD_WaypointRuntime` on the server; spawns through `Game.SpawnEntityPrefab`.
 * State: none.  Invariants: never returns a non-group entity: a spawned entity that is not an
 * `SCR_AIGroup` is deleted with its children and the call fails with `NOT_A_GROUP`.
 */

//! Why `TBD_AIGroupFactory.SpawnGroup` returned null.
enum TBD_EAIGroupSpawnFailure
{
	NONE, //!< the group spawned
	PREFAB_UNLOADABLE, //!< the prefab resource did not load
	NOT_A_GROUP //!< the prefab spawned nothing, or something that is not an SCR_AIGroup
}

//! Server-side AI group construction.
class TBD_AIGroupFactory
{
	//! Spawn a group prefab at `origin`, identity rotation, in the current world.
	//! @param prefab the group prefab ResourceName
	//! @param origin the world position
	//! @param failure set to `NONE` on success, else why the call failed
	//! @return the group, or null on failure (callers log with their own channel)
	//! @authority server
	static SCR_AIGroup SpawnGroup(ResourceName prefab, vector origin, out TBD_EAIGroupSpawnFailure failure)
	{
		Resource resource = Resource.Load(prefab);
		if (!resource || !resource.IsValid())
		{
			failure = TBD_EAIGroupSpawnFailure.PREFAB_UNLOADABLE;
			return null;
		}

		EntitySpawnParams params = new EntitySpawnParams();
		params.TransformMode = ETransformMode.WORLD;
		Math3D.MatrixIdentity4(params.Transform);
		params.Transform[3] = origin;

		IEntity ent = GetGame().SpawnEntityPrefab(resource, GetGame().GetWorld(), params);
		SCR_AIGroup group = SCR_AIGroup.Cast(ent);
		if (!group)
		{
			if (ent)
				SCR_EntityHelper.DeleteEntityAndChildren(ent);

			failure = TBD_EAIGroupSpawnFailure.NOT_A_GROUP;
			return null;
		}

		failure = TBD_EAIGroupSpawnFailure.NONE;
		return group;
	}

	//! Give `group` the faction of `member`: its affiliated faction, else its default one.
	//! A null member, or one without a faction, leaves the group's faction unchanged.
	//! @authority server
	static void AdoptMemberFaction(notnull SCR_AIGroup group, IEntity member)
	{
		if (!member)
			return;

		FactionAffiliationComponent affiliation = FactionAffiliationComponent.Cast(member.FindComponent(FactionAffiliationComponent));
		if (!affiliation)
			return;

		Faction faction = affiliation.GetAffiliatedFaction();
		if (!faction)
			faction = affiliation.GetDefaultAffiliatedFaction();

		if (faction)
			group.SetFaction(faction);
	}
}
