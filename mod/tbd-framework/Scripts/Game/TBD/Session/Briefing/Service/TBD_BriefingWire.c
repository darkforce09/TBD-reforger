/**
 * @file TBD_BriefingWire.c
 * @brief Flattens a briefing payload to one RPC string and rebuilds it on the client.
 *
 * Role: the briefing's record set over TBD_WireCodec, plus the orders arrays that ride beside it.
 * Position: SCR_PlayerController's TBD_RpcAsk_Briefing serialises on the server;
 * TBD_RpcDo_Briefing parses, adopts the orders and hands the payload to TBD_BriefingClient.
 * State: none.  Invariants: one record per line, the kind bare and every field marked, so an empty
 * field is still a token and the field counts `Parse` guards on hold; the wire is at most
 * MAX_PAYLOAD_LINES records and a clip warns; a malformed record is skipped, never fatal; a group
 * record that fails to decode drops the role records under it rather than attaching them to the
 * previous group; the written orders never enter the delimited string.
 */

//! Briefing record codec. Record kinds and their fields:
//!   `M` mission   name / terrain / factionKey / factionName
//!   `X` unavailable reason (terminal: nothing else follows)
//!   `S` own seat  group / role / kit
//!   `K` kit line  label / value
//!   `G` group     callsign / seats / isOwn (the `R` lines after it attach to it)
//!   `R` role      role / count / isOwn
//!   `Z` zone      title / detail / isOwn
//!   `W` win mode  label
//!   `E` end-on    one declared round-end trigger
//! A record with an empty field reads `G<TAB>.<TAB>.4<TAB>.1`; no token is ever the empty string.
class TBD_BriefingWire
{
	protected static const int MAX_PAYLOAD_LINES = 400; //!< records per payload; a larger payload is clipped
	protected static const string CLIP_WARNING = "payload clipped at %1 lines (mission has more) -- raise MAX_PAYLOAD_LINES"; //!< clip warning text; %1 = MAX_PAYLOAD_LINES

	//! Flatten `payload` to one string. Arms TBD_BriefingWireSelfCheck on the first call of the
	//! process. The written orders are not in it; they travel as the RPC's array parameters.
	//! @param payload the payload to write; null yields the empty string
	//! @return the wire string, clipped to MAX_PAYLOAD_LINES records
	static string Serialise(TBD_BriefingPayload payload)
	{
		// Guarded inside `Run`, which sets its flag before it re-enters this method.
		TBD_BriefingWireSelfCheck.Run();

		if (!payload)
			return string.Empty;

		array<string> lines = {};

		lines.Insert(TBD_WireCodec.Record4("M", payload.m_sMissionName, payload.m_sTerrain, payload.m_sFactionKey, payload.m_sFactionName));

		if (!payload.IsAvailable())
		{
			lines.Insert(TBD_WireCodec.Record1("X", payload.m_sUnavailableReason));
			return TBD_WireCodec.Join(lines, MAX_PAYLOAD_LINES, TBD_BriefingService.CH_BRIEFING, CLIP_WARNING);
		}

		lines.Insert(TBD_WireCodec.Record3("S", payload.m_sOwnGroup, payload.m_sOwnRole, payload.m_sOwnKit));

		foreach (TBD_BriefingKitLine kit : payload.m_aKit)
		{
			lines.Insert(TBD_WireCodec.Record2("K", kit.m_sLabel, kit.m_sValue));
		}

		foreach (TBD_BriefingGroup group : payload.m_aGroups)
		{
			lines.Insert(TBD_WireCodec.Record3("G", group.m_sCallsign, group.m_iSeats.ToString(), TBD_WireCodec.Flag(group.m_bIsOwn)));

			foreach (TBD_BriefingRole role : group.m_aRoles)
			{
				lines.Insert(TBD_WireCodec.Record3("R", role.m_sRole, role.m_iCount.ToString(), TBD_WireCodec.Flag(role.m_bIsOwn)));
			}
		}

		foreach (TBD_BriefingZone zone : payload.m_aZones)
		{
			lines.Insert(TBD_WireCodec.Record3("Z", zone.m_sTitle, zone.m_sDetail, TBD_WireCodec.Flag(zone.m_bIsOwn)));
		}

		if (!payload.m_sWinMode.IsEmpty())
			lines.Insert(TBD_WireCodec.Record1("W", payload.m_sWinMode));

		foreach (string trigger : payload.m_aEndConditions)
		{
			lines.Insert(TBD_WireCodec.Record1("E", trigger));
		}

		return TBD_WireCodec.Join(lines, MAX_PAYLOAD_LINES, TBD_BriefingService.CH_BRIEFING, CLIP_WARNING);
	}

