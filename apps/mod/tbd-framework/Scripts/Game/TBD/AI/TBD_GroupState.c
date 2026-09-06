//! T-678 - group AI state: combatMode, behaviour, formation, speedMode as GROUP defaults.
//!
//! == What was missing ========================================================================
//! T-706 put the four GRP attrs on `$defs/group`. Nothing read them. T-677 opened the AI spawn
//! gate for waypointed groups and applies waypoint-scoped `speedMode` / `behaviour` on each
//! waypoint. Group-level values are DEFAULTS: they apply to the live SCR_AIGroup when a waypoint
//! does not author an override. Combat-mode and formation are group-only on the wire.
//!
//! == Why a second JsonLoadContext pass =======================================================
//! `TBD_MissionOrbatGroupStruct` in Backend/TBD_MissionLoader.c declares no combatMode /
//! behaviour / formation / speedMode fields. Enfusion maps JSON keys onto NAMED class fields
//! only. This file runs its own pass over `TBD_MissionLoader.GetRawJson()` with a root that
//! declares `orbat.*.groups[]` combatMode/behaviour/formation/speedMode and nothing else.
//! Same pattern as AI/TBD_WaypointRuntime.c. MissionLoader stays out of this slice's owns list.
//!
//! == Presence, and the nested-ref landmine ===================================================
//! `JsonLoadContext.ReadValue` ALLOCATES a nested `ref <class>` field even when the JSON key is
//! ABSENT. These four attrs are STRINGS, so presence is an emptiness test (`IsEmpty()`), never
//! `if (group.combatMode)` on a nested class. Do not add a nested struct for the four keys.
//!
//! == Engine mapping (TBD vocabulary -> Reforger types, verified against 1.7 scripts) =========
//! ATTR-FIELD-GRP-COMBAT-MODE  blue|green|white|yellow|red
//!   -> EAIGroupCombatMode via SCR_AIGroupUtilityComponent.SetCombatMode
//!      blue/green = HOLD_FIRE, white = RETURN_FIRE, yellow/red = FIRE_AT_WILL
//! ATTR-FIELD-GRP-FORMATION    column|stagger_column|wedge|echelon_left|echelon_right|vee|line|file|diamond
//!   -> AIFormationComponent.SetFormation with SCR_EAIGroupFormation names
//!      (engine enum is Wedge/Line/Column/StaggeredColumn; extras map onto the nearest of those)
//! ATTR-FIELD-GRP-SPEED-MODE   limited|normal|full
//!   -> SCR_AIGroupCharactersMovementSpeedSetting at SCR_EAISettingOrigin.DEFAULT
//!      (DEFAULT=1000, WAYPOINT=4000 -- T-677's waypoint setting wins when authored)
//! ATTR-FIELD-GRP-BEHAVIOUR    careless|safe|aware|combat|stealth
//!   -> no engine behaviour-setting class (T-677 measured this). Used as the speed ceiling
//!      when `speedMode` is absent, same ladder as TBD_WaypointRuntime.SpeedFromWire.
//!
//! Groups with none of the four attrs are never collected. Absent attrs leave engine defaults.
//! This file does not spawn groups, does not ActivateAI, and does not rewrite waypoints.
//!
//! == What this file CANNOT prove =============================================================
//! The gate is `cargo xtask mod compile`. It cannot run a round. Whether a group actually holds
//! fire / walks in wedge on a dedicated server is a human checklist item.
//! @contract mission.schema.json#/$defs/group

//------------------------------------------------------------------------------------------------
//! One `$defs/group` object, only the T-678 keys. Field names are the JSON keys.
class TBD_GroupStateWireStruct
{
	string callsign;
	string combatMode; //!< Optional. blue|green|white|yellow|red.
	string behaviour;  //!< Optional. careless|safe|aware|combat|stealth.
	string formation;  //!< Optional. schema formation tokens.
	string speedMode;  //!< Optional. limited|normal|full.

	//------------------------------------------------------------------------------------------------
	//! Presence is emptiness, one test per line (Formula too complex).
	bool HasAnyAttr()
	{
		if (!combatMode.IsEmpty())
			return true;
		if (!behaviour.IsEmpty())
			return true;
		if (!formation.IsEmpty())
			return true;
		if (!speedMode.IsEmpty())
			return true;
		return false;
	}
}

//------------------------------------------------------------------------------------------------
class TBD_GroupStateFactionWireStruct
{
	ref array<ref TBD_GroupStateWireStruct> groups;
}

//------------------------------------------------------------------------------------------------
class TBD_GroupStateDocStruct
{
	ref map<string, ref TBD_GroupStateFactionWireStruct> orbat;
}

//------------------------------------------------------------------------------------------------
class TBD_GroupStateSquad
{
	string faction;
	string callsign;
	string combatMode;
	string behaviour;
	string formation;
	string speedMode;
	bool applied;
}

//------------------------------------------------------------------------------------------------
class TBD_GroupState
{
	static const string CH = "GroupState";
	static const int TICK_MS = 1000;

	static const string SPEED_LIMITED = "limited";
	static const string SPEED_NORMAL = "normal";
	static const string SPEED_FULL = "full";

	static const string BH_CARELESS = "careless";
	static const string BH_SAFE = "safe";
	static const string BH_AWARE = "aware";
	static const string BH_COMBAT = "combat";
	static const string BH_STEALTH = "stealth";

