/**
 * @file TBD_WaypointRuntime.c
 * @brief Arms authored waypoints: forms an AI group per waypointed squad at LIVE and issues its path.
 *
 * Role: reads `orbat.*.groups[].waypoints` through a second JSON pass, and once the round is LIVE
 * and the slot bodies exist, forms one `SCR_AIGroup` per waypointed squad from its unclaimed slot
 * bodies and issues its waypoints through `TBD_WaypointFactory`; later unclaimed bodies join the
 * group. Activation is the LIVE stage transition, the only activation the wire expresses; nothing
 * is issued in the lobby, briefing or safe start.  Position: `TBD_RuntimeHeartbeat` calls `Tick`
 * every `TICK_MS` and `Clear` at world start; `TBD_SlotBodyMaterializer` asks
 * `ShouldEnableAIAtSpawn` for each body.
 * State: the parsed squads and the mission id they were built for (static, server).
 * Invariants: player-claimed seats are never added to a group; groups without waypoints are never
 * enabled; a mission change rebuilds the registry; a squad arms once.
 */

//! One waypointed squad at runtime: its authored waypoints and the group formed for it.
class TBD_WaypointSquad
{
	string faction; //!< orbat faction key
	string callsign; //!< group callsign; the join key onto flattened slots
	ref array<ref TBD_WaypointWireStruct> waypoints; //!< the authored waypoints, in document order
	SCR_AIGroup group; //!< the formed group; null until armed
	bool armed; //!< true once the group is formed and its waypoints issued
}

//! Server-side waypoint reader and group commander.
class TBD_WaypointRuntime
{
	static const string CH = "Waypoint"; //!< `TBD_Log` channel
	static const int TICK_MS = 1000; //!< heartbeat period of `Tick`
	static const string ANNOUNCE_PARSED_KEY = "Waypoint.parsed"; //!< `TBD_AnnounceOnce` key of the once-per-mission parsed line
	static const ResourceName PREFAB_GROUP = "{000CD338713F2B5A}Prefabs/AI/Groups/Group_Base.et"; //!< ScenarioFramework default group

	protected static ref array<ref TBD_WaypointSquad> s_aSquads; //!< waypointed squads of the parsed mission; null until parsed
	protected static bool s_bParsed; //!< true once the waypoint pass has run for `s_sParsedForMission`
	protected static string s_sParsedForMission; //!< mission id the squads were parsed for

	//! Drop the parsed squads and re-arm the parsed line, so the next call parses again.
	static void Clear()
	{
		s_aSquads = null;
		s_bParsed = false;
		s_sParsedForMission = string.Empty;
		TBD_AnnounceOnce.Rearm(ANNOUNCE_PARSED_KEY);
	}

	//! @return the `faction:callsign` key of a squad
	protected static string SquadKey(string faction, string callsign)
	{
		return faction + ":" + callsign;
	}

	//! Whether an orbat group authors at least one waypoint; the slot body materializer opens the
	//! AI gate for these groups only.
	//! @return true for a waypointed group; false for an empty callsign or no mission document
	static bool GroupHasWaypoints(string faction, string callsign)
	{
		if (callsign.IsEmpty())
			return false;

		if (!EnsureParsed())
			return false;

		if (!s_aSquads)
			return false;

		string key = SquadKey(faction, callsign);
		foreach (TBD_WaypointSquad squad : s_aSquads)
		{
			if (!squad)
				continue;
			if (SquadKey(squad.faction, squad.callsign) == key)
				return true;
		}

		return false;
	}

	//! The spawn-time AI gate: true only for a seat of a waypointed group spawned while the round is
	//! LIVE (a respawned AI seat). Every other body has its AI disabled, including waypointed seats
	//! in the lobby and safe start.
	//! @return true to leave the body's AI enabled
	//! @authority server
	static bool ShouldEnableAIAtSpawn(TBD_MissionSlotStruct slot)
	{
		if (!slot)
			return false;

		if (!GroupHasWaypoints(slot.faction, slot.groupCallsign))
			return false;

		TBD_FrameworkManager fm = TBD_FrameworkManager.GetInstance();
		if (!fm)
			return false;

		return fm.GetStage() == TBD_EGameStage.LIVE;
	}

