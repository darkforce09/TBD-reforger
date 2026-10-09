/**
 * @file TBD_ReadyDeploy.c
 * @brief Ready and Continue: put this player into a body now, by the slot or, in PIE, a walk-on.
 *
 * Role: answers the briefing's primary button.  Position: the modded
 * SCR_PlayerController.TBD_RequestReadyDeploy asks through TBD_SpawnManager.DeployOnReady on the
 * authority.
 * State: none; deploy results live on TBD_DeployExecutor.
 * Invariants: a spent life is refused; a loaded framework mission deploys by the slot path (PIE
 * advances LOBBY to BRIEFING first); only PIE falls back to a walk-on body (a vanilla rifleman on dry
 * random ground), and never while the platform decides the player's event seat.
 */

//! The Ready and Continue deploy of the spawn manager.
class TBD_ReadyDeploy : Managed
{
	protected static const ResourceName WALK_ON_PREFAB = "{26A9756790131354}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Rifleman.et"; //!< walk-on body: vanilla US rifleman
	protected const int WALK_ON_SAMPLES = 64; //!< random terrain samples before falling back to the game mode origin
	protected const float WALK_ON_EDGE_MARGIN_M = 250.0; //!< keep samples this far (m) inside the world bounds; the edge is sea
	protected const float WALK_ON_MIN_ALTITUDE_M = 2.0; //!< lowest terrain height (m) that counts as dry; sea level is 0

	protected TBD_SpawnManager m_Spawn; //!< owning manager

	//! Bind the ready deploy to its manager.
	void TBD_ReadyDeploy(TBD_SpawnManager spawn)
	{
		m_Spawn = spawn;
	}

	//! Put `playerId` into a body now. While the platform decides the event seat, or cannot, the
	//! player is told so.
	//! @param why set to the refusal for the button label
	//! @return true when the player now has a body
	//! @authority server
	bool DeployOnReady(int playerId, out string why)
	{
		TBD_DeployExecutor executor = m_Spawn.GetDeployExecutor();
		executor.ForgetDeployResult(playerId);
		bool deployed = DeployNow(playerId, why);
		if (deployed || !executor.AwaitsPlatform(playerId))
			return deployed;

		why = "checking your seat with the TBD platform - you deploy once it allows it";
		if (executor.LastDeployResult(playerId) == TBD_EDeployResult.UNAUTHORIZED)
			why = "your seat cannot be authorized right now - the reason is in your chat";

		Print(string.Format("[TBD][Spawn] ready player=%1 -> path=slot result=%2 (%3)", playerId,
			typename.EnumToString(TBD_EDeployResult, executor.LastDeployResult(playerId)), why));
		return false;
	}

	//! The deploy itself: the slot path for a loaded mission, else (PIE only) a walk-on; one
	//! `[TBD][Spawn] ready` line per press.
	//! @authority server
	protected bool DeployNow(int playerId, out string why)
	{
		why = string.Empty;
		if (TBD_Authority.IsClient())
		{
			why = "not the authority";
			return false;
		}

		bool pie = RplSession.Mode() == RplMode.None;
		bool missionLoaded = TBD_MissionLoader.IsLoaded() && TBD_MissionLoader.IsValid();
		TBD_FrameworkManager framework = TBD_FrameworkManager.GetInstance();
		TBD_EGameStage stage = m_Spawn.GetStage();
		if (framework)
			stage = framework.GetStage();

		string mode = typename.EnumToString(RplMode, RplSession.Mode());
		string stageName = typename.EnumToString(TBD_EGameStage, stage);

		if (m_Spawn.IsOneLife() && m_Spawn.IsPlayerDead(playerId))
		{
			why = "one life spent -- only an admin respawn puts you back in";
			Print(string.Format("[TBD][Spawn] ready player=%1 mode=%2 stage=%3 -> REFUSED (%4)", playerId, mode, stageName, why), LogLevel.WARNING);
			return false;
		}

		string skipped = "no mission document loaded";
		if (missionLoaded)
		{
			if (pie && stage == TBD_EGameStage.LOBBY && framework)
			{
				framework.SetStage(TBD_EGameStage.BRIEFING);
				if (framework.GetStage() != TBD_EGameStage.BRIEFING)
					Print(string.Format("[TBD][Spawn] ready player=%1 -- PIE advance LOBBY->BRIEFING refused: %2", playerId, framework.GetLastStageRefusal()), LogLevel.WARNING);
				stageName = typename.EnumToString(TBD_EGameStage, framework.GetStage());
			}

			TBD_EDeployResult r = m_Spawn.DeployPlayerEx(playerId);
			string resultName = typename.EnumToString(TBD_EDeployResult, r);
			if (r == TBD_EDeployResult.DEPLOYED || r == TBD_EDeployResult.ALREADY)
			{
				m_Spawn.GetDeployWaves().MarkHolderDeployed(playerId);
				Print(string.Format("[TBD][Spawn] ready player=%1 mode=%2 stage=%3 -> path=slot result=%4", playerId, mode, stageName, resultName));
				return true;
			}

			if (!pie)
			{
				if (r == TBD_EDeployResult.RETRY)
				{
					m_Spawn.GetDeployWaves().ScheduleDeployRetry(playerId);
					why = "deploying -- bodies still settling";
				}
				else if (r == TBD_EDeployResult.FAILED && stage == TBD_EGameStage.LOBBY)
				{
					why = "bodies wait for BRIEFING -- an admin advances the stage";
				}
				else
				{
					why = "deploy " + resultName + " -- see the server log";
				}

				Print(string.Format("[TBD][Spawn] ready player=%1 mode=%2 stage=%3 -> path=slot result=%4 (%5)", playerId, mode, stageName, resultName, why), LogLevel.WARNING);
				return false;
			}

			skipped = "slot path " + resultName;
		}
		else if (!pie)
		{
			why = skipped;
			Print(string.Format("[TBD][Spawn] ready player=%1 mode=%2 stage=%3 -> REFUSED (%4)", playerId, mode, stageName, why), LogLevel.WARNING);
			return false;
		}

		// PIE walk-on: any character, anywhere dry.
		SCR_PlayerController pc = SCR_PlayerController.Cast(GetGame().GetPlayerManager().GetPlayerController(playerId));
		if (!pc)
		{
			why = "no player controller";
			Print(string.Format("[TBD][Spawn] ready player=%1 -- walk-on refused: no player controller", playerId), LogLevel.ERROR);
			return false;
		}

		IEntity body = SpawnWalkOnBody(playerId, why);
		if (!body)
			return false;

		m_Spawn.GetDeployExecutor().HandPlayerOntoBody(pc, body, playerId, m_Spawn.GetBodies().BodyFactionKey(body), "walk-on");
		m_Spawn.GetDeployWaves().MarkHolderDeployed(playerId);
		vector pos = body.GetOrigin();
		Print(string.Format("[TBD][Spawn] ready player=%1 mode=%2 stage=%3 -> path=walk-on pos=%4 %5 %6 reason=%7",
			playerId, mode, stageName, Math.Round(pos[0]), Math.Round(pos[1]), Math.Round(pos[2]), skipped));
		return true;
	}

