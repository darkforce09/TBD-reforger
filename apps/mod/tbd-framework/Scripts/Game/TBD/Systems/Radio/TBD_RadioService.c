//! T-181.40 — the SERVER half of the radio plan: which nets a player is on, and in what shape
//! they cross the wire.
//!
//! ── Why the server has to do this at all ────────────────────────────────────────────────────
//! Clients hold NO mission document. `TBD_FrameworkManager.OnPostInit` returns early for
//! `RplMode.Client` before `BeginLoad()`, so a client physically cannot read `radioPlan.nets[]`
//! for itself. Nets are server-fed or they do not exist.
//!
//! ── Side discipline, enforced at the WIRE ───────────────────────────────────────────────────
//! `net.faction` scopes a net to one side. Frequencies ARE intelligence: knowing OPFOR is on
//! 51.000 is knowing where to listen, and on a game whose radio the player can hand-tune, it is
//! knowing where to listen with the radio they are already carrying. `bridgehead-at-levie` authors
//! `net:cmd` on 41.0 for blufor and `net:cmd_op` on 51.0 for opfor; sending both to everyone and
//! filtering in a widget would hand each side the other's command net.
//!
//! Three properties make that structural rather than a promise, copied deliberately from
//! `TBD_MarkerData.c` (T-181.19), which is the model in this codebase:
//!   1. `BuildForPlayer` takes a **playerId and nothing else**. There is no faction parameter, so
//!      a client has nowhere to put a lie.
//!   2. The side is read from `TBD_SpawnManager.GetAssignedSlot(playerId)` — server-owned state a
//!      client cannot influence (`m_mPlayerSlot` is a plain map, not an `RplProp`).
//!   3. Only the resolved side's nets are ever placed in the arrays that get sent
//!      (`TBD_RadioPlan.GetNetsForFaction` BUILDS the answer rather than filtering a full list),
//!      so the other side's frequencies never leave the server process.
//!
//! **If a client asked for another faction's nets it could not phrase the question.** The request
//! RPC takes no arguments; the answer is whatever `GetAssignedSlot` says the caller is. A player
//! with no slot gets `served = false` and zero nets — fail closed, not fail open.
//!
//! A net with an EMPTY `faction` is deliberately shared with everyone: the schema makes `faction`
//! optional, so an unscoped net is an authoring choice meaning "common channel", not an oversight.
//!
//! ── Why parallel arrays and not a delimited string ──────────────────────────────────────────
//! Same reasoning as markers, and it is not hypothetical here either. `string.Split`'s empty-token
//! behaviour is a RUNTIME property no probe on this lane can settle, and a net `label` is authored
//! free text that may legally contain any delimiter we picked. So there is no delimiter: four
//! parallel `array<...>` RPC parameters carry the fields positionally, element i of each being
//! field i of net i. Both array types used here are `array<int>` / `array<string>` — the only two
//! that appear as replicated-method parameters in EITHER oracle, and the shape already proven and
//! shipped by `TBD_MarkerController.TBD_RpcDo_Markers`. The long-range flag is therefore an int
//! 0/1 rather than the `array<bool>` that would read more naturally.
//!
//! Frequencies cross the wire as INTEGER kHz, not as the schema's float MHz: kHz is the unit the
//! engine's own radio API speaks, integers have no formatting ambiguity, and the client formats
//! the display text from the integer so what a player reads is what a transceiver was set to.
class TBD_RadioService
{
	//! Hard cap on nets sent to one client. `TBD_RadioPlan` already caps the document at 32; this
	//! is the wire's own limit so a change there cannot silently widen a reliable RPC.
	static const int MAX_NETS_ON_WIRE = 32;

	//! playerId -> last outcome logged for them, so a polling client cannot fill the console.
	protected static ref map<int, string> s_mLastLogged;

	//! Hard ceiling on that map. Player ids are RECYCLED on a dedicated server and this file has no
	//! disconnect hook, so the whole table is dropped rather than leaked across a long session. The
	//! only cost of dropping it is one repeated log line per player.
	protected static const int MAX_LOG_STATES = 256;

