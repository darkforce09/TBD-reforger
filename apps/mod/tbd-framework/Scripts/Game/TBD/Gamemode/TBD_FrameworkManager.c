//! T-181.38 — JOIN-IN-PROGRESS POLICY, straight from `flow.jip`.
//!
//! The three schema values, in order of how much of the round they let a latecomer into. Read it as
//! a ladder: each one closes the door one stage earlier than the last.
//!
//! There is no `switch` on this enum anywhere, on purpose: duplicate `switch` case labels compile
//! CLEAN in Enfusion (measured landmine), so a switch cannot be trusted to prove these values are
//! distinct. The resolvers below are explicit `if` chains against NAMED stages, which are also
//! immune to someone reordering `TBD_EGameStage`.
enum TBD_EJipPolicy
{
	//! `"always"` — a join is permitted at every deployable stage. This is TBD's behaviour BEFORE
	//! this slice (T-181.15 deliberately allowed a deploy at any stage LOBBY..LIVE), and therefore
	//! the default when the mission authors nothing: an absent field must not change behaviour.
	ALWAYS,
	//! `"until_safestart_end"` — the roster stays open through planning and warmup, and closes the
	//! moment the round goes LIVE.
	UNTIL_SAFESTART_END,
	//! `"disabled"` — the roster closes when the event starts. LOBBY only.
	DISABLED
}

//! T-181.38 — the ONE place the mission's `flow` block is turned into answers.
//!
//! ── Why a separate class and not four scattered reads ───────────────────────────────────────
//! Three different subsystems consume `flow`, and one of them (`TBD_SpawnManager`'s JIP door) is
//! owned by a different slice. Concentrating the presence rules, the defaults and the vocabulary
//! here means a consumer asks ONE question and cannot get the sentinel handling subtly wrong —
//! which is precisely the class of bug the `JsonLoadContext` landmine keeps producing.
//!
//! ── The rule that governs every accessor here ───────────────────────────────────────────────
//! `JsonLoadContext` allocates `doc.flow` even when the mission has no `flow` key, so a null test
//! on it is ALWAYS TRUE and proves nothing. Every read below therefore tests CONTENT against
//! `TBD_MissionFlowStruct.ABSENT`. The null tests that DO appear guard `doc` itself, which really
//! can be null: clients never have a mission document (`TBD_FrameworkManager.OnPostInit` returns
//! before `BeginLoad()` on `RplMode.Client`), and neither does the server before the load lands.
//!
//! ── Stateless on purpose ────────────────────────────────────────────────────────────────────
//! Nothing is cached. An admin switching missions restarts the scenario in-process, and a cached
//! flow would outlive the world it came from — the same statics-outlive-a-world landmine
//! `IsFrameworkWorld()` exists to dodge. Every call is a fresh read of the live document, and
//! nothing here sits on a per-frame path (ENF-1).
class TBD_MissionFlow
{
	//! Greppable log channel for everything this block drives. Declared locally rather than in
	//! `TBD_Log` for the same reason `TBD_BriefingService.CH_BRIEFING` and `TBD_AdminAudit` are:
	//! the tag belongs next to the code that emits it.
	static const string CH_FLOW = "Flow";

	//! Returned by the seconds accessors when the mission authored nothing usable. Deliberately
	//! NEGATIVE, because an authored `0` is a real value with real meaning (`timeLimitSeconds: 0`
	//! is "no limit" — a statement the author made) and must never collide with "said nothing".
	static const int UNSET = -1;

	//! `source` labels from ResolveSeconds. Constants rather than literals so a caller comparing
	//! against them cannot drift from the producer.
	static const string SRC_AUTHORED = "authored";
	static const string SRC_DEFAULT  = "default";
	static const string SRC_INVALID  = "INVALID";

	//------------------------------------------------------------------------------------------------
	//! The raw block, or null when no mission document exists at all.
	//!
	//! NOTE what this does NOT mean: a non-null return says nothing about whether the mission
	//! authored a `flow` key. It is allocated either way. Callers get raw fields and must test them
	//! against ABSENT — which is exactly why this is protected and the typed accessors are not.
	protected static TBD_MissionFlowStruct Block()
	{
		TBD_MissionDocumentStruct doc = TBD_MissionLoader.GetMission();
		if (!doc)
			return null;

		return doc.flow;
	}

	//------------------------------------------------------------------------------------------------
	static int RawBriefingSeconds()
	{
		TBD_MissionFlowStruct flow = Block();
		if (!flow)
			return TBD_MissionFlowStruct.ABSENT;

		return flow.briefingSeconds;
	}

	//------------------------------------------------------------------------------------------------
	static int RawSafeStartSeconds()
	{
		TBD_MissionFlowStruct flow = Block();
		if (!flow)
			return TBD_MissionFlowStruct.ABSENT;

		return flow.safeStartSeconds;
	}

	//------------------------------------------------------------------------------------------------
	static int RawTimeLimitSeconds()
	{
		TBD_MissionFlowStruct flow = Block();
		if (!flow)
			return TBD_MissionFlowStruct.ABSENT;

		return flow.timeLimitSeconds;
	}

	//------------------------------------------------------------------------------------------------
	static string RawJip()
	{
		TBD_MissionFlowStruct flow = Block();
		if (!flow)
			return string.Empty;

		return flow.jip;
	}

	//------------------------------------------------------------------------------------------------
	//! ONE resolution rule for all three durations, so absent / 0 / negative are treated the same
	//! way everywhere and the log can say which of the three it was.
	//!
	//! A negative is neither clamped nor silently defaulted: the schema declares `minimum: 0` on all
	//! three, so a negative means the PRODUCER is broken and the operator needs to hear about it. It
	//! comes back as UNSET with `source = SRC_INVALID`, and the caller reports it.
	static int ResolveSeconds(int raw, out string source)
	{
		if (raw == TBD_MissionFlowStruct.ABSENT)
		{
			source = SRC_DEFAULT;
			return UNSET;
		}

		if (raw < 0)
		{
			source = SRC_INVALID;
			return UNSET;
		}

		source = SRC_AUTHORED;
		return raw;
	}

	//------------------------------------------------------------------------------------------------
	//! Authored BRIEFING length in seconds, or UNSET. Advisory only — see
	//! `TBD_FrameworkManager.OnEnterBriefing` for why nothing auto-advances on it.
	static int BriefingSeconds()
	{
		string source;
		return ResolveSeconds(RawBriefingSeconds(), source);
	}

	//------------------------------------------------------------------------------------------------
	//! Authored safestart countdown length in seconds, or UNSET.
	static int SafeStartSeconds()
	{
		string source;
		return ResolveSeconds(RawSafeStartSeconds(), source);
	}

	//------------------------------------------------------------------------------------------------
	//! Authored round length in seconds, or UNSET. `0` is a legal and meaningful answer: the author
	//! explicitly declared NO limit. Callers must distinguish `0` from UNSET.
	static int TimeLimitSeconds()
	{
		string source;
		return ResolveSeconds(RawTimeLimitSeconds(), source);
	}

