/**
 * @file TBD_RadioService.c
 * @brief Server half of the radio plan: which nets a player is on, and their tune.
 *
 * Role: builds one player's side-scoped net list, tunes their radio into it, and sweeps the
 * connected roster at SAFE_START and LIVE.  Position: called by the modded `SCR_PlayerController`
 * net request (RPC or in place on a listen host) and by `TBD_RadioBridgeStub.OnStageChanged`;
 * reads `TBD_SpawnManager.GetAssignedSlot`, `TBD_RadioPlan` and `TBD_RadioTuner`.
 * State: the per-player last logged outcome, static, on the server.
 * Invariants: frequencies are side-scoped intelligence. The request takes a player id and nothing
 * else, the side comes from server-owned slot state, and `TBD_RadioPlan.GetNetsForFaction` builds
 * only that side's (and shared) nets, so another side's frequencies never leave the server. A
 * player without a slot gets `served = false` and no nets (fail closed). Nets travel as parallel
 * arrays with integer kHz, the engine's radio unit; the long-range flag is an int because
 * `array<int>` is a proven RPC parameter type. The tune is part of the same call that builds the
 * wire, so the player is shown the measured outcome.
 */

//! Server-side radio net builder and stage sweep.
//! @authority server
class TBD_RadioService
{
	static const int MAX_NETS_ON_WIRE = 32; //!< most nets sent to one client; the wire's own limit
	protected static ref map<int, string> s_mLastLogged; //!< player id -> last logged outcome; null until first use
	protected static const int MAX_LOG_STATES = 256; //!< entries after which `s_mLastLogged` is dropped whole

	//! Build one player's net list, tune their radio into it, and log the outcome when it changed.
	//! Logging here covers the listen-host path, which skips the RPC handler.
	//! @param playerId the requesting player
	//! @return never null: `served = false` with a refusal reason when the player has no slot or
	//! no mission is loaded; otherwise served, with the side's nets (possibly none) and the tune
	//! result `TBD_RadioTuner` read back off the transceiver
	static TBD_RadioWire BuildForPlayer(int playerId)
	{
		TBD_RadioWire wire = Build(playerId);
		LogOutcome(playerId, wire);

		return wire;
	}

	//! The net decision and tune, without logging.
	//! @param playerId the requesting player
	//! @return the wire, never null
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
			// Fail closed: no seat means no side.
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

		// From here on the answer is authoritative even when it has no nets.
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

	//! At SAFE_START and LIVE, build, tune and push every connected player's nets and log one
	//! `sweep` line. With the client's pull, this also recovers a player who slotted late. Does
	//! nothing on a client, at other stages, or with no players.
	//! @param stage the stage entered
	//! @authority server
	static void OnStageChanged(TBD_EGameStage stage)
	{
		if (TBD_Authority.IsClient())
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

			// Push the wire just measured, so a client whose poll stopped learns the new tune.
			SCR_PlayerController controller = SCR_PlayerController.Cast(players.GetPlayerController(ids[i]));
			if (controller)
				controller.TBD_PushRadioNets(wire);
		}

		TBD_Log.Kv(TBD_RadioPlan.CH_RADIO, "sweep", string.Format(
			"stage=%1 players=%2 served=%3 radiosTuned=%4",
			typename.EnumToString(TBD_EGameStage, stage), count, served, tuned));
	}

	//! Map a net's `range` to the long-range flag, compared case-sensitively with the schema's
	//! lowercase enum.
	//! @param range `long`, `short`, or empty when absent
	//! @return 1 for `long` (backpack preference); 0 for anything else (handheld preference)
	protected static int LongRangeFlag(string range)
	{
		if (range == "long")
			return 1;

		// `short`, empty and the schema-rejected `any` name the handheld path explicitly.
		if (range == "short" || range.IsEmpty() || range == "any")
			return 0;

		return 0;
	}

	//! Log one line per player, only when the player's outcome changed, at NORMAL level. A served
	//! line pairs the net count with the tune result (for example `nets=2 tune=NO_BACKBONE`).
	//! @param playerId the requesting player
	//! @param wire the built answer
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

		// Built in steps -- a long `+` chain trips `Formula too complex`.
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

	//! Record the player's outcome; an unserved client re-asks every few seconds, so identical
	//! outcomes must not log again. The table is dropped whole past `MAX_LOG_STATES` entries.
	//! @return true when the outcome differs from the last one logged for this player
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

	//! Drop the log-state table and the parsed plan; statics outlive a world inside one process.
	static void Reset()
	{
		s_mLastLogged = null;
		TBD_RadioPlan.Reset();
	}
}

//! One player's net list, isomorphic to the RPC parameter list: element i of every column is net i.
class TBD_RadioWire
{
	bool m_bServed; //!< false while the server has no authoritative answer (no slot, no mission); the client keeps asking
	string m_sFactionKey; //!< side the nets belong to; logged on the server, never sent
	string m_sMissionId; //!< mission the nets belong to; tells a repeat from a mission switch

	ref array<string> m_aId = {}; //!< net id per net
	ref array<string> m_aLabel = {}; //!< label per net, cut to `TBD_RadioPlan.MAX_LABEL_CHARS`
	ref array<int> m_aFreqKHz = {}; //!< frequency in kHz per net
	ref array<int> m_aLongRange = {}; //!< 1 for `range: long` (backpack), 0 otherwise (handheld), per net

	string m_sTuneResult; //!< `TBD_ERadioTuneResult` by name, so the wire is not coupled to enum values
	int m_iTuned; //!< nets read back off a transceiver after tuning; 0 on a world without a backbone
	string m_sTuneDetail; //!< free-text detail of the tune; empty when none
	string m_sRefusal; //!< why the server declined; logged, never shown to the player

	//! @return the number of nets
	int Count()
	{
		return m_aId.Count();
	}
}
