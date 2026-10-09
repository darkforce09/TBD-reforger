/**
 * @file TBD_PossessTicketLedger.c
 * @brief Spawn tickets: the only spawn requests a framework world honours.
 *
 * Role: issues one ticket per cleared deploy, naming the exact body the player may take over, and
 * answers whether a spawn request is covered by one.  Position: issued by
 * TBD_DeployExecutor.HandPlayerOntoBody; checked by the modded SCR_PossessSpawnHandlerComponent
 * (possess requests) and SCR_RespawnSystemComponent.CanRequestSpawn_S (every other handler); spent
 * by the possess handler.
 * State: playerId to ticket, the ticket sequence, the per-player deny-log latch; server only.
 * Invariants: a ticket always names a body and matches only that body; a null target never
 * matches; a ticket lives at most SPAWN_AUTH_WINDOW_MS and its timer closes only its own ticket.
 */

//! One authorized spawn: one player, one target entity, spent on first use.
class TBD_SpawnTicket
{
	int epoch; //!< sequence number of the issuing call; the timeout closes only its own ticket
	IEntity target; //!< the only entity this ticket authorizes; never null
}

//! Spawn tickets of the spawn manager.
class TBD_PossessTicketLedger : Managed
{
	protected const int SPAWN_AUTH_WINDOW_MS = 5000; //!< ticket lifetime (ms); covers the request hop into CanHandleRequest_S, which runs before any preload

	protected ref map<int, ref TBD_SpawnTicket> m_mSpawnAuthorized; //!< playerId to the open ticket
	protected int m_iSpawnAuthEpoch; //!< last ticket sequence number issued
	protected ref map<int, bool> m_mDenyLogged; //!< players whose refusal was already logged since their last ticket

	//! Create empty tables.
	void TBD_PossessTicketLedger()
	{
		m_mSpawnAuthorized = new map<int, ref TBD_SpawnTicket>();
		m_mDenyLogged = new map<int, bool>();
	}

	//! Cancel the pending ticket timeouts.
	void CancelCallbacks()
	{
		GetGame().GetCallqueue().Remove(ExpireSpawnAuthorization);
	}

	//! Open a ticket for `playerId` to take over `target`. Refuses (logged ERROR) without a target.
	//! @authority server
	void AuthorizeSpawn(int playerId, IEntity target)
	{
		if (!target)
		{
			Print(string.Format("[TBD][Spawn] refusing to authorize a spawn for player=%1 with no target body", playerId), LogLevel.ERROR);
			return;
		}

		m_iSpawnAuthEpoch++;

		TBD_SpawnTicket ticket = new TBD_SpawnTicket();
		ticket.epoch = m_iSpawnAuthEpoch;
		ticket.target = target;
		m_mSpawnAuthorized.Set(playerId, ticket);

		m_mDenyLogged.Remove(playerId);
		GetGame().GetCallqueue().CallLater(ExpireSpawnAuthorization, SPAWN_AUTH_WINDOW_MS, false, playerId, m_iSpawnAuthEpoch);
	}

	//! The timeout: close the ticket of `playerId` only when it is still the one numbered `epoch`.
	protected void ExpireSpawnAuthorization(int playerId, int epoch)
	{
		TBD_SpawnTicket held;
		if (m_mSpawnAuthorized.Find(playerId, held) && held && held.epoch == epoch)
			m_mSpawnAuthorized.Remove(playerId);
	}

	//! Spend the ticket once vanilla handed `target` over, so one ticket buys one takeover.
	//! @return false when no matching ticket was open (the direct-bind fallback never spends one)
	//! @authority server
	bool ConsumeSpawnAuthorization(int playerId, IEntity target)
	{
		TBD_SpawnTicket held;
		if (!target || !m_mSpawnAuthorized.Find(playerId, held) || !held || held.target != target)
			return false;

		m_mSpawnAuthorized.Remove(playerId);
		return true;
	}

	//! Close the ticket of `playerId`, if any. Idempotent.
	void RevokeSpawnAuthorization(int playerId)
	{
		m_mSpawnAuthorized.Remove(playerId);
	}

	//! Close the ticket and clear the deny-log latch of `playerId`.
	void ForgetPlayer(int playerId)
	{
		RevokeSpawnAuthorization(playerId);
		m_mDenyLogged.Remove(playerId);
	}

	//! True when an open ticket of `playerId` names `target`. A null target never matches.
	//! @param denyLogOnce set true on the first refusal since the player's last ticket, so a client
	//! asking in a loop is logged once
	bool IsSpawnAuthorizedFor(int playerId, IEntity target, out bool denyLogOnce)
	{
		TBD_SpawnTicket held;
		if (target && m_mSpawnAuthorized.Find(playerId, held) && held && held.target == target)
			return true;

		denyLogOnce = !m_mDenyLogged.Contains(playerId);
		if (denyLogOnce)
			m_mDenyLogged.Set(playerId, true);
		return false;
	}

	//! The entity a spawn request aims at, resolved from the possess request's RplId through
	//! Replication.FindItem, or null when the request type names none.
	static IEntity ResolveSpawnDataEntity(SCR_SpawnData data)
	{
		SCR_PossessSpawnData possessData = SCR_PossessSpawnData.Cast(data);
		if (!possessData)
			return null;

		RplId rplId = possessData.GetRplId();
		if (!rplId.IsValid())
			return null;

		RplComponent rplComponent = RplComponent.Cast(Replication.FindItem(rplId));
		if (!rplComponent)
			return null;

		return rplComponent.GetEntity();
	}
}
