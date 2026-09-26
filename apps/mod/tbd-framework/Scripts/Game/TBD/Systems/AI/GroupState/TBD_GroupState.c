/**
 * @file TBD_GroupState.c
 * @brief Applies the authored group AI defaults (combat mode, formation, speed) to live AI groups.
 *
 * Role: reads the group AI attributes through a second JSON pass and, once the round is LIVE,
 * applies them to each squad's `SCR_AIGroup` as defaults: `combatMode` blue/green hold fire,
 * white returns fire, yellow/red fire at will (`SCR_AIGroupUtilityComponent.SetCombatMode`);
 * `formation` sets `AIFormationComponent` to the nearest of the engine's Wedge, Line, Column and
 * StaggeredColumn; `speedMode`, else the `behaviour` speed ceiling (the engine has no behaviour
 * setting), adds a movement speed setting at origin DEFAULT, below the WAYPOINT origin a
 * waypoint's own setting uses.  Position: `TBD_RuntimeHeartbeat` calls `Tick` every `TICK_MS`
 * and `Clear` at world start; the groups exist once `TBD_WaypointRuntime` arms its squads.
 * State: the parsed squads and the mission id they were built for (static, server).
 * Invariants: groups with none of the four attributes are never collected; an absent attribute
 * leaves the engine default; nothing here spawns groups, enables AI or rewrites waypoints; a
 * squad is applied once.
 */

//! One squad with authored AI attributes and whether they are applied.
class TBD_GroupStateSquad
{
	string faction; //!< orbat faction key
	string callsign; //!< group callsign; the join key onto flattened slots
	string combatMode; //!< authored combat mode token; empty when absent
	string behaviour; //!< authored behaviour token; empty when absent
	string formation; //!< authored formation token; empty when absent
	string speedMode; //!< authored speed token; empty when absent
	bool applied; //!< true once the attributes are on the live group
}

//! Server-side group AI state reader and applier.
class TBD_GroupState
{
	static const string CH = "GroupState"; //!< `TBD_Log` channel
	static const int TICK_MS = 1000; //!< heartbeat period of `Tick`
	static const string ANNOUNCE_PARSED_KEY = "GroupState.parsed"; //!< `TBD_AnnounceOnce` key of the once-per-mission parsed line

	static const string CM_BLUE = "blue"; //!< combatMode: hold fire
	static const string CM_GREEN = "green"; //!< combatMode: hold fire
	static const string CM_WHITE = "white"; //!< combatMode: return fire
	static const string CM_YELLOW = "yellow"; //!< combatMode: fire at will
	static const string CM_RED = "red"; //!< combatMode: fire at will

	static const string FORM_COLUMN = "column"; //!< formation: Column
	static const string FORM_STAGGER = "stagger_column"; //!< formation: StaggeredColumn
	static const string FORM_WEDGE = "wedge"; //!< formation: Wedge
	static const string FORM_ECH_L = "echelon_left"; //!< formation: Line
	static const string FORM_ECH_R = "echelon_right"; //!< formation: Line
	static const string FORM_VEE = "vee"; //!< formation: Wedge
	static const string FORM_LINE = "line"; //!< formation: Line
	static const string FORM_FILE = "file"; //!< formation: Column
	static const string FORM_DIAMOND = "diamond"; //!< formation: Wedge

	protected static ref array<ref TBD_GroupStateSquad> s_aSquads; //!< squads with authored attributes; null until parsed
	protected static bool s_bParsed; //!< true once the pass has run for `s_sParsedForMission`
	protected static string s_sParsedForMission; //!< mission id the squads were parsed for

	//! Drop the parsed squads and re-arm the parsed line, so the next call parses again.
	static void Clear()
	{
		s_aSquads = null;
		s_bParsed = false;
		s_sParsedForMission = string.Empty;
		TBD_AnnounceOnce.Rearm(ANNOUNCE_PARSED_KEY);
	}