	//------------------------------------------------------------------------------------------------
	//! The resolved policy. Anything unrecognised — including the empty string an absent key leaves
	//! behind — resolves to ALWAYS, which is byte-for-byte today's behaviour. An unrecognised string
	//! is NAMED once at load (`TBD_FrameworkManager.ReportJip`) rather than swallowed here; this
	//! function is on the join path and must stay silent (ENF-1).
	static TBD_EJipPolicy JipPolicy()
	{
		return PolicyFromString(RawJip());
	}

	//------------------------------------------------------------------------------------------------
	static TBD_EJipPolicy PolicyFromString(string raw)
	{
		if (raw == "disabled")
			return TBD_EJipPolicy.DISABLED;

		if (raw == "until_safestart_end")
			return TBD_EJipPolicy.UNTIL_SAFESTART_END;

		return TBD_EJipPolicy.ALWAYS;
	}

	//------------------------------------------------------------------------------------------------
	//! True when `raw` is a value THIS BUILD understands. Used only by the load-time report, so a
	//! string the schema allows but this build does not implement gets named instead of silently
	//! collapsing into the default.
	static bool IsKnownPolicyString(string raw)
	{
		return raw == "disabled" || raw == "until_safestart_end" || raw == "always";
	}

	//------------------------------------------------------------------------------------------------
	//! The resolved policy as the schema spells it — for logs, and for the JIP door's refusal label.
	static string JipPolicyName()
	{
		TBD_EJipPolicy policy = JipPolicy();

		if (policy == TBD_EJipPolicy.DISABLED)
			return "disabled";

		if (policy == TBD_EJipPolicy.UNTIL_SAFESTART_END)
			return "until_safestart_end";

		return "always";
	}

	//------------------------------------------------------------------------------------------------
	//! ══ THE JIP DOOR'S QUESTION ══════════════════════════════════════════════════════════════
	//! May a player arriving NOW, with the round at `stage`, be put into the world?
	//!
	//! This answers the AUTHOR'S question only. It deliberately knows nothing about one life, spent
	//! lives, auto-deploy or whether slot bodies exist — those are `TBD_SpawnManager`'s guards and
	//! they all still run. A `true` here is PERMISSION, not an instruction.
	//!
	//! LOADING is permitted because this policy has no opinion about it: nothing is materialised
	//! yet, and `TBD_SpawnManager.IsStageDeployable()` already refuses that stage on world-readiness
	//! grounds. Two gates answering the same question two ways is how they drift apart.
	static bool AllowsJoinAtStage(TBD_EGameStage stage)
	{
		TBD_EJipPolicy policy = JipPolicy();

		if (policy == TBD_EJipPolicy.ALWAYS)
			return true;

		// Before the event starts, nobody is joining anything IN PROGRESS. Both remaining policies
		// allow it, and must: everyone arrives during LOBBY.
		if (stage == TBD_EGameStage.LOADING || stage == TBD_EGameStage.LOBBY)
			return true;

		// DISABLED closes the roster the moment the side starts planning together. A player who
		// arrives after that has missed the brief their squad built around them.
		if (policy == TBD_EJipPolicy.DISABLED)
			return false;

		// UNTIL_SAFESTART_END — open through planning and warmup, shut at LIVE. END and DEBRIEF fall
		// through to false, which is also what IsStageDeployable() says about them.
		return stage == TBD_EGameStage.BRIEFING || stage == TBD_EGameStage.SAFE_START;
	}

	//------------------------------------------------------------------------------------------------
	//! Comma-separated list of the stages a join is permitted in, for the load-time report.
	//!
	//! Built by ASKING `AllowsJoinAtStage`, never from a second hand-written table: a label that can
	//! disagree with the rule it describes is worse than no label at all.
	static string JoinsPermittedLabel()
	{
		string label = string.Empty;

		for (int i = TBD_EGameStage.LOBBY; i <= TBD_EGameStage.LIVE; i++)
		{
			if (!AllowsJoinAtStage(i))
				continue;

			if (!label.IsEmpty())
				label += ",";

			label += typename.EnumToString(TBD_EGameStage, i);
		}

		if (label.IsEmpty())
			return "none";

		return label;
	}
}

[ComponentEditorProps(category: "TBD/Framework", description: "TBD platform game mode manager — mission load and stage machine.")]
class TBD_FrameworkManagerClass : SCR_BaseGameModeComponentClass {}

class TBD_FrameworkManager : SCR_BaseGameModeComponent
{
	//! @replicated m_Stage — server-owned; clients react in OnStageReplicated (onRplName hook).
	[RplProp(onRplName: "OnStageReplicated")]
	protected TBD_EGameStage m_Stage = TBD_EGameStage.LOADING;

	//! A5 — roster settle ticks elapsed (500 ms cadence; 4 = the 2 s force-settle deadline).
	protected int m_iRosterSettleTicks;

	//! T-181.17 — why the last SetStage() refused, or empty. `TBD_AdminService.AdvanceStage`
	//! detects a refusal by comparing the stage either side of the call, which tells an admin THAT
	//! it was refused but not why; this carries the why to them instead of only to the console.
	protected string m_sLastStageRefusal;

	//! T-181.38 — the round clock is not running. Negative rather than 0 for the same reason
	//! `TBD_SafestartManager.NOT_RUNNING` is: a 0 would read as "about to expire".
	protected static const int ROUND_CLOCK_OFF = -1;

	//! T-181.38 — seconds left on the authored round clock (`flow.timeLimitSeconds`), or
	//! ROUND_CLOCK_OFF. SERVER-SIDE ONLY, and deliberately not an `RplProp`: a replicated clock
	//! needs a client read-out, and every TBD screen is blocked behind the `resourceDatabase.rdb`
	//! regeneration. Players are told by chat broadcast instead, which needs no menu preset and
	//! works on a dedicated server today. See the slice report for the upgrade path.
	protected int m_iRoundSecondsRemaining = ROUND_CLOCK_OFF;

	//------------------------------------------------------------------------------------------------
	//! The framework manager on the CURRENTLY loaded world, or null if this world has none.
	//!
	//! T-181.30 — this used to be `return s_Instance;` off a constructor-set static, which is the
	//! exact shape `IsFrameworkWorld()` below carries a paragraph explaining why it must never use.
	//! Statics outlive a world inside one process (measured landmine, and `SelectMissionByNumber`
	//! restarts the scenario in-process, so it is reachable here), which left a stale manager from a
	//! dead world answering for a live one. The static is now gone entirely rather than left unread:
	//! a field that does not exist cannot be stale-read by the next edit.
	//!
	//! Safe at every call site because none of them can run before the game-mode entity is complete:
	//! all 11 are ticks, RPC/chat handlers, stage transitions or per-player builds. The earliest is
	//! `TickLoading`, itself a `CallLater` registered in `OnPostInit`. `PrintComponentRollCall` —
	//! `CallLater(…, 0)` from that same `OnPostInit`, so strictly earlier than any of them — already
	//! resolves its siblings by `FindComponent` on this entity and `world-boot.sh` asserts the
	//! resulting `=ok` line, which is the runtime proof that the lookup resolves this early.
	static TBD_FrameworkManager GetInstance()
	{
		SCR_BaseGameMode gameMode = SCR_BaseGameMode.Cast(GetGame().GetGameMode());
		if (!gameMode)
			return null;

		return TBD_FrameworkManager.Cast(gameMode.FindComponent(TBD_FrameworkManager));
	}

