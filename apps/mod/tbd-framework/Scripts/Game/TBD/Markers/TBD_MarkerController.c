//! T-181.19 — the wire that carries one player's map markers, and nobody else's.
//!
//! ── Why the transport hangs off SCR_PlayerController ────────────────────────────────────────
//! The player controller is replicated and owned by exactly one client, so `RplRcver.Owner`
//! delivers a reply to the requester and to NOBODY ELSE. That is not a convenience here, it is the
//! side-discipline mechanism: a broadcast channel would put both sides' markers on the wire and
//! leave "don't look at the other one" as a promise a modified client could break. This is also the
//! precedent already in the tree — `TBD_MissionBrowser.c` and `TBD_BriefingController.c` move
//! per-player payloads exactly this way.
//!
//! ── The four-modded-block question, stated rather than hidden ───────────────────────────────
//! This is the FOURTH `modded class SCR_PlayerController` block in the addon (mission browser,
//! briefing, lobby, markers). Three were proven by the wave-3 verifier to merge statically with a
//! negative-controlled probe, and this one compiles alongside them on the same gate. What this
//! lane still cannot prove is runtime coexistence — that is the open question filed as T-181.25.
//! This block deliberately keeps its exposure minimal:
//!   * it overrides NO vanilla method (the stacking risk T-181.9.1 avoided by moving the lobby to
//!     a game-mode component is a risk about overriding `OnControlledEntityChanged`, not about
//!     adding new methods);
//!   * it adds NO `modded enum ChimeraMenuPreset` entry, so it contributes nothing to the menu
//!     preset collision risk that is the substance of T-181.25;
//!   * every symbol it introduces is `TBD_`-prefixed.
//!
//! ── Host vs dedicated ───────────────────────────────────────────────────────────────────────
//! On a listen host the requester IS the authority, so the request short-circuits and builds the
//! payload in place instead of RPCing the machine to itself. Same code path, both topologies —
//! and the reason this does not have the recorded `onRplName`-only bug is that there is no
//! replication callback in this design at all.
modded class SCR_PlayerController
{
	//------------------------------------------------------------------------------------------------
	//! CLIENT (owner) -> SERVER: "what markers am I allowed to see?"
	//!
	//! Takes no arguments, and that is the point: there is no faction parameter for a client to
	//! forge. See `TBD_MarkerData.c` for the full three-property argument.
	void TBD_RequestMarkers()
	{
		// Authority only — a dedicated client has no mission document and must ask; on a listen
		// host this controller already IS the authority, so RPCing ourselves would be a round trip
		// to nowhere.
		if (RplSession.Mode() == RplMode.Client)
		{
			Rpc(TBD_RpcAsk_Markers);
			return;
		}

		TBD_MarkerWire wire = TBD_MarkerService.BuildForPlayer(GetPlayerId());
		TBD_MarkerClient.Accept(wire.m_aX, wire.m_aZ, wire.m_aIcon, wire.m_aLabel,
			wire.m_sFactionKey, wire.m_sMissionId, wire.m_bServed);
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server — resolve the caller's side from server-owned state and answer with THAT
	//! side's markers only.
	//! @rpc Reliable Server
	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void TBD_RpcAsk_Markers()
	{
		// `BuildForPlayer` does its own change-gated logging, so both this path and the listen-host
		// short-circuit above produce the same operator-visible record.
		TBD_MarkerWire wire = TBD_MarkerService.BuildForPlayer(GetPlayerId());

		Rpc(TBD_RpcDo_Markers, wire.m_aX, wire.m_aZ, wire.m_aIcon, wire.m_aLabel,
			wire.m_sFactionKey, wire.m_sMissionId, wire.m_bServed);
	}

	//------------------------------------------------------------------------------------------------
	//! @authority owner — executes on the requesting client and no other (RplRcver.Owner).
	//!
	//! Four PARALLEL ARRAYS, not a delimited string. `string.Split`'s empty-token behaviour is a
	//! runtime property no probe on this lane can settle, and an EMPTY `label` is schema-legal, so
	//! a delimited format would have shipped a landmine. Positional arrays have no such state:
	//! element i of each array is field i of marker i, and an empty label is an empty element.
	//! `array<int>` / `array<string>` RPC parameters are proven in both oracles and probed here.
	//! @rpc Reliable Owner
	[RplRpc(RplChannel.Reliable, RplRcver.Owner)]
	protected void TBD_RpcDo_Markers(array<int> xs, array<int> zs, array<string> icons,
		array<string> labels, string factionKey, string missionId, bool served)
	{
		TBD_MarkerClient.Accept(xs, zs, icons, labels, factionKey, missionId, served);
	}
}
