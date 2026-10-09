/**
 * @file TBD_LobbyRosterWire.c
 * @brief Flattens a lobby roster to one RPC string and rebuilds it on the client.
 *
 * Role: the lobby's record set over TBD_WireCodec.  Position: the modded SCR_PlayerController
 * serialises TBD_LobbyService's roster on the server (or in place on a listen host);
 * TBD_LobbyClient.Accept parses the reply; TBD_LobbyRosterWireSelfCheck round-trips both at boot.
 * State: none.  Invariants: one record per line, the kind bare and every field marked, so an empty
 * field is still a token; at most MAX_PAYLOAD_LINES records and a clip warns; counts are never on
 * the wire (TBD_LobbyRoster.Recount derives them); a malformed record is skipped, never fatal; a
 * side or squad record that fails to decode drops the rows under it rather than attaching them to
 * the previous side or squad; the `L` and `D` records precede the terminal `X`.
 */

//! Lobby roster record codec. Record kinds and their fields:
//!   `M` mission     name / terrain / stage
//!   `V` verdict     action / ok / reason / slotKey (what the server just did for this reader)
//!   `L` life spent  "1" when this reader has spent their life
//!   `D` in world    "1" when this reader already controls a body
//!   `X` unavailable reason (terminal: nothing else follows)
//!   `F` side        key / name (the `G` lines after it attach to it)
//!   `G` squad       callsign (the `S` lines after it attach to it)
//!   `S` seat        key / role / state / isOwn / holder
//! A record with an empty field reads `G<TAB>.`; no token is ever the empty string.
class TBD_LobbyRosterWire
{
	static const int MAX_PAYLOAD_LINES = 600; //!< records per payload: 128 seats, their squads and sides with headroom; a larger payload is clipped
	static const string CLIP_WARNING = "roster clipped at %1 lines (mission has more) - raise MAX_PAYLOAD_LINES"; //!< clip warning text; %1 = MAX_PAYLOAD_LINES

	//! Flatten `roster` to one string: `M`, then `V`, `L` and `D` when set, then either `X` or every side, squad and seat.
	//! @param roster the roster to write; null yields the empty string
	//! @return the wire string, clipped to MAX_PAYLOAD_LINES records
	static string Serialise(TBD_LobbyRoster roster)
	{
		if (!roster)
			return string.Empty;

		array<string> lines = {};

		lines.Insert(TBD_WireCodec.Record(3, "M", roster.m_sMissionName, roster.m_sTerrain, roster.m_sStage, string.Empty, string.Empty));

		if (!roster.m_sAction.IsEmpty())
			lines.Insert(TBD_WireCodec.Record(4, "V", roster.m_sAction, TBD_WireCodec.Flag(roster.m_bActionOk), roster.m_sActionReason, roster.m_sActionKey, string.Empty));

		if (roster.m_bLifeSpent)
			lines.Insert(TBD_WireCodec.Record(1, "L", "1", string.Empty, string.Empty, string.Empty, string.Empty));

		// Above the terminal `X`, with `L`: "you already have a body" survives a reply whose roster could
		// not be built.
		if (roster.m_bInWorld)
			lines.Insert(TBD_WireCodec.Record(1, "D", "1", string.Empty, string.Empty, string.Empty, string.Empty));

		if (!roster.IsAvailable())
		{
			lines.Insert(TBD_WireCodec.Record(1, "X", roster.m_sUnavailableReason, string.Empty, string.Empty, string.Empty, string.Empty));
			return TBD_WireCodec.Join(lines, MAX_PAYLOAD_LINES, TBD_LobbyService.CH_LOBBY, CLIP_WARNING);
		}

		foreach (TBD_LobbySide side : roster.m_aSides)
		{
			lines.Insert(TBD_WireCodec.Record(2, "F", side.m_sKey, side.m_sName, string.Empty, string.Empty, string.Empty));

			foreach (TBD_LobbyGroup group : side.m_aGroups)
			{
				lines.Insert(TBD_WireCodec.Record(1, "G", group.m_sCallsign, string.Empty, string.Empty, string.Empty, string.Empty));

				foreach (TBD_LobbySlot slot : group.m_aSlots)
				{
					lines.Insert(TBD_WireCodec.Record(5, "S", slot.m_sKey, slot.m_sRole, slot.m_sState, TBD_WireCodec.Flag(slot.m_bIsOwn), slot.m_sHolder));
				}
			}
		}

		return TBD_WireCodec.Join(lines, MAX_PAYLOAD_LINES, TBD_LobbyService.CH_LOBBY, CLIP_WARNING);
	}

