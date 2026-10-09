/**
 * @file TBD_TriggerPlayerSnapshot.c
 * @brief One tick's view of which live players stand where, on which side.
 *
 * Role: the presence evidence every player-based trigger condition reads.  Position: captured by
 * `TBD_TriggerRuntime.Tick` once per evaluation; read by `TBD_TriggerConditions`.
 * State: four parallel arrays (id, X, Z, faction key), rebuilt by `Capture`, owned by the server's
 * trigger runtime.  Invariants: a player is in the snapshot only with a live body and an unspent
 * life; `Capture` returns false, never an empty snapshot, when presence cannot be read at all.
 */

//! Per-tick player positions and sides.
//! @authority server
class TBD_TriggerPlayerSnapshot : Managed
{
	protected ref array<int> m_aPlayerIds; //!< player ids in the snapshot
	protected ref array<float> m_aPlayerX; //!< world X in metres, parallel to `m_aPlayerIds`
	protected ref array<float> m_aPlayerZ; //!< world Z in metres, parallel to `m_aPlayerIds`
	protected ref array<string> m_aPlayerFaction; //!< slot faction key, empty when unassigned; parallel

	//! Rebuild the snapshot from the live player list. A player is kept only with a body that is
	//! alive and a life that is not spent, since a dead player's controlled entity is their
	//! spectator streaming host, not a soldier.
	//! @return false when there is no `PlayerManager` or no `TBD_SpawnManager`: without the spawn
	//! manager a spent life cannot be told from a living one, and an empty snapshot would make every
	//! `not_present` condition hold
	bool Capture()
	{
		m_aPlayerIds = new array<int>();
		m_aPlayerX = new array<float>();
		m_aPlayerZ = new array<float>();
		m_aPlayerFaction = new array<string>();

		PlayerManager players = GetGame().GetPlayerManager();
		if (!players)
			return false;

		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		if (!spawn)
			return false;

		array<int> connected = new array<int>();
		players.GetPlayers(connected);

		foreach (int playerId : connected)
		{
			if (spawn.IsPlayerDead(playerId))
				continue;

			IEntity body = players.GetPlayerControlledEntity(playerId);
			if (!body || TBD_CharacterUtil.IsDead(body))
				continue;

			string factionKey = TBD_PlayerFaction.Of(spawn, playerId);

			vector origin = body.GetOrigin();
			m_aPlayerIds.Insert(playerId);
			m_aPlayerX.Insert(origin[0]);
			m_aPlayerZ.Insert(origin[2]);
			m_aPlayerFaction.Insert(factionKey);
		}

		return true;
	}

	//! Count snapshot players inside `zone` on the side `factionKey` selects. A player with no
	//! resolved slot has no side and is counted by neither mode, so an unassigned body cannot
	//! contest a seizure.
	//! @param zone the area; null = the whole world (a document-wide condition)
	//! @param factionKey the side; with `matching` true an empty key matches everybody
	//! @param matching true counts players of `factionKey`, false players of any other named side
	//! @return the count, 0 before the first `Capture`
	int CountInside(TBD_Zone zone, string factionKey, bool matching)
	{
		if (!m_aPlayerIds)
			return 0;

		int hits = 0;
		int count = m_aPlayerIds.Count();
		for (int i = 0; i < count; i++)
		{
			string playerFaction = m_aPlayerFaction[i];

			if (matching)
			{
				if (!factionKey.IsEmpty() && playerFaction != factionKey)
					continue;
			}
			else
			{
				if (playerFaction.IsEmpty() || playerFaction == factionKey)
					continue;
			}

			if (zone && !zone.Contains(m_aPlayerX[i], m_aPlayerZ[i]))
				continue;

			hits++;
		}

		return hits;
	}

	//! Count snapshot players inside `zone` whose faction is unresolved (between joining and slot
	//! assignment, or mid-respawn). `not_present` refuses to hold while this is non-zero, since
	//! it cannot say a side is absent over a body it cannot identify.
	//! @param zone the area; null = the whole world
	//! @return the count, 0 before the first `Capture`
	int CountUnknownInside(TBD_Zone zone)
	{
		if (!m_aPlayerIds)
			return 0;

		int hits = 0;
		int count = m_aPlayerIds.Count();
		for (int i = 0; i < count; i++)
		{
			if (!m_aPlayerFaction[i].IsEmpty())
				continue;

			if (zone && !zone.Contains(m_aPlayerX[i], m_aPlayerZ[i]))
				continue;

			hits++;
		}

		return hits;
	}

	//! Whether a player of another named side stands inside `zone` with a player of `ownerSide`
	//! within `radiusM` of them, measured in XZ with no line of sight traced. The observer does not
	//! have to be inside the zone.
	//! @param zone the watched area
	//! @param ownerSide the observing side
	//! @param radiusM the detection radius in metres
	//! @return true on the first intruder with an observer in range
	bool IsIntruderSpotted(notnull TBD_Zone zone, string ownerSide, float radiusM)
	{
		if (!m_aPlayerIds)
			return false;

		float radiusSq = radiusM * radiusM;
		int count = m_aPlayerIds.Count();

		for (int i = 0; i < count; i++)
		{
			string intruderFaction = m_aPlayerFaction[i];
			if (intruderFaction.IsEmpty() || intruderFaction == ownerSide)
				continue;

			if (!zone.Contains(m_aPlayerX[i], m_aPlayerZ[i]))
				continue;

			for (int j = 0; j < count; j++)
			{
				if (m_aPlayerFaction[j] != ownerSide)
					continue;

				float distSq = TBD_ZoneGeometry.DistanceSqXZ(m_aPlayerX[j], m_aPlayerZ[j],
					m_aPlayerX[i], m_aPlayerZ[i]);

				if (distSq <= radiusSq)
					return true;
			}
		}

		return false;
	}
}
