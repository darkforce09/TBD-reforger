[ComponentEditorProps(category: "TBD/Framework", description: "Spawns all registry POC aliases in a row for Workbench verification.")]
class TBD_RegistryPocComponentClass : SCR_BaseGameModeComponentClass {}

class TBD_RegistryPocComponent : SCR_BaseGameModeComponent
{
	[Attribute("0 1 0", desc: "World-space origin for POC spawns")]
	vector m_vSpawnOrigin;

	[Attribute("8", desc: "Metres between each spawned alias")]
	float m_fSpacing;

	//------------------------------------------------------------------------------------------------
	override void OnPostInit(IEntity owner)
	{
		super.OnPostInit(owner);

		if (RplSession.Mode() == RplMode.Client)
			return;

		GetGame().GetCallqueue().CallLater(RunPoc, 2000, false);
	}

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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