	//! Rebuild a roster from `wire`, skipping any malformed record, then Recount it. An absent `D` reads as not in the world, which keeps the picker up.
	//! @param wire the string Serialise wrote
	//! @return the roster; carries an unavailable reason when `wire` is empty
	static TBD_LobbyRoster Parse(string wire)
	{
		TBD_LobbyRoster roster = new TBD_LobbyRoster();

		if (wire.IsEmpty())
		{
			roster.m_sUnavailableReason = "No roster received from the server.";
			return roster;
		}

		array<string> lines = {};
		wire.Split(TBD_WireCodec.LINE_SEP, lines, false);

		TBD_LobbySide side;
		TBD_LobbyGroup group;

		foreach (string line : lines)
		{
			array<string> f = {};
			line.Split(TBD_WireCodec.FIELD_SEP, f, false);
			if (f.IsEmpty())
				continue;

			string kind = f[0];

			if (kind == "M" && f.Count() >= 4)
			{
				roster.m_sMissionName = TBD_WireCodec.Unmark(f[1]);
				roster.m_sTerrain = TBD_WireCodec.Unmark(f[2]);
				roster.m_sStage = TBD_WireCodec.Unmark(f[3]);
			}
			else if (kind == "V" && f.Count() >= 5)
			{
				roster.m_sAction = TBD_WireCodec.Unmark(f[1]);
				roster.m_bActionOk = TBD_WireCodec.IsSet(f[2]);
				roster.m_sActionReason = TBD_WireCodec.Unmark(f[3]);
				roster.m_sActionKey = TBD_WireCodec.Unmark(f[4]);
			}
			else if (kind == "L" && f.Count() >= 2)
			{
				roster.m_bLifeSpent = TBD_WireCodec.IsSet(f[1]);
			}
			else if (kind == "D" && f.Count() >= 2)
			{
				roster.m_bInWorld = TBD_WireCodec.IsSet(f[1]);
			}
			else if (kind == "X" && f.Count() >= 2)
			{
				roster.m_sUnavailableReason = TBD_WireCodec.Unmark(f[1]);
			}
			else if (kind == "F")
			{
				// Cursor cleared first: the squads after a rejected side are dropped, never attached to the
				// previous side.
				side = null;
				group = null;

				if (f.Count() >= 3)
				{
					roster.m_aSides.Insert(new TBD_LobbySide(TBD_WireCodec.Unmark(f[1]), TBD_WireCodec.Unmark(f[2])));
					side = roster.m_aSides[roster.m_aSides.Count() - 1];
				}
			}
			else if (kind == "G")
			{
				// The same one level down: a rejected squad never donates its seats to the squad above it.
				group = null;

				if (f.Count() >= 2 && side)
				{
					side.m_aGroups.Insert(new TBD_LobbyGroup(TBD_WireCodec.Unmark(f[1])));
					group = side.m_aGroups[side.m_aGroups.Count() - 1];
				}
			}
			else if (kind == "S" && f.Count() >= 6 && group)
			{
				group.m_aSlots.Insert(new TBD_LobbySlot(TBD_WireCodec.Unmark(f[1]), TBD_WireCodec.Unmark(f[2]), TBD_WireCodec.Unmark(f[3]), TBD_WireCodec.Unmark(f[5]), TBD_WireCodec.IsSet(f[4])));
			}
		}

		roster.Recount();
		return roster;
	}
}