	//------------------------------------------------------------------------------------------------
	//! True when the CURRENTLY loaded world runs the TBD framework — the guard every
	//! vanilla-suppressing modded class asks before standing vanilla down. Resolved off
	//! the live game mode for the reason spelled out on GetInstance() above; it predates
	//! that fix and is why the house idiom was already available to copy.
	static bool IsFrameworkWorld()
	{
		SCR_BaseGameMode gm = SCR_BaseGameMode.Cast(GetGame().GetGameMode());
		if (!gm)
			return false;

		return gm.FindComponent(TBD_FrameworkManager) != null;
	}

	//------------------------------------------------------------------------------------------------
	TBD_EGameStage GetStage()
	{
		return m_Stage;
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.17 — the reason the last stage transition was refused, or empty if it was not.
	//! Read by the admin surfaces so "stage unchanged" comes with the why attached.
	string GetLastStageRefusal()
	{
		return m_sLastStageRefusal;
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server — mission load and the stage machine run on the server only.
	override void OnPostInit(IEntity owner)
	{
		super.OnPostInit(owner);

		// Deferred one frame so sibling components are certainly constructed — the roll-call
		// must not report MISSING merely because it asked too early.
		GetGame().GetCallqueue().CallLater(PrintComponentRollCall, 0);

		// Authority only — clients never drive mission load or the stage machine.
		if (RplSession.Mode() == RplMode.Client)
			return;

		SetStage(TBD_EGameStage.LOADING);
		TBD_MissionLoader.BeginLoad();
		GetGame().GetCallqueue().CallLater(TickLoading, 1000, true);
	}

	//------------------------------------------------------------------------------------------------
	//! Cancel every callqueue entry this component owns.
	//!
	//! Why this is not optional: `SelectMissionByNumber` restarts the scenario IN-PROCESS via
	//! `GameStateTransitions.RequestScenarioRestart()`, and a recorded landmine in this program is
	//! that statics outlive a world inside one process. Without this, all four timers survive the
	//! teardown and fire against a dead component on the next world — `GetOwner()` returns null and
	//! the roll-call would report a phantom failure, while the tick functions would run their logic
	//! against a stale instance. `ScriptCallQueue.Remove` cancels BY FUNCTION, which is exactly
	//! right here: there is one instance of each of these per world.
	override void OnDelete(IEntity owner)
	{
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
		{
			queue.Remove(PrintComponentRollCall);
			queue.Remove(TickLoading);
			queue.Remove(TickRosterSettle);
			queue.Remove(TickWinConditions);
			// T-181.38 — the round clock is the longest-lived timer here (up to
			// flow.timeLimitSeconds — 90 minutes on bridgehead), so it is by far the most likely to
			// still be pending across an in-process scenario restart.
			queue.Remove(TickRoundClock);
		}

		super.OnDelete(owner);
	}

	//------------------------------------------------------------------------------------------------
	//! One-shot component roll-call for the entity that owns the framework.
	//!
	//! Why this exists: a component whose class fails to resolve is dropped from the prefab
	//! SILENTLY. `TBD_GameMode.et` still lists it, every script still compiles clean, and the
	//! only symptom is a feature that never runs. Three components (lobby, spectator,
	//! safestart) were added to that prefab across three separate slices with nothing anywhere
	//! proving they instantiate — the compile gate cannot see prefab wiring at all.
	//!
	//! `scripts/mod/world-boot.sh` boots the real scenario headlessly and asserts this line, so
	//! a dropped component fails the wave gate instead of surfacing mid-event. The framework
	//! manager itself is not listed: it is the thing printing, so its presence is self-evident.
	protected void PrintComponentRollCall()
	{
		IEntity owner = GetOwner();
		if (!owner)
		{
			Print("[TBD] roll-call: no owner entity — cannot enumerate components.", LogLevel.ERROR);
			return;
		}

		array<string> missing = new array<string>();
		string line = "[TBD] roll-call:";
		line += RollCallEntry(owner, TBD_SpawnManager, "SpawnManager", missing);
		line += RollCallEntry(owner, TBD_SafestartManager, "Safestart", missing);
		line += RollCallEntry(owner, TBD_LoadoutEquipComponent, "LoadoutEquip", missing);
		line += RollCallEntry(owner, TBD_SpectatorComponent, "Spectator", missing);
		line += RollCallEntry(owner, TBD_LobbyComponent, "Lobby", missing);
		line += RollCallEntry(owner, TBD_PlayAreaComponent, "PlayArea", missing);
		line += RollCallEntry(owner, TBD_MarkerComponent, "Markers", missing);
		line += RollCallEntry(owner, TBD_RadioComponent, "Radio", missing);
		line += RollCallEntry(owner, TBD_ObjectivesComponent, "Objectives", missing);

		// PrintFormat, not Print: `Print(someLocalVariable)` emits the DECLARATION
		// (`string line = '…'`) rather than the value, which made the log line awkward to match
		// and the world-boot selftest fixtures diverge from reality. Measured, not assumed.
		if (missing.IsEmpty())
		{
			PrintFormat("%1", line);
			return;
		}

		PrintFormat("%1", line, level: LogLevel.ERROR);
		Print(string.Format("[TBD] roll-call: %1 component(s) declared on TBD_GameMode.et did not instantiate.",
			missing.Count()), LogLevel.ERROR);
	}

	//------------------------------------------------------------------------------------------------
	//! One roll-call cell. Returns the " Label=ok" / " Label=MISSING" fragment and records the
	//! misses, so the caller builds the whole verdict in one line without a ternary (Enfusion
	//! Script has none).
	protected string RollCallEntry(notnull IEntity owner, typename componentType, string label, notnull array<string> missing)
	{
		if (owner.FindComponent(componentType))
			return " " + label + "=ok";

		missing.Insert(label);
		return " " + label + "=MISSING";
	}

	//------------------------------------------------------------------------------------------------
	protected void TickLoading()
	{
		if (m_Stage != TBD_EGameStage.LOADING)
		{
			GetGame().GetCallqueue().Remove(TickLoading);
			return;
		}

		if (!TBD_MissionLoader.IsLoaded())
			return;

		if (!TBD_MissionLoader.IsValid())
		{
			Print("[TBD] Mission loaded but invalid — staying in LOADING.", LogLevel.ERROR);
			return;
		}

		GetGame().GetCallqueue().Remove(TickLoading);

		// T-181.38 — BEFORE anything else consumes the document, and before the stage machine
		// leaves LOADING. `flow.safeStartSeconds` in particular has to reach TBD_SafestartManager
		// while it is still impossible for SAFE_START to have been entered.
		ApplyMissionFlow();

		TBD_Registry.Load();

		TBD_SpawnManager sm = TBD_SpawnManager.GetInstance();
		if (sm)
			sm.MaterializeSlotBodies();

		// A5 (determinism): the roster fetch must SETTLE before LOBBY so slot
		// assignment is a pure function of settled state — the old same-tick
		// BeginLoad()+SetStage(LOBBY) let the 250 ms deploy wave race the REST
		// round-trip (roster vs round-robin flipped run-to-run).
		TBD_RosterLoader.BeginLoad();
		m_iRosterSettleTicks = 0;
		GetGame().GetCallqueue().CallLater(TickRosterSettle, 500, true);
	}

	//------------------------------------------------------------------------------------------------
	//! A5 — wait for the roster to settle (loaded or failed), force-settle at the 2 s
	//! deadline, then enter LOBBY exactly once.
	protected void TickRosterSettle()
	{
		m_iRosterSettleTicks++;

		if (!TBD_RosterLoader.IsLoaded() && m_iRosterSettleTicks < 4)
			return;

		GetGame().GetCallqueue().Remove(TickRosterSettle);

		if (!TBD_RosterLoader.IsLoaded())
			TBD_RosterLoader.ForceSettle();

		Print(string.Format("[TBD][Spawn] roster settled=%1 assignments=%2",
			TBD_RosterLoader.GetSettleReason(), TBD_RosterLoader.GetAssignmentCount()));

		SetStage(TBD_EGameStage.LOBBY);
	}

	// ══ T-181.38 — THE `flow` BLOCK ═══════════════════════════════════════════════════════════
	//
	// `flow` is the mission's own statement of how the event is PACED: how long the brief runs, how
	// long the warmup runs, how long the round runs, and who may still join. Every golden mission
	// authors all four fields and, until this slice, the block was not in
	// `TBD_MissionDocumentStruct` at all — so all four were silently discarded. "JSON is the
	// contract" (TBD_MOD_DESIGN.md §2) is exactly what that failed.
	//
	// Each field is put where it actually acts, and nowhere else:
	//   * safeStartSeconds -> handed to TBD_SafestartManager through its EXISTING configured-seconds
	//                         seam, the same one `#tbd safestart <seconds>` drives.
	//   * timeLimitSeconds -> a round clock armed at LIVE, ending the round through SetStage(END) —
	//                         the SAME path faction_eliminated uses. There is exactly one way to end
	//                         a round and this slice does not add a second.
	//   * jip              -> resolved here, ENFORCED at the JIP door in TBD_SpawnManager (owned by
	//                         another slice; see TBD_MissionFlow.AllowsJoinAtStage and the report).
	//   * briefingSeconds  -> announced on entering BRIEFING. Advisory: nothing auto-advances that
	//                         stage today and this slice does not invent it. See OnEnterBriefing.

	//------------------------------------------------------------------------------------------------
	//! Read the mission's `flow` block and put each field into force. Runs ONCE per mission load.
	//!
	//! ══ EVERY TEST HERE IS ON CONTENT, NEVER ON NULL ═══════════════════════════════════════════
	//! `JsonLoadContext` ALLOCATES `doc.flow` even when the JSON has no `flow` key at all (measured
	//! 2026-07-25; see the landmine header on `TBD_MissionShapeStruct`), so `if (doc.flow)` is
	//! ALWAYS TRUE. `golden-missions/empty-warning-fields.json` authors a literal `"flow": {}` and
	//! must behave IDENTICALLY to a mission with no flow key at all — and both must leave today's
	//! behaviour exactly as it was. `TBD_MissionFlowStruct.ABSENT` is what makes that possible, and
	//! it is also what keeps an authored `0` (which for `timeLimitSeconds` means "no limit" — a
	//! statement, not silence) distinguishable from "the author said nothing".
	//!
	//! Every field is reported on its own line whether it was authored or not, because the failure
	//! this slice exists to fix was INVISIBLE: the hardcoded 300 s safestart happened to match
	//! bridgehead's authored 300 by coincidence, so no log anywhere would have told an operator that
	//! montfort's authored 180 was being ignored.
	//! @authority server
	protected void ApplyMissionFlow()
	{
		string source;

		int briefing = TBD_MissionFlow.ResolveSeconds(TBD_MissionFlow.RawBriefingSeconds(), source);
		ReportSeconds("briefingSeconds", briefing, source, "no briefing timer exists in this build");

		ApplySafeStartSeconds();

		int limit = TBD_MissionFlow.ResolveSeconds(TBD_MissionFlow.RawTimeLimitSeconds(), source);
		ReportSeconds("timeLimitSeconds", limit, source, "no time limit");

		ReportJip();
	}

	//------------------------------------------------------------------------------------------------
	//! One `[TBD][Flow]` line per duration field, naming the SOURCE of the value in force.
	//!
	//! The source label is the entire point of this function. A number on its own cannot tell an
	//! operator whether the mission was honoured or ignored — and for safestart the two produced the
	//! identical number on the one mission anybody had tested with.
	//! @authority server
	protected void ReportSeconds(string field, int value, string source, string defaultLabel)
	{
		if (source == TBD_MissionFlow.SRC_INVALID)
		{
			TBD_Log.Error(TBD_MissionFlow.CH_FLOW, string.Format(
				"flow.%1 is NEGATIVE in the mission document (schema declares minimum 0) — ignored, default in force: %2",
				field, defaultLabel));
			return;
		}

		if (source == TBD_MissionFlow.SRC_DEFAULT)
		{
			TBD_Log.Event(TBD_MissionFlow.CH_FLOW, string.Format(
				"flow.%1=<absent> (default: %2)", field, defaultLabel));
			return;
		}

		TBD_Log.Event(TBD_MissionFlow.CH_FLOW, string.Format("flow.%1=%2 (authored)", field, value));
	}

	//------------------------------------------------------------------------------------------------
	//! Hand `flow.safeStartSeconds` to TBD_SafestartManager.
	//!
	//! ── Through the existing seam, not by reaching into that file ───────────────────────────────
	//! `AdminSetSeconds` is the same entry point `#tbd safestart <seconds>` already drives, so an
	//! authored length and an admin-typed length now travel an identical path and get an identical
	//! bounds check (MIN_COUNTDOWN_SECONDS..MAX_COUNTDOWN_SECONDS). That file belongs to another
	//! slice; using its published seam is what makes this a change of ZERO lines there.
	//!
	//! ── Why the manager's own words are logged rather than ours ─────────────────────────────────
	//! "We called the setter" is not evidence that the setter took — the same reasoning
	//! `TBD_SafestartManager.RestoreOne` applies when it READS BACK `IsDamageHandlingEnabled()`. So
	//! the reply string is logged verbatim and `StatusLine()` is then read back, which quotes
	//! `m_iConfiguredSeconds` through `FormatClock`. The `next arm = M:SS` in that line is the
	//! manager reporting what it is actually holding.
	//!
	//! ── An authored value the manager REFUSES ───────────────────────────────────────────────────
	//! `AdminSetSeconds` rejects anything outside 5..3600, which includes an authored `0`. That is
	//! reported at ERROR with the manager's own reason attached, and the previous value stays in
	//! force. It is NOT clamped: silently running a 300 s safestart when the author asked for 0 is
	//! precisely the failure this slice exists to end, and quietly substituting 5 for 0 would be the
	//! same crime with a smaller number.
	//! @authority server
	protected void ApplySafeStartSeconds()
	{
		string source;
		int seconds = TBD_MissionFlow.ResolveSeconds(TBD_MissionFlow.RawSafeStartSeconds(), source);

		TBD_SafestartManager safestart = TBD_SafestartManager.GetInstance();
		if (!safestart)
		{
			// Not an error: SetStage already REFUSES SAFE_START outright on a world with no
			// safestart component, so there is no countdown for this value to be the length of.
			TBD_Log.Warn(TBD_MissionFlow.CH_FLOW,
				"flow.safeStartSeconds not applied — TBD_SafestartManager is not on this game mode (SAFE_START is refused on this world anyway).");
			return;
		}

		string defaultLabel = string.Format("%1s (TBD_SafestartManager.DEFAULT_COUNTDOWN_SECONDS)",
			TBD_SafestartManager.DEFAULT_COUNTDOWN_SECONDS);

		if (source != TBD_MissionFlow.SRC_AUTHORED)
		{
			ReportSeconds("safeStartSeconds", seconds, source, defaultLabel);
			TBD_Log.Kv(TBD_MissionFlow.CH_FLOW, "safestart", safestart.StatusLine());
			return;
		}

		bool applied = false;
		string reply = safestart.AdminSetSeconds(seconds, applied);

		if (!applied)
		{
			// Built in appended steps, not one long format chain: that is the measured
			// "Formula too complex" landmine, and its misleading second diagnostic.
			string refused = string.Format("flow.safeStartSeconds=%1 (authored) REFUSED by TBD_SafestartManager", seconds);
			refused += " — " + reply;
			refused += " The authored length is NOT in force.";
			TBD_Log.Error(TBD_MissionFlow.CH_FLOW, refused);
			TBD_Log.Kv(TBD_MissionFlow.CH_FLOW, "safestart", safestart.StatusLine());
			return;
		}

		TBD_Log.Event(TBD_MissionFlow.CH_FLOW, string.Format("flow.safeStartSeconds=%1 (authored)", seconds));
		// Read-back, not a claim: this line is TBD_SafestartManager quoting its own
		// m_iConfiguredSeconds. `next arm = 3:00` is 180 authored seconds, in force.
		TBD_Log.Kv(TBD_MissionFlow.CH_FLOW, "safestart", safestart.StatusLine());
	}

	//------------------------------------------------------------------------------------------------
	//! Report the resolved JIP policy and the stages it permits a join in.
	//!
	//! The permitted-stage list is derived from `AllowsJoinAtStage` rather than written out a second
	//! time, so this line cannot describe a rule the code does not implement.
	//! @authority server
	protected void ReportJip()
	{
		string raw = TBD_MissionFlow.RawJip();
		string permitted = TBD_MissionFlow.JoinsPermittedLabel();

		if (raw.IsEmpty())
		{
			TBD_Log.Event(TBD_MissionFlow.CH_FLOW, string.Format(
				"flow.jip=<absent> (default: %1) joins-permitted=%2",
				TBD_MissionFlow.JipPolicyName(), permitted));
			return;
		}

		if (!TBD_MissionFlow.IsKnownPolicyString(raw))
		{
			TBD_Log.Error(TBD_MissionFlow.CH_FLOW, string.Format(
				"flow.jip='%1' is not a value this build understands — falling back to '%2'. joins-permitted=%3",
				raw, TBD_MissionFlow.JipPolicyName(), permitted));
			return;
		}

		TBD_Log.Event(TBD_MissionFlow.CH_FLOW, string.Format(
			"flow.jip=%1 (authored) joins-permitted=%2", raw, permitted));
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server — mutates the replicated m_Stage and calls Replication.BumpMe() to push it.
	void SetStage(TBD_EGameStage stage)
	{
		if (m_Stage == stage)
			return;

		// T-181.17 — SAFE_START is a PROMISE that nobody can be hurt. If the component that keeps
		// that promise is not on this world's game mode, entering the stage would announce a
		// safestart that does not exist — and under ONE LIFE the first negligent discharge ends
		// somebody's event. Refuse the transition and say why, loudly. The admin can still take
		// the round straight to LIVE (`#tbd stage LIVE`), which is honest about being unprotected.
		if (stage == TBD_EGameStage.SAFE_START && !TBD_SafestartManager.GetInstance())
		{
			m_sLastStageRefusal = "SAFE_START has no enforcement on this world (TBD_SafestartManager is missing from the game mode prefab) — go straight to LIVE with '#tbd stage LIVE', and warn players that weapons are hot.";
			TBD_Log.Banner(TBD_Log.CH_SAFESTART,
				"SAFE_START REFUSED — TBD_SafestartManager is not on the game mode; nothing would enforce damage-off",
				true);
			return;
		}

		// T-181.32 — ONE LIFE cannot be enforced while this host hands out non-durable player
		// keys, so SAFE_START/LIVE are refused until it can (or an admin signs the waiver).
		m_sLastStageRefusal = TBD_SpawnManager.StageRefusalFor(stage);
		if (!m_sLastStageRefusal.IsEmpty())
			return;

		m_sLastStageRefusal = string.Empty;
		TBD_EGameStage previous = m_Stage;
		m_Stage = stage;
		Replication.BumpMe();

		// T-181.14 left this hook for whoever owned this file; T-181.17 owns it now. Logged
		// BEFORE the subsystem fan-out so the transition line precedes whatever the subsystems
		// say about it. The legacy `[TBD] Stage` line below is kept verbatim — README.md and
		// STAGING-SERVER.md quote it.
		TBD_Log.Stage(previous, stage);

		TBD_RadioBridgeStub.OnStageChanged(stage);

		TBD_SpawnManager sm = TBD_SpawnManager.GetInstance();
		if (sm)
			sm.OnStageChanged(stage);

		// T-181.17 — EVERY transition, not just the interesting ones: SAFE_START arms the
		// safestart and anything else lifts it, so an admin jumping SAFE_START -> END cannot
		// strand the server with damage off. See TBD_SafestartManager.OnStageChanged.
		TBD_SafestartManager safestart = TBD_SafestartManager.GetInstance();
		if (safestart)
			safestart.OnStageChanged(stage);

		Print("[TBD] Stage → " + typename.EnumToString(TBD_EGameStage, stage));

		// Authority path for the local UI. onRplName does NOT fire on authority, so a listen host
		// needs this explicit call; a dedicated server no-ops inside it. See NotifyLocalStageUI().
		NotifyLocalStageUI();

		if (stage == TBD_EGameStage.LOBBY)
			OnEnterLobby();
		else if (stage == TBD_EGameStage.BRIEFING)
			OnEnterBriefing();
		else if (stage == TBD_EGameStage.LIVE)
			OnEnterLive();
	}

	//------------------------------------------------------------------------------------------------
	protected void OnEnterLobby()
	{
		// Preload the available-mission list so admins can browse/switch immediately.
		TBD_MissionListLoader.Refresh();
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.38 — `flow.briefingSeconds`, made VISIBLE to the people who act on it.
	//!
	//! ══ WHY THIS DOES NOT AUTO-ADVANCE THE STAGE ═══════════════════════════════════════════════
	//! Honestly: BRIEFING HAS NO TIMER IN THIS BUILD, and this slice does not invent one. Nothing
	//! anywhere advances BRIEFING today — an admin does, with `#tbd stage next`. Turning an authored
	//! duration into an automatic transition would be three separate behaviour changes nobody asked
	//! for:
	//!   1. it takes round pacing away from the admin running the event;
	//!   2. the stage it would advance INTO is SAFE_START, which `SetStage` can legitimately REFUSE
	//!      (T-181.17: no safestart component on the world; T-181.32: the host cannot carry ONE
	//!      LIFE) — an automatic advance would then either strand the round or, worse, look like it
	//!      worked;
	//!   3. a briefing is over when the side has finished planning, which is not a number.
	//! So the authored length is ANNOUNCED — it now reaches the admin and the players instead of
	//! being discarded — and the stage machine is left exactly as it was.
	//!
	//! The seam to change that decision is one `CallLater` in this method; it is written up in the
	//! slice report rather than left half-built here.
	//! @authority server
	protected void OnEnterBriefing()
	{
		if (RplSession.Mode() == RplMode.Client)
			return;

		int seconds = TBD_MissionFlow.BriefingSeconds();
		if (seconds == TBD_MissionFlow.UNSET)
			return;

		if (seconds == 0)
		{
			TBD_Log.Kv(TBD_MissionFlow.CH_FLOW, "briefing",
				"authoredSeconds=0 — the mission asks for no briefing pause; advance when ready");
			return;
		}

		string clock = TBD_SafestartManager.FormatClock(seconds);
		TBD_Log.Kv(TBD_MissionFlow.CH_FLOW, "briefing", string.Format(
			"authoredSeconds=%1 (%2) — ADVISORY, no auto-advance; an admin advances with '#tbd stage next'",
			seconds, clock));

		string msg = "[TBD] BRIEFING — the mission allows ";
		msg += clock;
		msg += " for orders. Read your side's brief now.";
		Broadcast(msg);
	}

	//------------------------------------------------------------------------------------------------
	//! Everything that starts when the round starts.
	//!
	//! T-181.38 — the end conditions are armed INDEPENDENTLY. Before this slice `OnEnterLive`
	//! returned early when the mission did not declare `faction_eliminated`, so a mission declaring
	//! only `time_limit` (`golden-missions/empty-warning-fields.json` declares exactly `time_limit`
	//! + `all_objectives_captured`) entered LIVE with nothing at all watching it, and could never
	//! end except by an admin.
	//! @authority server
	protected void OnEnterLive()
	{
		if (RplSession.Mode() == RplMode.Client)
			return;

		ArmFactionEliminated();
		ArmRoundClock();
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.13 — start watching for a side to be wiped out. Only armed if the mission actually
	//! declared `faction_eliminated`; a mission that declared nothing runs until an admin ends
	//! it rather than ending on its own.
	//! @authority server
	protected void ArmFactionEliminated()
	{
		// T-181.39 — arm on ANY supported trigger. The old guard checked only
		// faction_eliminated, so a mission declaring just `all_objectives_captured` never armed
		// the win tick at all and ran silently to the time limit.
		bool anyTrigger = TBD_MissionLoader.HasEndTrigger("faction_eliminated");
		if (TBD_MissionLoader.HasEndTrigger(TBD_ObjectiveRegistry.TRIGGER_ALL_CAPTURED))
			anyTrigger = true;
		if (TBD_MissionLoader.HasEndTrigger(TBD_ObjectiveRegistry.TRIGGER_DESTROYED))
			anyTrigger = true;
		if (TBD_MissionLoader.HasEndTrigger(TBD_ObjectiveRegistry.TRIGGER_HOLD_EXPIRED))
			anyTrigger = true;

		if (!anyTrigger)
		{
			Print("[TBD][Win] no faction_eliminated trigger in mission — round runs until admin ends it");
			return;
		}

		// 2 s cadence: an elimination is not time-critical, and this walks every claimed slot.
		GetGame().GetCallqueue().Remove(TickWinConditions);
		GetGame().GetCallqueue().CallLater(TickWinConditions, 2000, true);
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.38 — start the authored round clock. Armed at LIVE, NOT at LOADING: the clock measures
	//! the ROUND, and the lobby, brief and warmup that precede it are not the round.
	//!
	//! Because it hangs off the LIVE transition it is also automatically subject to every guard
	//! `SetStage` applies — including T-181.32's one-life identity gate. A round refused entry to
	//! LIVE never starts its clock, which is correct.
	//!
	//! ══ BOTH HALVES ARE REQUIRED, AND EITHER ONE ALONE IS REPORTED ═════════════════════════════
	//! `winConditions.endOn` is the mission's own statement of HOW the round may end — the exact
	//! list `faction_eliminated` is already gated on — and `flow.timeLimitSeconds` is HOW LONG. The
	//! clock arms only when the mission said both, because:
	//!   * a duration with no `time_limit` trigger would end a round on a condition the author never
	//!     declared;
	//!   * a `time_limit` trigger with no duration cannot end anything, and a round that quietly
	//!     runs forever is the exact failure this slice exists to fix.
	//! Neither half alone is guessed at, and neither is silent: both mismatches are WARNINGs naming
	//! the mission's own fields. All four goldens author both halves consistently, so this is a
	//! guard against a future producer bug rather than a live workaround.
	//!
	//! An authored `0` is NOT "absent" — the schema's `minimum: 0` makes it a legal, deliberate
	//! statement that this mission has no time limit, and it is logged as exactly that.
	//! @authority server
	protected void ArmRoundClock()
	{
		m_iRoundSecondsRemaining = ROUND_CLOCK_OFF;
		GetGame().GetCallqueue().Remove(TickRoundClock);

		int limit = TBD_MissionFlow.TimeLimitSeconds();
		bool declared = TBD_MissionLoader.HasEndTrigger("time_limit");

		if (!declared)
		{
			if (limit > 0)
			{
				TBD_Log.Warn(TBD_MissionFlow.CH_FLOW, string.Format(
					"flow.timeLimitSeconds=%1 is authored but winConditions.endOn does not declare 'time_limit' — clock NOT armed, this round will not end on time.",
					limit));
			}
			return;
		}

		if (limit == TBD_MissionFlow.UNSET)
		{
			TBD_Log.Warn(TBD_MissionFlow.CH_FLOW,
				"winConditions.endOn declares 'time_limit' but flow.timeLimitSeconds is not authored — this round CANNOT end on time.");
			return;
		}

		if (limit == 0)
		{
			TBD_Log.Event(TBD_MissionFlow.CH_FLOW,
				"flow.timeLimitSeconds=0 (authored) — an explicit NO LIMIT; the round will not end on time.");
			return;
		}

		m_iRoundSecondsRemaining = limit;
		GetGame().GetCallqueue().CallLater(TickRoundClock, 1000, true);

		string clock = TBD_SafestartManager.FormatClock(limit);
		TBD_Log.Kv(TBD_MissionFlow.CH_FLOW, "time_limit",
			string.Format("armed seconds=%1 (%2) at stage=LIVE", limit, clock));

		string msg = "[TBD] ROUND TIME LIMIT: ";
		msg += clock;
		msg += ". The round ends when it expires.";
		Broadcast(msg);
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.38 — the round clock. 1 Hz, mirroring `TBD_SafestartManager.TickCountdown`, which is
	//! this framework's established cadence for a player-visible countdown (ENF-1: not a per-frame
	//! path — 5400 ticks across the whole of bridgehead's 90-minute round).
	//!
	//! It disarms itself the instant the round is no longer LIVE, so an admin ending the round early
	//! or restarting it to LOBBY cannot leave a clock running that later ends a round which already
	//! ended. `ScriptCallQueue.Remove` cancels BY FUNCTION, which is exactly right here: there is
	//! one round clock per world.
	//! @authority server
	protected void TickRoundClock()
	{
		if (m_Stage != TBD_EGameStage.LIVE)
		{
			GetGame().GetCallqueue().Remove(TickRoundClock);
			m_iRoundSecondsRemaining = ROUND_CLOCK_OFF;
			return;
		}

		m_iRoundSecondsRemaining--;

		if (m_iRoundSecondsRemaining > 0)
		{
			if (IsRoundClockMilestone(m_iRoundSecondsRemaining))
			{
				string warn = "[TBD] ROUND TIME REMAINING: ";
				warn += TBD_SafestartManager.FormatClock(m_iRoundSecondsRemaining);
				warn += ".";
				Broadcast(warn);
			}
			return;
		}

		GetGame().GetCallqueue().Remove(TickRoundClock);
		m_iRoundSecondsRemaining = ROUND_CLOCK_OFF;

		// The same `[TBD][Win]` prefix the elimination path uses, so one grep finds every way a
		// round has ever ended.
		Print("[TBD][Win] time_limit — authored round clock expired");
		Broadcast("[TBD] TIME. The round is over.");

		// THE SAME END PATH `faction_eliminated` uses. This slice adds a new REASON for a round to
		// end, never a second way of ending one: every guard, log line and subsystem fan-out inside
		// SetStage applies identically.
		SetStage(TBD_EGameStage.END);

		// SetStage can legitimately REFUSE a transition (T-181.17's safestart guard, T-181.32's
		// one-life identity gate). Neither gates END today, but the clock has already disarmed
		// itself by this point, so a refused END must not be left looking like a completed round.
		if (m_Stage != TBD_EGameStage.END)
		{
			TBD_Log.Error(TBD_MissionFlow.CH_FLOW,
				"time limit expired but the END transition was REFUSED — the round is still running: " + m_sLastStageRefusal);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.38 — when players are told how long is left. Sparse: chat is the durable channel and so
	//! also the one that becomes noise fastest (same reasoning, and the same ladder from ten minutes
	//! down, as `TBD_SafestartManager.IsChatMilestone`).
	static bool IsRoundClockMilestone(int seconds)
	{
		if (seconds == 1800)
			return true;
		if (seconds == 900)
			return true;
		if (seconds == 600)
			return true;
		if (seconds == 300)
			return true;
		if (seconds == 120)
			return true;
		if (seconds == 60)
			return true;
		if (seconds == 30)
			return true;
		if (seconds == 10)
			return true;
		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! Server -> every connected player's chat feed.
	//!
	//! Duplicated from `TBD_SafestartManager.Broadcast` rather than shared, deliberately: that
	//! method is `protected` on a component this slice does not own, and under ONE LIFE a round
	//! clock that ends somebody's event without ever having warned them is not acceptable. Chat is
	//! also the only player-facing channel that works TODAY — every TBD screen is blocked behind the
	//! `resourceDatabase.rdb` regeneration, and a chat line needs no menu preset. If a third caller
	//! ever appears, lift this into a shared helper; two is not yet a pattern.
	//! @authority server
	protected void Broadcast(string text)
	{
		// Authority only — the server is the only machine that should be telling everyone anything.
		if (RplSession.Mode() == RplMode.Client)
			return;

		Print("[TBD][Flow] broadcast: " + text, LogLevel.NORMAL);

		PlayerManager players = GetGame().GetPlayerManager();
		if (!players)
			return;

		array<int> ids = {};
		int count = players.GetPlayers(ids);
		for (int i = 0; i < count; i++)
		{
			PlayerController controller = players.GetPlayerController(ids[i]);
			if (!controller)
				continue;

			SCR_ChatComponent chat = SCR_ChatComponent.Cast(controller.FindComponent(SCR_ChatComponent));
			if (!chat)
				continue;

			chat.SendPrivateMessage(text, ids[i]);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Ends the round when a playable faction has no living claimed slots left.
	//!
	//! Guards that matter under ONE LIFE: it only fires while LIVE, it ignores factions that
	//! never had anyone claim in (0 claimed != eliminated, otherwise an unplayed side would end
	//! the round at kickoff), and it needs at least two factions with players so a solo test
	//! session does not instantly end.
	//! @authority server
	protected void TickWinConditions()
	{
		if (m_Stage != TBD_EGameStage.LIVE)
		{
			GetGame().GetCallqueue().Remove(TickWinConditions);
			return;
		}

		// T-181.39 — objective-driven end triggers, evaluated BEFORE the attrition check so a
		// mission that wins on objectives does not have to also eliminate a faction. The registry
		// gates each branch on HasEndTrigger itself, and returns empty when it never built (its
		// component absent from the prefab), so this degrades to a no-op rather than a crash.
		string objectiveWinner;
		string objectiveTrigger = TBD_ObjectiveRegistry.EvaluateEndTriggers(objectiveWinner);
		if (!objectiveTrigger.IsEmpty())
		{
			GetGame().GetCallqueue().Remove(TickWinConditions);
			PrintFormat("[TBD][Win] %1 — winner=%2", objectiveTrigger, objectiveWinner);
			SetStage(TBD_EGameStage.END);
			return;
		}

		TBD_SpawnManager sm = TBD_SpawnManager.GetInstance();
		array<ref TBD_MissionFactionStruct> factions = TBD_MissionLoader.GetFactions();
		if (!sm || !factions)
			return;

		int contesting;      // factions that had at least one claimed slot
		int stillAlive;      // ...of those, how many still have a living player
		string lastAlive;

		foreach (TBD_MissionFactionStruct f : factions)
		{
			if (!f || f.key.IsEmpty())
				continue;

			int claimed = sm.CountClaimedForFaction(f.key);
			if (claimed == 0)
				continue;      // never fielded — cannot be "eliminated"

			contesting++;
			if (sm.CountAliveForFaction(f.key) > 0)
			{
				stillAlive++;
				lastAlive = f.key;
			}
		}

		if (contesting < 2)
			return;            // need a real contest before anyone can win

		if (stillAlive > 1)
			return;

		GetGame().GetCallqueue().Remove(TickWinConditions);
		Print(string.Format("[TBD][Win] faction_eliminated — winner=%1 (%2 factions contested)",
			lastAlive, contesting));
		SetStage(TBD_EGameStage.END);
	}

	//------------------------------------------------------------------------------------------------
	//! Current mission's terrain key (empty if no mission loaded).
	protected string GetCurrentTerrain()
	{
		TBD_MissionDocumentStruct m = TBD_MissionLoader.GetMission();
		if (!m || !m.meta)
			return string.Empty;
		return m.meta.terrain;
	}

	//------------------------------------------------------------------------------------------------
	//! Admin: numbered mission list as display lines.
	array<string> BuildMissionListText()
	{
		array<string> lines = new array<string>();
		array<ref TBD_MissionListEntry> entries = TBD_MissionListLoader.GetEntries();
		if (!entries || entries.IsEmpty())
		{
			lines.Insert("TBD: no missions loaded yet — try '#tbd refresh' in a moment.");
			return lines;
		}

		lines.Insert(string.Format("TBD missions (%1) — current terrain: %2", entries.Count(), GetCurrentTerrain()));
		for (int i = 0; i < entries.Count(); i++)
		{
			TBD_MissionListEntry e = entries[i];
			lines.Insert(string.Format("  %1) %2 [%3] %4 slots", i + 1, e.name, e.terrain, e.slotCount));
		}
		return lines;
	}

	//------------------------------------------------------------------------------------------------
	//! Admin: refresh the mission list from the backend.
	void RefreshMissionList()
	{
		TBD_MissionListLoader.Refresh();
	}

	//------------------------------------------------------------------------------------------------
	//! Admin: select a mission by 1-based number — persist it and reload the world.
	string SelectMissionByNumber(int number)
	{
		TBD_MissionListEntry e = TBD_MissionListLoader.GetEntryByNumber(number);
		if (!e)
			return string.Format("TBD: no mission #%1.", number);

		if (e.slotCount <= 0)
			Print(string.Format("[TBD] Selected mission %1 has 0 slots — players will have no spawn.", e.id), LogLevel.WARNING);

		if (!TBD_BackendConfig.SetMissionId(e.id))
			return "TBD: failed to persist mission selection.";

		string target = e.terrain;
		string current = GetCurrentTerrain();

		if (target.IsEmpty() || target == current)
		{
			Print(string.Format("[TBD] Admin selected %1 (%2) — same terrain, restarting scenario.", e.id, target));
			GameStateTransitions.RequestScenarioRestart();
			return string.Format("TBD: loading %1…", e.name);
		}

		string scenario = TBD_ScenarioRouter.GetScenarioForTerrain(target);
		if (scenario.IsEmpty())
			return string.Format("TBD: no scenario for terrain '%1' yet (mission stays selected for next %1 load).", target);

		Print(string.Format("[TBD] Admin selected %1 (%2) — switching scenario to %3.", e.id, target, scenario));
		GameStateTransitions.RequestScenarioChangeTransition(scenario, string.Empty, TBD_ScenarioRouter.GetAddonList());
		return string.Format("TBD: switching to %1 on %2…", e.name, target);
	}

	//------------------------------------------------------------------------------------------------
	//! Admin: repoint the backend URL (and optionally token), then refresh the list.
	string SetBackend(string url, string token)
	{
		if (url.IsEmpty())
			return "Usage: #tbd backend <url> [token]";
		if (!TBD_BackendConfig.SetBackend(url, token))
			return "TBD: failed to set backend.";
		TBD_MissionListLoader.Refresh();
		return string.Format("TBD: backend set to %1 — refreshing list…", url);
	}

	//------------------------------------------------------------------------------------------------
	//! @authority client — onRpl hook for m_Stage (RplProp onRplName); runs on clients on replication.
	void OnStageReplicated()
	{
		NotifyLocalStageUI();
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.23 — hand the current stage to THIS machine's local player UI, if it has one.
	//!
	//! Called from two places on purpose, because either one alone is wrong:
	//!   • `OnStageReplicated()` — the PROXY path. `[RplProp(onRplName:)]` fires only on the proxy
	//!     (TBD_MOD_DESIGN.md §5), so this is how a dedicated-server client hears about a stage
	//!     change at all.
	//!   • `SetStage()` — the AUTHORITY path. On a listen host the authority IS the player, and
	//!     authority never receives its own onRplName callback. Without this call the host's
	//!     briefing screen would never open — which is exactly the regression the 500 ms poll this
	//!     replaces was papering over, since a poll reads `GetStage()` on both topologies.
	//!
	//! A DEDICATED server no-ops here: it has no workspace and no local player controller, so both
	//! guards below fail and nothing happens. The server-side stage machine is untouched — this
	//! method only ever drives local UI, and never feeds back into replication.
	protected void NotifyLocalStageUI()
	{
		// No workspace = dedicated server. It must never try to drive a menu.
		if (!GetGame().GetWorkspace())
			return;

		SCR_PlayerController pc = SCR_PlayerController.Cast(GetGame().GetPlayerController());
		if (!pc)
			return;

		// Idempotent on the receiving side: TBD_OnStageChanged acts on TRANSITIONS only, so a
		// redundant replication callback cannot re-open the briefing or wipe a received payload.
		pc.TBD_OnStageChanged(m_Stage);
	}

	//------------------------------------------------------------------------------------------------
	//! Admin chat command entry — `#stage next` / `#stage LOBBY` etc.
	void HandleAdminStageCommand(string args)
	{
		if (args.IsEmpty())
			return;

		if (args == "next")
		{
			int next = m_Stage + 1;
			if (next > TBD_EGameStage.DEBRIEF)
				return;
			SetStage(next);
			return;
		}

		// Named stage: LOBBY, LIVE, …
		for (int i = TBD_EGameStage.LOADING; i <= TBD_EGameStage.DEBRIEF; i++)
		{
			string name = typename.EnumToString(TBD_EGameStage, i);
			if (args == name)
			{
				SetStage(i);
				return;
			}
		}
	}
}