	static const string CM_BLUE = "blue";
	static const string CM_GREEN = "green";
	static const string CM_WHITE = "white";
	static const string CM_YELLOW = "yellow";
	static const string CM_RED = "red";

	static const string FORM_COLUMN = "column";
	static const string FORM_STAGGER = "stagger_column";
	static const string FORM_WEDGE = "wedge";
	static const string FORM_ECH_L = "echelon_left";
	static const string FORM_ECH_R = "echelon_right";
	static const string FORM_VEE = "vee";
	static const string FORM_LINE = "line";
	static const string FORM_FILE = "file";
	static const string FORM_DIAMOND = "diamond";

	protected static ref array<ref TBD_GroupStateSquad> s_aSquads;
	protected static bool s_bParsed;
	protected static string s_sParsedForMission;
	protected static bool s_bAnnounced;

	//------------------------------------------------------------------------------------------------
	static void Clear()
	{
		s_aSquads = null;
		s_bParsed = false;
		s_sParsedForMission = string.Empty;
		s_bAnnounced = false;
	}

	//------------------------------------------------------------------------------------------------
	protected static string CurrentMissionId()
	{
		TBD_MissionDocumentStruct doc = TBD_MissionLoader.GetMission();
		if (!doc || !doc.meta)
			return string.Empty;

		return doc.meta.id;
	}

	//------------------------------------------------------------------------------------------------
	protected static bool EnsureParsed()
	{
		string missionId = CurrentMissionId();
		if (s_bParsed && missionId == s_sParsedForMission)
			return true;

		string raw = TBD_MissionLoader.GetRawJson();
		if (raw.IsEmpty())
			return false;

		JsonLoadContext ctx = new JsonLoadContext();
		if (!ctx.LoadFromString(raw))
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	static void Tick()
	{
		if (!EnsureParsed())
			return;

		string missionId = CurrentMissionId();
		if (missionId != s_sParsedForMission)
		{
			TBD_Log.Warn(CH, string.Format("the loaded mission is '%1' but the group-state registry was built for '%2' - rebuilding",
				missionId, s_sParsedForMission));
			Clear();
			return;
		}

		if (!s_bAnnounced)
		{
			int n = 0;
			if (s_aSquads)
				n = s_aSquads.Count();
			TBD_Log.Event(CH, string.Format("parsed groups with AI state=%1 mission='%2'", n, missionId));
			s_bAnnounced = true;
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	//! Walk this squad's slot bodies until one already belongs to an SCR_AIGroup (T-677 arms
	//! waypointed groups). No group means the subjects are not AI-enabled yet -- retry next tick.
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	//! Engine enum is four names. Schema extras map onto the nearest of those four.
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

	//------------------------------------------------------------------------------------------------
	//! Group-level speed default. Origin DEFAULT so T-677's WAYPOINT setting wins per-waypoint.
	protected static void ApplySpeedDefault(SCR_AIGroup group, TBD_GroupStateSquad squad)
	{
		EMovementType speed;
		if (!SpeedFromWire(squad, speed))
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

	//------------------------------------------------------------------------------------------------
	//! speedMode wins. When it is absent, behaviour selects a speed ceiling so GRP-BEHAVIOUR is
	//! not a dead parsed field: careless/safe/stealth walk, aware runs, combat sprints.
	protected static bool SpeedFromWire(TBD_GroupStateSquad squad, out EMovementType speed)
	{
		if (squad.speedMode == SPEED_LIMITED)
		{
			speed = EMovementType.WALK;
			return true;
		}
		if (squad.speedMode == SPEED_NORMAL)
		{
			speed = EMovementType.RUN;
			return true;
		}
		if (squad.speedMode == SPEED_FULL)
		{
			speed = EMovementType.SPRINT;
			return true;
		}

		if (squad.behaviour == BH_CARELESS || squad.behaviour == BH_SAFE || squad.behaviour == BH_STEALTH)
		{
			speed = EMovementType.WALK;
			return true;
		}
		if (squad.behaviour == BH_AWARE)
		{
			speed = EMovementType.RUN;
			return true;
		}
		if (squad.behaviour == BH_COMBAT)
		{
			speed = EMovementType.SPRINT;
			return true;
		}

		return false;
	}
}

//------------------------------------------------------------------------------------------------
//! Heartbeat. Same idiom as TBD_WaypointRuntime: a modded game mode self-wires because this
//! slice cannot add a component to TBD_GameMode.et. Fenced by IsFrameworkWorld so a vanilla
//! scenario with the mod loaded schedules nothing.
modded class SCR_BaseGameMode
{
	protected bool m_bTBD_GroupStateTickArmed;

	//------------------------------------------------------------------------------------------------
	protected override void OnGameStart()
	{
		super.OnGameStart();

		TBD_GroupState.Clear();

		if (RplSession.Mode() == RplMode.Client)
			return;

		if (!TBD_FrameworkManager.IsFrameworkWorld())
			return;

		if (m_bTBD_GroupStateTickArmed)
			return;

		m_bTBD_GroupStateTickArmed = true;
		GetGame().GetCallqueue().CallLater(TBD_GroupStateTick, TBD_GroupState.TICK_MS, false);
	}

	//------------------------------------------------------------------------------------------------
	void TBD_GroupStateTick()
	{
		if (GetGame().GetGameMode() != this)
			return;

		TBD_GroupState.Tick();

		GetGame().GetCallqueue().CallLater(TBD_GroupStateTick, TBD_GroupState.TICK_MS, false);
	}
}
