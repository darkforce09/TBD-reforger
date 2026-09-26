/**
 * @file TBD_BriefingWireSelfCheck.c
 * @brief Round-trips a payload full of empty fields through the briefing wire once per process.
 *
 * Role: proves on the running engine build that TBD_BriefingWire keeps empty fields and keeps each
 * role under its own group, and reports whether the native `string.Split` keeps empty tokens.
 * Position: armed by TBD_FrameworkManager's component roll-call at boot (through
 * TBD_BriefingService.SelfCheckWire) and by TBD_BriefingWire.Serialise; writes one Briefing log line.
 * State: `s_bWireChecked`, a process-wide static, so the check runs once per process and a world
 * change does not repeat it.  Invariants: the flag is set before the round trip, so the
 * re-entry through Serialise returns at once; the verdict line is written whether the round trip
 * passes or fails; `split-empties=` reports the measured token count, not a constant.
 */

//! Once-per-process lossless round-trip check of the briefing wire.
class TBD_BriefingWireSelfCheck
{
	protected static bool s_bWireChecked; //!< the check has run in this process; default false

	//! Round-trip a payload whose free-text fields are empty and log the verdict: an EVENT line
	//! `wire self-check PASS empty-fields=lossless split-empties=<kept|dropped>`, or an ERROR line
	//! naming each field that did not survive. Runs once per process; later calls return true.
	//! @return true when the round trip is lossless or the check has already run
	static bool Run()
	{
		if (s_bWireChecked)
			return true;

		// Set before the round trip: `Serialise` calls back into this method.
		s_bWireChecked = true;

		array<string> probe = {};
		string sample = "a" + TBD_WireCodec.FIELD_SEP + TBD_WireCodec.FIELD_SEP + "b";
		sample.Split(TBD_WireCodec.FIELD_SEP, probe, false);

		string splitVerdict = "dropped";
		if (probe.Count() >= 3)
			splitVerdict = "kept";

		TBD_BriefingPayload sent = new TBD_BriefingPayload();
		sent.m_bHasSlot = true;
		sent.m_sMissionName = string.Empty;   // meta.name: minLength 1 in the schema, unenforced on the profile path
		sent.m_sTerrain = "everon";
		sent.m_sFactionKey = string.Empty;
		sent.m_sFactionName = "US Army";
		sent.m_sOwnGroup = string.Empty;      // slot.groupCallsign: same
		sent.m_sOwnRole = "SL";
		sent.m_sOwnKit = string.Empty;
		sent.m_aKit.Insert(new TBD_BriefingKitLine("Primary", string.Empty));

		// A leading empty callsign: the group must survive and keep its role.
		sent.m_aGroups.Insert(new TBD_BriefingGroup(string.Empty));
		sent.m_aGroups[0].m_iSeats = 4;
		sent.m_aGroups[0].m_bIsOwn = true;
		sent.m_aGroups[0].m_aRoles.Insert(new TBD_BriefingRole("RFL", 3, false));

		sent.m_aZones.Insert(new TBD_BriefingZone(string.Empty, "area", true));
		sent.m_sWinMode = "Capture";
		sent.m_aEndConditions.Insert("All objectives captured");

		TBD_BriefingPayload got = TBD_BriefingWire.Parse(TBD_BriefingWire.Serialise(sent));

		array<string> faults = {};

		if (got.m_sMissionName != sent.m_sMissionName)
			faults.Insert("missionName");

		if (got.m_sTerrain != sent.m_sTerrain)
			faults.Insert("terrain");

		if (got.m_sFactionKey != sent.m_sFactionKey)
			faults.Insert("factionKey");

		if (got.m_sFactionName != sent.m_sFactionName)
			faults.Insert("factionName");

		if (!got.m_bHasSlot || got.m_sOwnGroup != sent.m_sOwnGroup || got.m_sOwnRole != sent.m_sOwnRole || got.m_sOwnKit != sent.m_sOwnKit)
			faults.Insert("ownSeat");

		if (got.m_aKit.Count() != 1 || got.m_aKit[0].m_sLabel != "Primary" || !got.m_aKit[0].m_sValue.IsEmpty())
			faults.Insert("kit");

		// The one that matters most: the group survived, kept its seat count and its own-flag, and
		// its role stayed attached to IT rather than leaking into a neighbour.
		if (got.m_aGroups.Count() != 1)
		{
			faults.Insert("groupCount");
		}
		else
		{
			TBD_BriefingGroup group = got.m_aGroups[0];
			if (!group.m_sCallsign.IsEmpty() || group.m_iSeats != 4 || !group.m_bIsOwn)
				faults.Insert("group");

			if (group.m_aRoles.Count() != 1 || group.m_aRoles[0].m_sRole != "RFL" || group.m_aRoles[0].m_iCount != 3 || group.m_aRoles[0].m_bIsOwn)
				faults.Insert("role");
		}

		if (got.m_aZones.Count() != 1 || !got.m_aZones[0].m_sTitle.IsEmpty() || got.m_aZones[0].m_sDetail != "area" || !got.m_aZones[0].m_bIsOwn)
			faults.Insert("zone");

		if (got.m_sWinMode != sent.m_sWinMode)
			faults.Insert("winMode");

		if (got.m_aEndConditions.Count() != 1 || got.m_aEndConditions[0] != "All objectives captured")
			faults.Insert("endOn");

		if (faults.IsEmpty())
		{
			// string.Format, never Print(localVariable): Print emits a local's declaration, not
			// its value.
			TBD_Log.Event(TBD_BriefingService.CH_BRIEFING, string.Format(
				"wire self-check PASS empty-fields=lossless split-empties=%1", splitVerdict));
			return true;
		}

		string detail;
		foreach (string fault : faults)
		{
			if (!detail.IsEmpty())
				detail = detail + ",";

			detail = detail + fault;
		}

		TBD_Log.Error(TBD_BriefingService.CH_BRIEFING, string.Format(
			"wire self-check FAIL split-empties=%1 lost=%2 -- an empty briefing field does not survive the wire",
			splitVerdict, detail));
		return false;
	}
}