	//! Spawn a walk-on body: the slot body recipe without anything a slot carries. Refused while
	//! the platform decides the player's seat.
	//! @param why set on failure
	//! @authority server
	protected IEntity SpawnWalkOnBody(int playerId, out string why)
	{
		if (m_Spawn.GetDeployExecutor().AwaitsPlatform(playerId))
		{
			why = "the TBD platform decides this seat - no walk-on";
			return null;
		}

		Resource resource = Resource.Load(WALK_ON_PREFAB);
		if (!resource || !resource.IsValid())
		{
			why = "walk-on prefab failed to load";
			Print(string.Format("[TBD][Spawn] walk-on player=%1 -- prefab failed to load: %2", playerId, WALK_ON_PREFAB), LogLevel.ERROR);
			return null;
		}

		vector pos;
		if (!FindDryLandPoint(pos))
		{
			IEntity owner = m_Spawn.GetOwner();
			if (owner)
				pos = owner.GetOrigin();
			pos[1] = GetGame().GetWorld().GetSurfaceY(pos[0], pos[2]);
			Print(string.Format("[TBD][Spawn] walk-on player=%1 -- no dry sample in %2 rolls, falling back to the game mode origin", playerId, WALK_ON_SAMPLES), LogLevel.WARNING);
		}
		pos[1] = pos[1] + TBD_SlotBodyMaterializer.CAPSULE_GROUND_OFFSET_M;

		EntitySpawnParams params = new EntitySpawnParams();
		params.TransformMode = ETransformMode.WORLD;
		Math3D.MatrixIdentity4(params.Transform);
		params.Transform[3] = pos;
		float yawRad = Math.RandomFloat(0, Math.PI2);
		params.Transform[0] = Vector(Math.Cos(yawRad), 0, Math.Sin(yawRad));
		params.Transform[2] = Vector(-Math.Sin(yawRad), 0, Math.Cos(yawRad));

		IEntity body = GetGame().SpawnEntityPrefab(resource, GetGame().GetWorld(), params);
		if (!body)
		{
			why = "walk-on body failed to spawn";
			Print(string.Format("[TBD][Spawn] walk-on player=%1 -- SpawnEntityPrefab failed for %2", playerId, WALK_ON_PREFAB), LogLevel.ERROR);
			return null;
		}

		m_Spawn.GetBodies().DisableBodyAI(body);
		return body;
	}

	//! A random point on dry terrain (above sea level, not under water) inside the world bounds
	//! minus the edge margin.
	//! @return false after WALK_ON_SAMPLES wet samples
	protected bool FindDryLandPoint(out vector pos)
	{
		BaseWorld world = GetGame().GetWorld();
		if (!world)
			return false;

		vector mins;
		vector maxs;
		world.GetBoundBox(mins, maxs);
		float minX = mins[0] + WALK_ON_EDGE_MARGIN_M;
		float maxX = maxs[0] - WALK_ON_EDGE_MARGIN_M;
		float minZ = mins[2] + WALK_ON_EDGE_MARGIN_M;
		float maxZ = maxs[2] - WALK_ON_EDGE_MARGIN_M;
		if (minX >= maxX || minZ >= maxZ)
			return false;

		for (int i = 0; i < WALK_ON_SAMPLES; i++)
		{
			float x = Math.RandomFloat(minX, maxX);
			float z = Math.RandomFloat(minZ, maxZ);
			float y = world.GetSurfaceY(x, z);
			if (y < WALK_ON_MIN_ALTITUDE_M)
				continue;

			vector candidate = Vector(x, y, z);
			if (ChimeraWorldUtils.TryGetWaterSurfaceSimple(world, candidate))
				continue;

			pos = candidate;
			return true;
		}

		return false;
	}
}
