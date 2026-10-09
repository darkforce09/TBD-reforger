/**
 * @file TBD_MarkerService.c
 * @brief Server half of mission map markers: which markers a player may see.
 *
 * Role: builds one player's marker set from the mission briefing of that player's side.
 * Position: called by the modded `SCR_PlayerController` marker request (RPC or in place on a
 * listen host); reads `TBD_SpawnManager.GetAssignedSlot` and `TBD_MissionLoader`; its
 * `TBD_MarkerWire` answer goes to `TBD_MarkerStyleCodec.PackIntoX` and then to the requester.
 * State: the per-player last logged outcome, static, on the server.
 * Invariants: markers are side-scoped intelligence (`briefings` is keyed by faction). The request
 * takes a player id and nothing else, the side comes from server-owned slot state, and only that
 * side's rows enter the wire, so another side's markers never leave the server. A player without
 * a slot gets `served = false` and no rows (fail closed). At most `MAX_MARKERS` rows are sent and
 * truncation is logged; labels are cut to `MAX_LABEL_CHARS`, never dropped.
 */

//! Server-side marker set builder.
//! @authority server
class TBD_MarkerService
{
	static const string CH_MARKERS = "Markers"; //!< log channel of every marker line
	static const int MAX_MARKERS = 64; //!< most rows sent to one client; the schema has no `maxItems`
	static const int MAX_LABEL_CHARS = 64; //!< longest label sent; the schema has no `maxLength`

	protected static ref map<int, string> s_mLastLogged; //!< player id -> last logged outcome; null until first use
	protected static const int MAX_LOG_STATES = 256; //!< entries after which `s_mLastLogged` is dropped whole

	//! Build one player's marker set and log the outcome when it changed. Logging here covers the
	//! listen-host path, which skips the RPC handler.
	//! @param playerId the requesting player
	//! @return never null: `served = false` with a refusal reason when the player has no slot or
	//! no mission is loaded; otherwise served, with the side's rows (possibly none)
	static TBD_MarkerWire BuildForPlayer(int playerId)
	{
		TBD_MarkerWire wire = Build(playerId);
		LogOutcome(playerId, wire);

		return wire;
	}

	//! The marker decision itself, without logging.
	//! @param playerId the requesting player
	//! @return the wire, never null
	protected static TBD_MarkerWire Build(int playerId)
	{
		TBD_MarkerWire wire = new TBD_MarkerWire();

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

		// From here on the answer is authoritative even when it has no rows.
		wire.m_bServed = true;

		TBD_MissionBriefingStruct briefing = TBD_MissionLoader.GetBriefingForFaction(slot.faction);
		if (!briefing || !briefing.markers)
			return wire;

		int total = briefing.markers.Count();
		int sent = 0;

		foreach (TBD_MissionMarkerStruct marker : briefing.markers)
		{
			if (!marker)
				continue;

			if (sent >= MAX_MARKERS)
				break;

			// Marker positions are world (X, Z), matching the schema's `{x, z}`.
			wire.m_aX.Insert(TBD_Rounding.RoundToInt(marker.x));
			wire.m_aZ.Insert(TBD_Rounding.RoundToInt(marker.z));
			wire.m_aIcon.Insert(marker.icon);
			wire.m_aLabel.Insert(CapLabel(marker.label));

			int sizeFp = -1;
			if (marker.size > 0)
				sizeFp = TBD_Rounding.RoundToInt(marker.size * 100);

			int rotationFp = -1;
			if (marker.rotationDeg >= 0)
				rotationFp = TBD_Rounding.RoundToInt(marker.rotationDeg * 100);

			int alpha255 = -1;
			if (marker.alpha >= 0)
				alpha255 = TBD_Rounding.RoundToInt(marker.alpha * 255);

			wire.m_aSizeFp.Insert(sizeFp);
			wire.m_aRotationFp.Insert(rotationFp);
			wire.m_aShape.Insert(marker.shape);
			wire.m_aBrush.Insert(marker.brush);
			wire.m_aColorHex.Insert(marker.color);
			wire.m_aAlpha255.Insert(alpha255);

			sent++;
		}

		if (total > sent)
		{
			TBD_Log.Warn(CH_MARKERS, string.Format(
				"mission '%1' authored %2 markers for faction '%3'; sent the first %4 (cap %5).",
				wire.m_sMissionId, total, wire.m_sFactionKey, sent, MAX_MARKERS));
		}

		return wire;
	}

	//! Log one line per player, only when the player's outcome changed. Both outcomes log at
	//! NORMAL level: an unslotted player asking during the lobby is the ordinary case.
	//! @param playerId the requesting player
	//! @param wire the built answer
	protected static void LogOutcome(int playerId, TBD_MarkerWire wire)
	{
		if (!wire.m_bServed)
		{
			if (ShouldLog(playerId, "refused:" + wire.m_sRefusal))
			{
				TBD_Log.Kv(CH_MARKERS, "refused",
					string.Format("player=%1 reason='%2'", playerId, wire.m_sRefusal));
			}

			return;
		}

		string outcome = string.Format("served:%1:%2:%3",
			wire.m_sFactionKey, wire.m_sMissionId, wire.Count());

		if (!ShouldLog(playerId, outcome))
			return;

		TBD_Log.Kv(CH_MARKERS, "served", string.Format(
			"player=%1 faction=%2 mission=%3 markers=%4",
			playerId, wire.m_sFactionKey, wire.m_sMissionId, wire.Count()));
	}

	//! Record the player's outcome; an unslotted client re-asks every few seconds, so identical
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

	//! Cut the label to `MAX_LABEL_CHARS`; an over-long caption never withholds the marker.
	//! @return the label, truncated when longer than the cap
	protected static string CapLabel(string label)
	{
		if (label.Length() <= MAX_LABEL_CHARS)
			return label;

		return label.Substring(0, MAX_LABEL_CHARS);
	}
}
