[ComponentEditorProps(category: "TBD/Framework", description: "Server-only Phase 1 player spawner — round-robin faction assignment + spawn on SAFE_START.")]
class TBD_SpawnManagerClass : SCR_BaseGameModeComponentClass {}

//! Phase 1 player spawn. Authority-side: assigns each connecting player a faction
//! (round-robin blufor/opfor), then spawns + possesses them when the mission reaches SAFE_START.
class TBD_SpawnManager : SCR_BaseGameModeComponent
{
	protected static TBD_SpawnManager s_Instance;

	protected ref map<int, string> m_mPlayerFaction;
	protected ref set<int> m_aSpawned;
	protected int m_iRoundRobin;

	//------------------------------------------------------------------------------------------------
	void TBD_SpawnManager(IEntityComponentSource src, IEntity ent, IEntity parent)
	{
		s_Instance = this;
		m_mPlayerFaction = new map<int, string>();
		m_aSpawned = new set<int>();
	}

	//------------------------------------------------------------------------------------------------
	static TBD_SpawnManager GetInstance()
	{
		return s_Instance;
	}

	//------------------------------------------------------------------------------------------------
	override void OnPlayerConnected(int playerId)
	{
		super.OnPlayerConnected(playerId);
		AssignFaction(playerId);
	}

	//------------------------------------------------------------------------------------------------
	override void OnPlayerAuditSuccess(int playerId)
	{
		super.OnPlayerAuditSuccess(playerId);
		AssignFaction(playerId);
	}

	//------------------------------------------------------------------------------------------------
	//! Round-robin faction assignment, idempotent per player.
	protected void AssignFaction(int playerId)
	{
		if (m_mPlayerFaction.Contains(playerId))
			return;

		string faction;
		if (m_iRoundRobin % 2 == 0)
			faction = "blufor";
		else
			faction = "opfor";

		m_iRoundRobin++;
		m_mPlayerFaction.Insert(playerId, faction);
		Print("[TBD] SpawnManager: player " + playerId + " -> " + faction);
	}

	//------------------------------------------------------------------------------------------------
	//! Driven by TBD_FrameworkManager.SetStage. Spawns every tracked player once SAFE_START begins.
	void OnStageChanged(TBD_EGameStage stage)
	{
		if (stage != TBD_EGameStage.SAFE_START)
			return;

		foreach (int playerId, string faction : m_mPlayerFaction)
		{
			SpawnPlayer(playerId, faction);
		}
	}

	//------------------------------------------------------------------------------------------------
	protected void SpawnPlayer(int playerId, string faction)
	{
		if (m_aSpawned.Contains(playerId))
			return;

		string kitAlias = KitAliasForFaction(faction);
		if (kitAlias.IsEmpty())
		{
			Print("[TBD] SpawnManager: no kit for faction '" + faction + "'.", LogLevel.ERROR);
			return;
		}

		bool ok;
		ResourceName prefab = TBD_Registry.Resolve(kitAlias, ok);
		if (!ok)
			return;

		vector pos = TBD_MissionLoader.GetSpawnZoneForFaction(faction);

		IEntity body = SpawnPrefab(prefab, pos);
		if (!body)
		{
			Print("[TBD] SpawnManager: spawn failed for player " + playerId, LogLevel.ERROR);
			return;
		}

		SCR_PlayerController pc = SCR_PlayerController.Cast(GetGame().GetPlayerManager().GetPlayerController(playerId));
		if (pc)
			pc.SetInitialMainEntity(body);

		m_aSpawned.Insert(playerId);
		Print("[TBD] Spawned " + playerId + " at " + pos.ToString());
	}

	//------------------------------------------------------------------------------------------------
	protected string KitAliasForFaction(string faction)
	{
		if (faction == "blufor")
			return "kit:us_rifleman";

		if (faction == "opfor")
			return "kit:sov_rifleman";

		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! Mirrors TBD_RegistryPocComponent.SpawnPrefab — Resource.Load + world-space EntitySpawnParams.
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
