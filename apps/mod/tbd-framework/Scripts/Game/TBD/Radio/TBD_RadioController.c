//! T-181.40 — the wire that carries one player's radio nets, and nobody else's.
//!
//! ── Why the transport hangs off SCR_PlayerController ────────────────────────────────────────
//! The player controller is replicated and owned by exactly one client, so `RplRcver.Owner`
//! delivers a reply to the requester and to NOBODY ELSE. That is not a convenience here, it is the
//! side-discipline mechanism: frequencies are intelligence, and a broadcast channel would put both
//! sides' command nets on the wire and leave "don't tune to that" as a promise a modified client
//! could break. This is also the precedent already in the tree — `TBD_MarkerController.c`,
//! `TBD_MissionBrowser.c` and `TBD_BriefingController.c` move per-player payloads exactly this way.
//!
//! ── The modded-block question, stated rather than hidden ────────────────────────────────────
//! This is the SIXTH `modded class SCR_PlayerController` block in the addon (mission browser,
//! briefing, lobby, spectator host, markers, radio). The measured facts, and only these:
//!   * N blocks COMPILE fine and methods declared in one are callable from the others — verified
//!     at N=2, 3, 5, and now 6 on this slice's own gate.
//!   * Runtime coexistence has NEVER been observed. `world-boot.sh` boots with zero players and
//!     every one of these blocks only does anything when a client is connected. "Compiles" is not
//!     "works", and this is the first thing T-181.25 must settle on a dedicated server.
//! This block keeps its exposure minimal for the same reasons T-181.19 did: it overrides NO
//! vanilla method, adds NO `modded enum ChimeraMenuPreset` entry (so it contributes nothing to the
//! menu-preset collision that is the substance of T-181.25), and every symbol it introduces is
//! `TBD_`-prefixed.
//!
//! ── Host vs dedicated ───────────────────────────────────────────────────────────────────────
//! On a listen host the requester IS the authority, so the request short-circuits and builds the
//! payload in place instead of RPCing the machine to itself. Same code path, both topologies — and
//! the reason this does not have the recorded `onRplName`-only bug is that there is no replication
//! callback in this design at all.
modded class SCR_PlayerController
{
	//------------------------------------------------------------------------------------------------
	//! CLIENT (owner) -> SERVER: "which radio nets am I on?"
	//!
	//! Takes no arguments, and that is the point: there is no faction parameter for a client to
	//! forge. See `TBD_RadioService.c` for the full three-property argument.
	void TBD_RequestRadioNets()
	{
		// Authority only — a dedicated client has no mission document and must ask; on a listen
		// host this controller already IS the authority, so RPCing ourselves would be a round trip
		// to nowhere.
		if (RplSession.Mode() == RplMode.Client)
		{
			Rpc(TBD_RpcAsk_RadioNets);
			return;
		}

		TBD_RadioWire wire = TBD_RadioService.BuildForPlayer(GetPlayerId());
		TBD_RadioClient.Accept(wire.m_aId, wire.m_aLabel, wire.m_aFreqKHz, wire.m_aLongRange,
			wire.m_sMissionId, wire.m_sTuneResult, wire.m_iTuned, wire.m_bServed);
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server — SERVER -> one client, unprompted.
	//!
	//! Why this exists: `TBD_RadioService.OnStageChanged` re-tunes every player's radio at
	//! SAFE_START and LIVE. Without a push, that tune would land on the radio while the player's
	//! on-screen net list still said "not tuned automatically" — the client's poll stops once it
	//! has been served, so it would not find out until it next opened the map. A feature telling
	//! the player something that stopped being true is the exact failure mode this slice is built
	//! to avoid, so the sweep pushes the same wire it just measured.
	//!
	//! THE LISTEN-HOST BRANCH IS NOT OPTIONAL. On a listen host the authority IS the local player,
	//! and an `RplRcver.Owner` RPC is not delivered to the machine that sent it — so wiring only
	//! the Rpc would silently leave the host player's own display stale, which is a recorded
	//! landmine in this program ("client-side UI must be driven from BOTH paths through one guarded
	//! helper"). `GetGame().GetPlayerController() == this` is the same test `TBD_MissionBrowser`
	//! already uses for it.
	void TBD_PushRadioNets(notnull TBD_RadioWire wire)
	{
		if (RplSession.Mode() == RplMode.Client)
			return;

		if (GetGame().GetPlayerController() == this)
		{
			TBD_RadioClient.Accept(wire.m_aId, wire.m_aLabel, wire.m_aFreqKHz, wire.m_aLongRange,
				wire.m_sMissionId, wire.m_sTuneResult, wire.m_iTuned, wire.m_bServed);
			return;
		}

		Rpc(TBD_RpcDo_RadioNets, wire.m_aId, wire.m_aLabel, wire.m_aFreqKHz, wire.m_aLongRange,
			wire.m_sMissionId, wire.m_sTuneResult, wire.m_iTuned, wire.m_bServed);
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server — resolve the caller's side from server-owned state and answer with THAT
	//! side's nets only.
	//! @rpc Reliable Server
	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void TBD_RpcAsk_RadioNets()
	{
		// `BuildForPlayer` does its own change-gated logging, so both this path and the listen-host
		// short-circuit above produce the same operator-visible record. It is also what performs
		// the tune attempt, so a client asking is a client being put on their nets.
		TBD_RadioWire wire = TBD_RadioService.BuildForPlayer(GetPlayerId());

		Rpc(TBD_RpcDo_RadioNets, wire.m_aId, wire.m_aLabel, wire.m_aFreqKHz, wire.m_aLongRange,
			wire.m_sMissionId, wire.m_sTuneResult, wire.m_iTuned, wire.m_bServed);
	}

	//------------------------------------------------------------------------------------------------
	//! @authority owner — executes on the requesting client and no other (RplRcver.Owner).
	//!
	//! FOUR PARALLEL ARRAYS, not a delimited string. A net `label` is authored free text that may
	//! legally contain any delimiter, and `string.Split`'s empty-token behaviour is a RUNTIME
	//! property no probe on this lane can settle — so a delimited format would have shipped a
	//! landmine. Positional arrays have no such state: element i of each array is field i of net i.
	//!
	//! EIGHT parameters, which is the measured ceiling: nine fails with `Too many parameters for
	//! 'Rpc' method`. `m_sFactionKey` is what got cut, and it is the right thing to cut — the client
	//! only ever used it as a diagnostic label and as part of a change-detection fingerprint, and
	//! the net rows themselves already change when the side does, because nets are side-scoped. The
	//! server still logs the faction on its own side of the wire.
	//!
	//! `m_iTuned` and `m_sTuneResult` ride along so the client can tell the player the TRUTH about
	//! whether anything was actually tuned, rather than assuming a served net list means a tuned
	//! radio. On a world with no `RadioManagerEntity` those two say `0` and `NO_BACKBONE`, and the
	//! client says so.
	//! @rpc Reliable Owner
	[RplRpc(RplChannel.Reliable, RplRcver.Owner)]
	protected void TBD_RpcDo_RadioNets(array<string> ids, array<string> labels, array<int> freqKHz,
		array<int> longRange, string missionId, string tuneResult, int tuned, bool served)
	{
		TBD_RadioClient.Accept(ids, labels, freqKHz, longRange, missionId, tuneResult, tuned, served);
	}
}