	//! Rebuild a payload from `wire`. A malformed record is skipped, so a briefing renders what it
	//! can instead of a blank screen.
	//! @param wire the string `Serialise` produced
	//! @return a new payload; an empty `wire` yields one whose unavailable reason says nothing arrived
	static TBD_BriefingPayload Parse(string wire)
	{
		TBD_BriefingPayload payload = new TBD_BriefingPayload();

		if (wire.IsEmpty())
		{
			payload.m_sUnavailableReason = "No briefing received from the server.";
			return payload;
		}

		array<string> lines = {};
		wire.Split(TBD_WireCodec.LINE_SEP, lines, false);

		TBD_BriefingGroup current;

		foreach (string line : lines)
		{
			array<string> f = {};
			line.Split(TBD_WireCodec.FIELD_SEP, f, false);
			if (f.IsEmpty())
				continue;

			string kind = f[0];

			if (kind == "M" && f.Count() >= 5)
			{
				payload.m_sMissionName = TBD_WireCodec.Unmark(f[1]);
				payload.m_sTerrain = TBD_WireCodec.Unmark(f[2]);
				payload.m_sFactionKey = TBD_WireCodec.Unmark(f[3]);
				payload.m_sFactionName = TBD_WireCodec.Unmark(f[4]);
			}
			else if (kind == "X" && f.Count() >= 2)
			{
				payload.m_sUnavailableReason = TBD_WireCodec.Unmark(f[1]);
			}
			else if (kind == "S" && f.Count() >= 4)
			{
				payload.m_bHasSlot = true;
				payload.m_sOwnGroup = TBD_WireCodec.Unmark(f[1]);
				payload.m_sOwnRole = TBD_WireCodec.Unmark(f[2]);
				payload.m_sOwnKit = TBD_WireCodec.Unmark(f[3]);
			}
			else if (kind == "K" && f.Count() >= 3)
			{
				payload.m_aKit.Insert(new TBD_BriefingKitLine(TBD_WireCodec.Unmark(f[1]), TBD_WireCodec.Unmark(f[2])));
			}
			else if (kind == "G")
			{
				// A rejected group clears `current`, so the `R` lines after it are dropped instead
				// of attaching to the previous group: misattributed seats are worse than missing
				// ones. A wire clipped at MAX_PAYLOAD_LINES can still cut a record short.
				current = null;

				if (f.Count() >= 4)
				{
					payload.m_aGroups.Insert(new TBD_BriefingGroup(TBD_WireCodec.Unmark(f[1])));
					current = payload.m_aGroups[payload.m_aGroups.Count() - 1];
					current.m_iSeats = TBD_WireCodec.Unmark(f[2]).ToInt();
					current.m_bIsOwn = TBD_WireCodec.IsSet(f[3]);
				}
				else
				{
					TBD_Log.Warn(TBD_BriefingService.CH_BRIEFING, string.Format(
						"dropped malformed group record (%1 fields) and every role under it", f.Count()));
				}
			}
			else if (kind == "R" && f.Count() >= 4 && current)
			{
				current.m_aRoles.Insert(new TBD_BriefingRole(TBD_WireCodec.Unmark(f[1]), TBD_WireCodec.Unmark(f[2]).ToInt(), TBD_WireCodec.IsSet(f[3])));
			}
			else if (kind == "Z" && f.Count() >= 4)
			{
				payload.m_aZones.Insert(new TBD_BriefingZone(TBD_WireCodec.Unmark(f[1]), TBD_WireCodec.Unmark(f[2]), TBD_WireCodec.IsSet(f[3])));
			}
			else if (kind == "W" && f.Count() >= 2)
			{
				payload.m_sWinMode = TBD_WireCodec.Unmark(f[1]);
			}
			else if (kind == "E" && f.Count() >= 2)
			{
				payload.m_aEndConditions.Insert(TBD_WireCodec.Unmark(f[1]));
			}
		}

		return payload;
	}

	//! Attach the orders arrays a client received to the payload it parsed. Orders are free prose
	//! with meaningful newlines, so they ride as `array<string>` RPC parameters, one element per
	//! paragraph, with no delimiter to collide with. The arrays are copied because they belong to
	//! the RPC call frame and the payload outlives it on TBD_BriefingClient.
	//! @param payload the parsed payload; null does nothing
	//! @param situation the situation paragraphs; null means none
	//! @param mission the mission paragraphs; null means none
	//! @param execution the execution paragraphs; null means none
	static void AdoptOrders(TBD_BriefingPayload payload, array<string> situation, array<string> mission, array<string> execution)
	{
		if (!payload)
			return;

		CopyInto(payload.m_aSituation, situation);
		CopyInto(payload.m_aMission, mission);
		CopyInto(payload.m_aExecution, execution);
	}

	//! Replace `destination`'s contents with a copy of `source`. A null `source` is what an RPC
	//! array parameter is when the sender had nothing to send, and leaves `destination` empty.
	//! @param destination the payload array to fill
	//! @param source the received array; may be null
	protected static void CopyInto(array<string> destination, array<string> source)
	{
		destination.Clear();

		if (!source)
			return;

		foreach (string line : source)
		{
			destination.Insert(line);
		}
	}
}