	//! Parse the group attributes of the current mission once per mission id. A document that is
	//! not JSON or whose root does not read applies nothing this round (one ERROR).
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
			TBD_Log.Error(CH, "the mission document did not parse as JSON on the group-state pass - no group AI state is applied this round");
			s_aSquads = new array<ref TBD_GroupStateSquad>();
			s_bParsed = true;
			s_sParsedForMission = missionId;
			return true;
		}

		TBD_GroupStateDocStruct doc = new TBD_GroupStateDocStruct();
		if (!ctx.ReadValue("", doc))
		{
			TBD_Log.Error(CH, "the mission document parsed but its root would not read on the group-state pass - no group AI state is applied this round");
			s_aSquads = new array<ref TBD_GroupStateSquad>();
			s_bParsed = true;
			s_sParsedForMission = missionId;
			return true;
		}

		s_aSquads = new array<ref TBD_GroupStateSquad>();
		if (doc.orbat)
		{
			foreach (string factionKey, TBD_GroupStateFactionWireStruct faction : doc.orbat)
			{
				CollectFaction(factionKey, faction);
			}
		}

		s_bParsed = true;
		s_sParsedForMission = missionId;
		return true;
	}

	//! Collect every group of `faction` with a callsign and at least one attribute.
	protected static void CollectFaction(string factionKey, TBD_GroupStateFactionWireStruct faction)
	{
		if (!faction || !faction.groups)
			return;

		foreach (TBD_GroupStateWireStruct group : faction.groups)
		{
			if (!group)
				continue;
			if (group.callsign.IsEmpty())
				continue;
			if (!group.HasAnyAttr())
				continue;

			TBD_GroupStateSquad squad = new TBD_GroupStateSquad();
			squad.faction = factionKey;
			squad.callsign = group.callsign;
			squad.combatMode = group.combatMode;
			squad.behaviour = group.behaviour;
			squad.formation = group.formation;
			squad.speedMode = group.speedMode;
			squad.applied = false;
			s_aSquads.Insert(squad);
		}
	}

	//! One heartbeat: parse if needed, rebuild on a mission change, announce the squad count once,
	//! then at LIVE with the slot bodies present apply each squad whose group exists.
	//! @authority server
	static void Tick()
	{
		if (!EnsureParsed())
			return;

		string missionId = TBD_MissionLoader.GetMissionId();
		if (missionId != s_sParsedForMission)
		{
			TBD_Log.Warn(CH, string.Format("the loaded mission is '%1' but the group-state registry was built for '%2' - rebuilding",
				missionId, s_sParsedForMission));
			Clear();
			return;
		}

		if (TBD_AnnounceOnce.Claim(ANNOUNCE_PARSED_KEY))
		{
			int n = 0;
			if (s_aSquads)
				n = s_aSquads.Count();
			TBD_Log.Event(CH, string.Format("parsed groups with AI state=%1 mission='%2'", n, missionId));
		}

		if (!s_aSquads || s_aSquads.Count() < 1)
			return;

		TBD_FrameworkManager fm = TBD_FrameworkManager.GetInstance();
		if (!fm || fm.GetStage() != TBD_EGameStage.LIVE)
			return;

		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		if (!spawn || !spawn.AreSlotBodiesMaterialized())
			return;

		foreach (TBD_GroupStateSquad squad : s_aSquads)
		{
			if (!squad)
				continue;
			if (squad.applied)
				continue;

			ApplySquad(squad, spawn);
		}
	}

	//! Apply combat mode, formation and speed to the squad's live group and mark it applied; no
	//! group yet leaves it for the next tick.
	//! @authority server
	protected static void ApplySquad(TBD_GroupStateSquad squad, TBD_SpawnManager spawn)
	{
		SCR_AIGroup group = FindLiveGroup(squad, spawn);
		if (!group)
			return;

		ApplyCombatMode(group, squad);
		ApplyFormation(group, squad);
		ApplySpeedDefault(group, squad);

		squad.applied = true;
		TBD_Log.Event(CH, string.Format("applied %1:%2 combatMode='%3' behaviour='%4' formation='%5' speedMode='%6'",
			squad.faction, squad.callsign, squad.combatMode, squad.behaviour, squad.formation, squad.speedMode));
	}

	//! The `SCR_AIGroup` the first of the squad's slot bodies with an AI agent belongs to.
	//! @return the group, or null while no body is in a group yet
	protected static SCR_AIGroup FindLiveGroup(TBD_GroupStateSquad squad, TBD_SpawnManager spawn)
	{
		array<ref TBD_MissionSlotStruct> slots = TBD_MissionLoader.GetSlots();
		if (!slots)
			return null;

		foreach (TBD_MissionSlotStruct slot : slots)
		{
			if (!slot)
				continue;
			if (slot.faction != squad.faction)
				continue;
			if (slot.groupCallsign != squad.callsign)
				continue;

			IEntity body = spawn.GetSlotBody(slot.Key());
			if (!body)
				continue;

			AIControlComponent control = AIControlComponent.Cast(body.FindComponent(AIControlComponent));
			if (!control)
				continue;

			AIAgent agent = control.GetAIAgent();
			if (!agent)
				continue;

			SCR_AIGroup group = SCR_AIGroup.Cast(agent.GetParentGroup());
			if (group)
				return group;
		}

		return null;
	}

	//! Set the group's combat mode from the authored token; an unknown token or a group without a
	//! utility component is one WARNING and keeps the engine default.
	protected static void ApplyCombatMode(SCR_AIGroup group, TBD_GroupStateSquad squad)
	{
		if (squad.combatMode.IsEmpty())
			return;

		EAIGroupCombatMode mode;
		if (!CombatModeFromWire(squad.combatMode, mode))
		{
			TBD_Log.Warn(CH, string.Format("%1:%2 combatMode '%3' is not a TBD ladder token - left on engine default",
				squad.faction, squad.callsign, squad.combatMode));
			return;
		}

		SCR_AIGroupUtilityComponent utility = SCR_AIGroupUtilityComponent.Cast(group.FindComponent(SCR_AIGroupUtilityComponent));
		if (!utility)
		{
			TBD_Log.Warn(CH, string.Format("%1:%2 has no SCR_AIGroupUtilityComponent - combatMode not applied",
				squad.faction, squad.callsign));
			return;
		}

		utility.SetCombatMode(mode);
	}

	//! @return false for an unknown token, leaving `mode` untouched
	protected static bool CombatModeFromWire(string token, out EAIGroupCombatMode mode)
	{
		if (token == CM_BLUE || token == CM_GREEN)
		{
			mode = EAIGroupCombatMode.HOLD_FIRE;
			return true;
		}
		if (token == CM_WHITE)
		{
			mode = EAIGroupCombatMode.RETURN_FIRE;
			return true;
		}
		if (token == CM_YELLOW || token == CM_RED)
		{
			mode = EAIGroupCombatMode.FIRE_AT_WILL;
			return true;
		}

		return false;
	}

	//! Set the group's formation from the authored token; an unknown token or a group without a
	//! formation component is one WARNING and keeps the engine default.
	protected static void ApplyFormation(SCR_AIGroup group, TBD_GroupStateSquad squad)
	{
		if (squad.formation.IsEmpty())
			return;

		SCR_EAIGroupFormation formation;
		if (!FormationFromWire(squad.formation, formation))
		{
			TBD_Log.Warn(CH, string.Format("%1:%2 formation '%3' is not a TBD formation token - left on engine default",
				squad.faction, squad.callsign, squad.formation));
			return;
		}

		AIFormationComponent formComp = AIFormationComponent.Cast(group.FindComponent(AIFormationComponent));
		if (!formComp)
		{
			TBD_Log.Warn(CH, string.Format("%1:%2 has no AIFormationComponent - formation not applied",
				squad.faction, squad.callsign));
			return;
		}

		formComp.SetFormation(SCR_Enum.GetEnumName(SCR_EAIGroupFormation, formation));
	}

	//! Map a schema formation token onto the nearest of the engine's four formations.
	//! @return false for an unknown token, leaving `formation` untouched
	protected static bool FormationFromWire(string token, out SCR_EAIGroupFormation formation)
	{
		if (token == FORM_WEDGE || token == FORM_VEE || token == FORM_DIAMOND)
		{
			formation = SCR_EAIGroupFormation.Wedge;
			return true;
		}
		if (token == FORM_LINE || token == FORM_ECH_L || token == FORM_ECH_R)
		{
			formation = SCR_EAIGroupFormation.Line;
			return true;
		}
		if (token == FORM_COLUMN || token == FORM_FILE)
		{
			formation = SCR_EAIGroupFormation.Column;
			return true;
		}
		if (token == FORM_STAGGER)
		{
			formation = SCR_EAIGroupFormation.StaggeredColumn;
			return true;
		}

		return false;
	}

	//! Add the group-level speed default at origin DEFAULT, so a waypoint's WAYPOINT-origin setting
	//! wins while that waypoint runs.
	protected static void ApplySpeedDefault(SCR_AIGroup group, TBD_GroupStateSquad squad)
	{
		EMovementType speed;
		if (!TBD_AIWireEnums.SpeedFromWire(squad.speedMode, squad.behaviour, speed))
			return;

		SCR_AIGroupSettingsComponent settingsComp = SCR_AIGroupSettingsComponent.Cast(group.FindComponent(SCR_AIGroupSettingsComponent));
		if (!settingsComp)
		{
			TBD_Log.Warn(CH, string.Format("%1:%2 has no SCR_AIGroupSettingsComponent - speedMode/behaviour not applied",
				squad.faction, squad.callsign));
			return;
		}

		SCR_AIGroupCharactersMovementSpeedSetting setting = SCR_AIGroupCharactersMovementSpeedSetting.Create(
			SCR_EAISettingOrigin.DEFAULT, speed);
		if (!setting)
			return;

		settingsComp.AddSetting(setting, false, true);
	}
}