	//------------------------------------------------------------------------------------------------
	//! @authority server — build ONE player's net list, and try to tune their radio into it.
	//!
	//! Never returns null: an unslotted player, an unloaded mission and a mission with no nets for
	//! their side are three different legal states, and each yields an empty served=false answer
	//! carrying the reason.
	//!
	//! The tune attempt is deliberately part of the SAME call that builds the wire, so the outcome
	//! the player is shown and the outcome the log records are the same measurement. There is no
	//! path in this file that reports a tune it did not verify — `TBD_RadioTuner` reads the
	//! frequency back off the transceiver and the result rides the wire as `m_sTuneResult`.
	static TBD_RadioWire BuildForPlayer(int playerId)
	{
		TBD_RadioWire wire = Build(playerId);

		// Logged HERE, not at the RPC handler, because a listen host short-circuits the RPC
		// entirely and would otherwise be the one topology that produced no radio log at all.
		// Host/dedicated behaviour divergence is a recorded failure mode in this program.
		LogOutcome(playerId, wire);

		return wire;
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server — the decision itself, with no logging in it.
	protected static TBD_RadioWire Build(int playerId)
	{
		TBD_RadioWire wire = new TBD_RadioWire();

		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		if (!spawn)
		{
			wire.m_sRefusal = "no spawn manager";
			return wire;
		}

		TBD_MissionSlotStruct slot = spawn.GetAssignedSlot(playerId);
		if (!slot)
		{
			// Fail closed. No seat means no side, and no side means no side's frequencies.
			wire.m_sRefusal = "no slot assigned";
			return wire;
		}

		TBD_MissionDocumentStruct doc = TBD_MissionLoader.GetMission();
		if (!doc || !doc.meta)
		{
			wire.m_sRefusal = "no mission loaded";
			return wire;
		}

		wire.m_sFactionKey = slot.faction;
		wire.m_sMissionId = doc.meta.id;

		// From here on the answer is authoritative even when it is empty: the player HAS a side and
		// the server HAS a mission, so "this side authored no nets" is a real, served answer and
		// the client should stop asking.
		wire.m_bServed = true;

		array<TBD_MissionNetStruct> nets = TBD_RadioPlan.GetNetsForFaction(slot.faction);

		foreach (TBD_MissionNetStruct net : nets)
		{
			if (wire.Count() >= MAX_NETS_ON_WIRE)
				break;

			int khz = TBD_RadioPlan.FreqKHz(net.freqMHz);

			wire.m_aId.Insert(net.id);
			wire.m_aLabel.Insert(net.label);
			wire.m_aFreqKHz.Insert(khz);
			wire.m_aLongRange.Insert(LongRangeFlag(net.range));
		}

		TBD_RadioTuneReport report = TBD_RadioTuner.TunePlayer(playerId, wire.m_aFreqKHz, wire.m_aLongRange);
		wire.m_sTuneResult = report.ResultName();
		wire.m_iTuned = report.m_iTuned;
		wire.m_sTuneDetail = report.m_sDetail;

		return wire;
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server — the whole connected roster, at a stage boundary.
	//!
	//! Called from `TBD_RadioBridgeStub.OnStageChanged`, which is an EXISTING call site
	//! (`TBD_FrameworkManager.c:250`) — no new hook was added to a file this slice does not own.
	//! SAFE_START and LIVE are the two transitions at which everybody who is going to be in a body
	//! is in one, which makes them the honest moments to push a tune.
	//!
	//! Retuning at the top of LIVE is also what recovers a player who slotted late: the pull path
	//! in `TBD_RadioController` covers them too, and having both means neither is load-bearing
	//! alone. T-181.28 records the briefing shipping push-only and silently missing late joiners;
	//! this slice does not repeat that.
	static void OnStageChanged(TBD_EGameStage stage)
	{
		if (RplSession.Mode() == RplMode.Client)
			return;

		if (stage != TBD_EGameStage.SAFE_START && stage != TBD_EGameStage.LIVE)
			return;

		PlayerManager players = GetGame().GetPlayerManager();
		if (!players)
			return;

		array<int> ids = {};
		int count = players.GetPlayers(ids);

		// Zero players is the ordinary world-boot case and is not worth a line at all.
		if (count == 0)
			return;

		int served = 0;
		int tuned = 0;

		for (int i = 0; i < count; i++)
		{
			TBD_RadioWire wire = BuildForPlayer(ids[i]);
			if (!wire.m_bServed)
				continue;

			served++;
			if (wire.m_iTuned > 0)
				tuned++;

			// Push the SAME wire that was just measured. `BuildForPlayer` performed the tune, so
			// the player's display and their radio are updated from one measurement rather than
			// two — and a client whose poll already stopped still learns that its radio changed.
			SCR_PlayerController controller = SCR_PlayerController.Cast(players.GetPlayerController(ids[i]));
			if (controller)
				controller.TBD_PushRadioNets(wire);
		}

		TBD_Log.Kv(TBD_RadioPlan.CH_RADIO, "sweep", string.Format(
			"stage=%1 players=%2 served=%3 radiosTuned=%4",
			typename.EnumToString(TBD_EGameStage, stage), count, served, tuned));
	}

	//------------------------------------------------------------------------------------------------
	//! Schema `range` -> "this net wants the long-range set", as 0/1.
	//!
	//! An `int` flag and not a `bool`, because this value goes into `m_aLongRange`, which IS an RPC
	//! parameter — and `array<int>` is proven in both oracles while `array<bool>` appears in
	//! neither. See `TBD_RadioTuner.TunePlayer`.
	//!
	//! T-292 — the schema admits exactly two values (`short` | `long`, default `short`), matching
	//! Enfusion's two radio gadget classes. Only `long` returns 1 (backpack preference). `short`
	//! and ABSENT (empty — `JsonLoadContext` leaves a missing string at its initializer; schema
	//! default is `short`) return 0 (handheld preference). The retired value `any` is rejected by
	//! `mission.schema.json` but still maps to 0 here so a pre-T-292 document that somehow skipped
	//! schema validation does not flip into backpack mode by accident.
	//!
	//! Compared case-sensitively against the schema's own lowercase enum, deliberately: `ToLower()`
	//! MUTATES IN PLACE AND RETURNS A COUNT in Enfusion, so the obvious normalising one-liner does
	//! not do what it looks like it does, and an authored value outside the enum is a document that
	//! never passed schema validation.
	protected static int LongRangeFlag(string range)
	{
		if (range == "long")
			return 1;

		// Explicit `short` (and empty / legacy `any`) → handheld. Named so the handheld path is not
		// a silent fall-through that made `short` look discarded next to a three-value schema.
		if (range == "short" || range.IsEmpty() || range == "any")
			return 0;

		return 0;
	}

	//------------------------------------------------------------------------------------------------
	//! One line per player, only when the answer CHANGES.
	//!
	//! Both outcomes are NORMAL level. A refusal is not an error — an unslotted player asking
	//! during LOBBY is the ordinary case and is how the client knows to keep asking — and
	//! `world-boot.sh` triages any TBD-owned `SCRIPT (E)` line as a gate failure.
	//!
	//! The tune result is on the SAME line as the net count on purpose. That pairing is the whole
	//! honesty contract of this slice in one string: `nets=2 tune=NO_BACKBONE` reads as "the player
	//! was told about two nets and no radio was touched", which is exactly the truth today.
	protected static void LogOutcome(int playerId, TBD_RadioWire wire)
	{
		if (!wire.m_bServed)
		{
			if (ShouldLog(playerId, "refused:" + wire.m_sRefusal))
			{
				TBD_Log.Kv(TBD_RadioPlan.CH_RADIO, "refused",
					string.Format("player=%1 reason='%2'", playerId, wire.m_sRefusal));
			}

			return;
		}

		// Built in steps — a long `+` chain trips `Formula too complex`.
		string outcome = "served:";
		outcome = outcome + wire.m_sFactionKey;
		outcome = outcome + ":";
		outcome = outcome + wire.m_sMissionId;
		outcome = outcome + ":";
		outcome = outcome + wire.Count().ToString();
		outcome = outcome + ":";
		outcome = outcome + wire.m_sTuneResult;
		outcome = outcome + ":";
		outcome = outcome + wire.m_iTuned.ToString();

		if (!ShouldLog(playerId, outcome))
			return;

		string detail = string.Empty;
		if (!wire.m_sTuneDetail.IsEmpty())
			detail = " (" + wire.m_sTuneDetail + ")";

		TBD_Log.Kv(TBD_RadioPlan.CH_RADIO, "served", string.Format(
			"player=%1 faction=%2 mission=%3 nets=%4 tune=%5 tuned=%6%7",
			playerId, wire.m_sFactionKey, wire.m_sMissionId, wire.Count(),
			wire.m_sTuneResult, wire.m_iTuned, detail));
	}

	//------------------------------------------------------------------------------------------------
	//! True the FIRST time this player's outcome differs from the last one logged for them.
	//!
	//! An unserved client re-asks every few seconds, and a full server sitting in LOBBY would
	//! otherwise emit a dozen identical "refused" lines a second — a defect already on the books
	//! against the admin service (T-181.30 item 4).
	protected static bool ShouldLog(int playerId, string outcome)
	{
		if (!s_mLastLogged)
			s_mLastLogged = new map<int, string>();

		if (s_mLastLogged.Count() > MAX_LOG_STATES)
			s_mLastLogged.Clear();

		string previous;
		if (s_mLastLogged.Find(playerId, previous) && previous == outcome)
			return false;

		s_mLastLogged.Set(playerId, outcome);
		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! Statics outlive a world inside one process (recorded landmine), so the log-state table and
	//! the parsed plan are both released with the world.
	static void Reset()
	{
		s_mLastLogged = null;
		TBD_RadioPlan.Reset();
	}
}

//! One player's net list, server-side, in the exact shape the RPC takes.
//!
//! Deliberately parallel arrays rather than an array of row objects: these fields ARE the RPC
//! parameter list, and keeping the class isomorphic to the wire means the packing step cannot
//! reorder or drop a column without the compiler noticing.
class TBD_RadioWire
{
	//! False = the server had no authoritative answer for this player yet (no slot, no mission).
	//! The client keeps asking while this is false, and stops the moment it is true — including
	//! when it is true with zero nets, which is a real answer.
	bool m_bServed;

	//! The side these nets belong to. Diagnostic only; the client never sends it back and never
	//! makes a trust decision with it.
	string m_sFactionKey;

	//! Lets the client tell "same nets again" from "the admin switched missions".
	string m_sMissionId;

	ref array<string> m_aId = {};       //!< `net:<id>`, stable channel key.
	ref array<string> m_aLabel = {};    //!< Display name, already length-capped.
	ref array<int> m_aFreqKHz = {};     //!< Kilohertz — the unit the engine's radio API speaks.
	ref array<int> m_aLongRange = {};   //!< 1 when `range: long` (backpack); 0 when `range: short` / absent (handheld).

	//! `TBD_ERadioTuneResult` by NAME, so the client can render the truth without importing the
	//! enum's numeric values across a wire that would then be version-coupled to them.
	string m_sTuneResult;

	//! How many nets were VERIFIABLY placed on a transceiver (read back and compared). Zero with a
	//! non-empty net list is the current, honest, expected state on a world with no radio backbone.
	int m_iTuned;

	//! Human-readable nuance about the tune, when there is any.
	string m_sTuneDetail;

	//! Why the server declined, when it did. Logged, not shown to the player.
	string m_sRefusal;

	//------------------------------------------------------------------------------------------------
	int Count()
	{
		return m_aId.Count();
	}
}