	//! Parse the waypoint subset of the current mission once per mission id. A document that is not
	//! JSON or whose root does not read arms nothing this round (one ERROR).
	//! @return false only when there is no mission document yet
	protected static bool EnsureParsed()
	{
		string missionId = TBD_MissionLoader.GetMissionId();
		if (s_bParsed && missionId == s_sParsedForMission)
			return true;

		TBD_EMissionJsonPassOutcome outcome;
		JsonLoadContext ctx = TBD_MissionJsonPass.LoadRoot(outcome);
		if (outcome == TBD_EMissionJsonPassOutcome.NO_DOCUMENT)
			return false;

		if (!ctx)
		{
			TBD_Log.Error(CH, "the mission document did not parse as JSON on the waypoint pass - no waypoint is armed this round");
			s_aSquads = new array<ref TBD_WaypointSquad>();
			s_bParsed = true;
			s_sParsedForMission = missionId;
			return true;
		}

		TBD_WaypointDocStruct doc = new TBD_WaypointDocStruct();
		if (!ctx.ReadValue("", doc))
		{
			TBD_Log.Error(CH, "the mission document parsed but its root would not read on the waypoint pass - no waypoint is armed this round");
			s_aSquads = new array<ref TBD_WaypointSquad>();
			s_bParsed = true;
			s_sParsedForMission = missionId;
			return true;
		}

		s_aSquads = new array<ref TBD_WaypointSquad>();
		if (doc.orbat)
		{
			foreach (string factionKey, TBD_WaypointFactionWireStruct faction : doc.orbat)
			{
				CollectFaction(factionKey, faction);
			}
		}

		s_bParsed = true;
		s_sParsedForMission = missionId;
		return true;
	}

	//! Collect every group of `faction` with a callsign and at least one waypoint.
	protected static void CollectFaction(string factionKey, TBD_WaypointFactionWireStruct faction)
	{
		if (!faction || !faction.groups)
			return;

		foreach (TBD_WaypointGroupWireStruct group : faction.groups)
		{
			if (!group)
				continue;
			if (group.callsign.IsEmpty())
				continue;

			// Array presence is null or empty.
			if (!group.waypoints)
				continue;
			if (group.waypoints.Count() < 1)
				continue;

			TBD_WaypointSquad squad = new TBD_WaypointSquad();
			squad.faction = factionKey;
			squad.callsign = group.callsign;
			squad.waypoints = group.waypoints;
			squad.armed = false;
			s_aSquads.Insert(squad);
		}
	}

	//! One heartbeat: parse if needed, rebuild on a mission change, announce the squad count once,
	//! then at LIVE with the slot bodies present arm each squad or add new members to it.
	//! @authority server
	static void Tick()
	{
		if (!EnsureParsed())
			return;

		string missionId = TBD_MissionLoader.GetMissionId();
		if (missionId != s_sParsedForMission)
		{
			TBD_Log.Warn(CH, string.Format("the loaded mission is '%1' but the waypoint registry was built for '%2' - rebuilding",
				missionId, s_sParsedForMission));
			Clear();
			return;
		}

		if (TBD_AnnounceOnce.Claim(ANNOUNCE_PARSED_KEY))
		{
			int n = 0;
			if (s_aSquads)
				n = s_aSquads.Count();
			TBD_Log.Event(CH, string.Format("parsed waypointed groups=%1 mission='%2'", n, missionId));
		}

		if (!s_aSquads || s_aSquads.Count() < 1)
			return;

		TBD_FrameworkManager fm = TBD_FrameworkManager.GetInstance();
		if (!fm || fm.GetStage() != TBD_EGameStage.LIVE)
			return;

		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		if (!spawn || !spawn.AreSlotBodiesMaterialized())
			return;

		foreach (TBD_WaypointSquad squad : s_aSquads)
		{
			if (!squad)
				continue;
			if (squad.armed)
			{
				AbsorbNewMembers(squad, spawn);
				continue;
			}

			ArmSquad(squad, spawn);
		}
	}

