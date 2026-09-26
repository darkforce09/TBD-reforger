/**
 * @file TBD_ConnectionEpochs.c
 * @brief Connection epochs: one stamp per sitting of a numeric player id.
 *
 * Role: answers "is the player on this id still the one a deferred callback or a held record was
 * made for".  Position: owned by TBD_SpawnManager; stamped by every deferred per-player callback of
 * the spawn helpers and by TBD_SpectatorHost and TBD_DeploymentAuthorization records.
 * State: the epoch map and its sequence, server only, owned by the manager.  Invariants: epochs are
 * positive and never reused; 0 means nobody is connected under the id and never matches; only the
 * join hook and disconnect advance or close an epoch.
 */

//! Per-player connection epochs. ScriptCallQueue cancels by function only, so every per-player
//! deferred callback quotes the epoch it was scheduled under and does nothing once it moved on.
class TBD_ConnectionEpochs : Managed
{
	protected ref map<int, int> m_mConnectEpoch; //!< playerId to its live epoch; no row when disconnected
	protected int m_iConnectEpochSeq; //!< last epoch issued; pre-incremented, so live epochs start at 1

	//! Create an empty epoch table.
	void TBD_ConnectionEpochs()
	{
		m_mConnectEpoch = new map<int, int>();
	}

	//! The live epoch of `playerId`, or 0 when nobody is connected under that id.
	//! @authority server
	int ConnectEpochOf(int playerId)
	{
		int epoch;
		m_mConnectEpoch.Find(playerId, epoch);
		return epoch;
	}

	//! The epoch to stamp a deferred callback with. Opens one on demand for a connected player who
	//! never passed the join hook (a listen host's own player); returns 0, which never matches, for
	//! an id with no player controller.
	//! @authority server
	int EnsureConnectEpoch(int playerId)
	{
		int epoch = ConnectEpochOf(playerId);
		if (epoch != 0)
			return epoch;

		PlayerManager players = GetGame().GetPlayerManager();
		if (!players || !players.GetPlayerController(playerId))
			return 0;

		m_iConnectEpochSeq++;
		m_mConnectEpoch.Set(playerId, m_iConnectEpochSeq);
		return m_iConnectEpochSeq;
	}

	//! True when `epoch` is non-zero and still the live epoch of `playerId`: false after a
	//! disconnect and for whoever is later handed that recycled id.
	//! @authority server
	bool IsSameConnection(int playerId, int epoch)
	{
		return epoch != 0 && ConnectEpochOf(playerId) == epoch;
	}

	//! Open a fresh epoch for a new sitting of `playerId`, retiring any earlier one.
	//! @return the new epoch
	//! @authority server
	int OpenFreshEpoch(int playerId)
	{
		m_iConnectEpochSeq++;
		int epoch = m_iConnectEpochSeq;
		m_mConnectEpoch.Set(playerId, epoch);
		return epoch;
	}

	//! Close the epoch of a departing player, so every callback stamped with it does nothing.
	//! @authority server
	void CloseEpoch(int playerId)
	{
		m_mConnectEpoch.Remove(playerId);
	}
}
