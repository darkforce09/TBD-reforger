/**
 * @file TBD_OneLifeLedger.c
 * @brief ONE LIFE: who has spent their life, keyed on the durable bind key.
 *
 * Role: records spent lives and pending admin respawns.  Position: owned by TBD_SpawnManager;
 * written by the death flow and the admin respawn; read by every deploy path, the lobby, the win
 * conditions and the admin screens through TBD_SpawnManager.IsPlayerDead.
 * State: bind key to spent, and the players whose admin respawn waits on a retry; server only.
 * Invariants: keyed on the bind key, never the numeric id, so quitting and rejoining under a new id
 * keeps the life spent; a life is given back only by an admin respawn that put a body in the
 * player's hands.
 */

//! The one-life ledger of the spawn manager.
class TBD_OneLifeLedger : Managed
{
	protected TBD_SpawnManager m_Spawn; //!< owning manager
	protected ref map<string, bool> m_mDeadPlayers; //!< bind keys that spent their life
	protected ref map<int, bool> m_mAdminRespawnPending; //!< players whose admin respawn waits on a retry or a platform decision

	//! Bind the ledger to its manager.
	void TBD_OneLifeLedger(TBD_SpawnManager spawn)
	{
		m_Spawn = spawn;
		m_mDeadPlayers = new map<string, bool>();
		m_mAdminRespawnPending = new map<int, bool>();
	}

	//! True when `playerId` has spent their life, resolved through their bind key.
	bool IsPlayerDead(int playerId)
	{
		return IsBindKeyDead(m_Spawn.GetIdentity().PlayerBindKey(playerId));
	}

	//! True when `bindKey` has spent its life.
	bool IsBindKeyDead(string bindKey)
	{
		return m_mDeadPlayers && m_mDeadPlayers.Contains(bindKey);
	}

	//! Spend the life of `playerId`; warns when the key it is recorded against is not durable.
	void MarkLifeSpent(int playerId)
	{
		TBD_SpawnIdentityKeys identity = m_Spawn.GetIdentity();
		string bindKey = identity.PlayerBindKey(playerId);
		m_mDeadPlayers.Set(bindKey, true);
		if (!TBD_SpawnIdentityKeys.IsDurableKey(bindKey))
			identity.NoteIdentityDegraded(playerId, TBD_SpawnIdentityKeys.KeyModeLabel(bindKey), "the key this death was recorded against is not durable, so it may not survive a reconnect");
	}

	//! Give the life of `playerId` back. Only the admin respawn calls it, after a body landed.
	void ClearLifeSpent(int playerId)
	{
		m_mDeadPlayers.Remove(m_Spawn.GetIdentity().PlayerBindKey(playerId));
	}

	//! Drop the spent mark of `bindKey`: a NUMERIC key names the next holder of that id once its
	//! player left.
	void ForgetBindKey(string bindKey)
	{
		m_mDeadPlayers.Remove(bindKey);
	}

	//! True while an admin respawn of `playerId` waits on a retry or a platform decision.
	bool IsAdminRespawnPending(int playerId)
	{
		return m_mAdminRespawnPending.Contains(playerId);
	}

	//! Mark an admin respawn of `playerId` as pending, so its retries carry the admin override.
	void SetAdminRespawnPending(int playerId)
	{
		m_mAdminRespawnPending.Set(playerId, true);
	}

	//! Clear the pending admin respawn of `playerId`.
	void ClearAdminRespawnPending(int playerId)
	{
		m_mAdminRespawnPending.Remove(playerId);
	}
}