	//! Form the squad's group from its unclaimed slot bodies, give it the first member's faction,
	//! and issue its waypoints. A group that cannot spawn, or gets no waypoint, logs an ERROR and
	//! stays unarmed.
	//! @authority server
	protected static void ArmSquad(TBD_WaypointSquad squad, TBD_SpawnManager spawn)
	{
		array<IEntity> members = CollectUnclaimedBodies(squad, spawn);
		if (!members || members.Count() < 1)
			return;

		vector origin = members[0].GetOrigin();
		TBD_EAIGroupSpawnFailure failure;
		SCR_AIGroup group = TBD_AIGroupFactory.SpawnGroup(PREFAB_GROUP, origin, failure);
		if (failure == TBD_EAIGroupSpawnFailure.PREFAB_UNLOADABLE)
			TBD_Log.Error(CH, "Group_Base prefab failed to load");

		if (!group)
		{
			TBD_Log.Error(CH, string.Format("failed to spawn AI group for %1:%2 - seats stay parked",
				squad.faction, squad.callsign));
			return;
		}

		TBD_AIGroupFactory.AdoptMemberFaction(group, members[0]);

		foreach (IEntity body : members)
		{
			if (!body)
				continue;
			group.AddAIEntityToGroup(body);
		}

		if (!TBD_WaypointFactory.IssueWaypoints(group, squad))
		{
			TBD_Log.Error(CH, string.Format("group %1:%2 formed but no waypoint entity spawned - subjects exist with nothing to follow",
				squad.faction, squad.callsign));
			return;
		}

		squad.group = group;
		squad.armed = true;
		TBD_Log.Event(CH, string.Format("armed %1:%2 members=%3 waypoints=%4",
			squad.faction, squad.callsign, members.Count(), squad.waypoints.Count()));
	}

	//! Add unclaimed bodies of an armed squad that are not yet in its group (respawned AI seats).
	//! @authority server
	protected static void AbsorbNewMembers(TBD_WaypointSquad squad, TBD_SpawnManager spawn)
	{
		if (!squad.group)
			return;

		array<IEntity> members = CollectUnclaimedBodies(squad, spawn);
		if (!members)
			return;

		foreach (IEntity body : members)
		{
			if (!body)
				continue;

			AIControlComponent control = AIControlComponent.Cast(body.FindComponent(AIControlComponent));
			if (!control)
				continue;

			AIAgent agent = control.GetAIAgent();
			if (agent && agent.GetParentGroup() == squad.group)
				continue;

			squad.group.AddAIEntityToGroup(body);
		}
	}

	//! The slot bodies of the squad that no connected player is assigned to.
	//! @return the bodies; null when the mission has no slots
	protected static array<IEntity> CollectUnclaimedBodies(TBD_WaypointSquad squad, TBD_SpawnManager spawn)
	{
		array<ref TBD_MissionSlotStruct> slots = TBD_MissionLoader.GetSlots();
		if (!slots)
			return null;

		array<IEntity> members = new array<IEntity>();
		foreach (TBD_MissionSlotStruct slot : slots)
		{
			if (!slot)
				continue;
			if (slot.faction != squad.faction)
				continue;
			if (slot.groupCallsign != squad.callsign)
				continue;
			if (SlotIsPlayerClaimed(spawn, slot))
				continue;

			IEntity body = spawn.GetSlotBody(slot.Key());
			if (!body)
				continue;

			members.Insert(body);
		}

		return members;
	}

	//! Whether any connected player is assigned to `slot`.
	protected static bool SlotIsPlayerClaimed(TBD_SpawnManager spawn, TBD_MissionSlotStruct slot)
	{
		if (!spawn || !slot)
			return false;

		array<int> ids = {};
		GetGame().GetPlayerManager().GetPlayers(ids);
		foreach (int id : ids)
		{
			TBD_MissionSlotStruct assigned = spawn.GetAssignedSlot(id);
			if (!assigned)
				continue;
			if (assigned.Key() == slot.Key())
				return true;
		}

		return false;
	}
}
