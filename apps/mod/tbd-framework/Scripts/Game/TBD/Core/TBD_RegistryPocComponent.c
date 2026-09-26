/**
 * @file TBD_RegistryPocComponent.c
 * @brief Workbench check that spawns every registry alias in a row.
 *
 * Role: on play, when enabled, resolves every alias of `TBD_Registry` and spawns its prefab along
 * the x axis from an origin, logging each spawn or failure.
 * Position: a game mode component placed by hand in a test world; reads `TBD_Registry`.
 * State: the three attributes; a one-shot call-queue call two seconds after init.
 * Invariants: runs only on the server and only with `m_bRunPoc` on, which is off by default and
 * never shipped enabled.
 */

//! Component class of `TBD_RegistryPocComponent`.
[ComponentEditorProps(category: "TBD/Framework", description: "Spawns all registry POC aliases in a row for Workbench verification.")]
class TBD_RegistryPocComponentClass : SCR_BaseGameModeComponentClass {}

//! Spawns every registry alias for a visual check in Workbench.
class TBD_RegistryPocComponent : SCR_BaseGameModeComponent
{
	[Attribute("0", desc: "Run the registry POC spawn dump on play (dev only -- default OFF; do not ship enabled).")]
	bool m_bRunPoc; //!< default false

	[Attribute("0 1 0", desc: "World-space origin for POC spawns")]
	vector m_vSpawnOrigin; //!< world position, m; default 0 1 0

	[Attribute("8", desc: "Metres between each spawned alias")]
	float m_fSpacing; //!< m; default 8

	//! Schedule the spawn row two seconds after init, on the server and only when enabled.
	//! @authority server
	override void OnPostInit(IEntity owner)
	{
		super.OnPostInit(owner);

		if (TBD_Authority.IsClient())
			return;

		if (!m_bRunPoc)
			return;

		GetGame().GetCallqueue().CallLater(RunPoc, 2000, false);
	}

	//! Spawn every resolvable alias `m_fSpacing` metres apart from `m_vSpawnOrigin`.
	protected void RunPoc()
	{
		if (!TBD_Registry.Load())
			return;

		array<string> aliases = TBD_Registry.GetAllAliases();
		float offset = 0;

		foreach (string alias : aliases)
		{
			bool ok;
			ResourceName prefab = TBD_Registry.Resolve(alias, ok);
			if (!ok)
				continue;

			vector pos = m_vSpawnOrigin + Vector(offset, 0, 0);
			IEntity ent = SpawnPrefab(prefab, pos);
			if (ent)
				Print("[TBD] Registry POC spawned " + alias + " at " + pos.ToString());
			else
				Print("[TBD] Registry POC FAILED " + alias, LogLevel.ERROR);

			offset += m_fSpacing;
		}
	}

	//! @return the spawned entity at `position`, or null (with an ERROR) when the prefab does not load
	protected IEntity SpawnPrefab(ResourceName prefab, vector position)
	{
		Resource resource = Resource.Load(prefab);
		if (!resource || !resource.IsValid())
		{
			Print("[TBD] Resource.Load failed for " + prefab, LogLevel.ERROR);
			return null;
		}

		EntitySpawnParams params = new EntitySpawnParams();
		params.TransformMode = ETransformMode.WORLD;
		Math3D.MatrixIdentity4(params.Transform);
		params.Transform[3] = position;

		return GetGame().SpawnEntityPrefab(resource, GetGame().GetWorld(), params);
	}
}
