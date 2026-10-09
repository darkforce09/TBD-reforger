/**
 * @file TBD_LobbyRosterWireSelfCheck.c
 * @brief Proves once per process, at boot, that the lobby roster wire keeps every field and drops orphan rows.
 *
 * Role: round-trips a roster that is empty in every legal position and carries the literal values
 * `~` and `.`, then parses a wire with a truncated side and squad record, and logs one PASS or FAIL
 * line with the observed `string.Split` empty-token behaviour.  Position: run by
 * TBD_LobbyComponent.OnPostInit on every world boot, including the zero-player headless boot of
 * `cargo xtask mod world-boot`, which never executes the lobby RPCs and treats the FAIL line (an
 * ERROR) as a failed boot.
 * State: `s_bWireChecked` and `s_bWireOk`, process-wide statics, so the check runs once per
 * process and a world change does not repeat it.
 * Invariants: the once-guard sits at Run's own entry, so every caller shares it; faults are
 * collected and reported on one line; the check never depends on the Split behaviour it reports.
 */

//! Boot-time self-check of TBD_LobbyRosterWire.
class TBD_LobbyRosterWireSelfCheck
{
	protected static bool s_bWireChecked; //!< the check has run in this process; default false
	protected static bool s_bWireOk; //!< the verdict of that run; default false

	//! Run both phases once per process and log `wire self-check PASS ... split-empties=<kept|dropped>` on channel Lobby, or an ERROR `wire self-check FAIL ... lost=<faults>`. Later calls return the first verdict.
	//! @return true when both phases are clean; callers may ignore it, the log line is the product
	static bool Run()
	{
		if (s_bWireChecked)
			return s_bWireOk;

		s_bWireChecked = true;

		array<string> probe = {};
		string sample = "a" + TBD_WireCodec.FIELD_SEP + TBD_WireCodec.FIELD_SEP + "b";
		sample.Split(TBD_WireCodec.FIELD_SEP, probe, false);

		string splitVerdict = "dropped";
		if (probe.Count() >= 3)
			splitVerdict = "kept";

		array<string> faults = {};

		CheckRoundTrip(faults);
		CheckOrphanRows(faults);

		if (faults.IsEmpty())
		{
			TBD_Log.Event(TBD_LobbyService.CH_LOBBY, string.Format(
				"wire self-check PASS marker=bijective empty-fields=lossless orphan-rows=dropped split-empties=%1",
				splitVerdict));

			s_bWireOk = true;
			return true;
		}

		string detail;
		foreach (string fault : faults)
		{
			if (!detail.IsEmpty())
				detail = detail + ",";

			detail = detail + fault;
		}

		TBD_Log.Error(TBD_LobbyService.CH_LOBBY, string.Format(
			"wire self-check FAIL split-empties=%1 lost=%2 - a lobby field does not survive the wire",
			splitVerdict, detail));

		s_bWireOk = false;
		return false;
	}

	//! Phase 1: round-trip a roster empty in every legal position and carrying `~` and `.` as values, and append the name of every field that did not survive to `faults`.
	protected static void CheckRoundTrip(notnull array<string> faults)
	{
		TBD_LobbyRoster sent = new TBD_LobbyRoster();
		sent.m_sMissionName = string.Empty; // an empty name: the profile load path does not enforce minLength
		sent.m_sTerrain = "~"; // a literal tilde, which a sentinel scheme would read as empty
		sent.m_sStage = "LOBBY";
		sent.m_bLifeSpent = true;
		sent.m_bInWorld = true; // the in-world fact must survive the wire
		sent.m_sAction = TBD_LobbyService.ACTION_CLAIM;
		sent.m_bActionOk = false;
		sent.m_sActionReason = string.Empty;
		sent.m_sActionKey = TBD_WireCodec.FIELD_MARK; // the field marker itself, as a value

		sent.m_aSides.Insert(new TBD_LobbySide(string.Empty, "~"));
		TBD_LobbySide side = sent.m_aSides[0];

		// A leading empty callsign is the record most likely to take its squad down with it.
		side.m_aGroups.Insert(new TBD_LobbyGroup(string.Empty));
		side.m_aGroups[0].m_aSlots.Insert(new TBD_LobbySlot("a.b", string.Empty, TBD_LobbyService.STATE_OPEN, string.Empty, false));
		side.m_aGroups[0].m_aSlots.Insert(new TBD_LobbySlot("~", "SL", TBD_LobbyService.STATE_HELD, "Bob", true));

		side.m_aGroups.Insert(new TBD_LobbyGroup("BRAVO"));
		side.m_aGroups[1].m_aSlots.Insert(new TBD_LobbySlot("c", "RFL", TBD_LobbyService.STATE_DEAD, string.Empty, false));

		TBD_LobbyRoster got = TBD_LobbyRosterWire.Parse(TBD_LobbyRosterWire.Serialise(sent));

		if (!got.m_sMissionName.IsEmpty())
			faults.Insert("missionName");

		// The bijectivity assertion: a literal tilde comes back as a tilde.
		if (got.m_sTerrain != "~")
			faults.Insert("terrain");

		if (got.m_sStage != "LOBBY")
			faults.Insert("stage");

		if (!got.m_bLifeSpent)
			faults.Insert("lifeSpent");

		if (!got.m_bInWorld)
			faults.Insert("inWorld");

		if (got.m_sAction != TBD_LobbyService.ACTION_CLAIM || got.m_bActionOk || !got.m_sActionReason.IsEmpty() || got.m_sActionKey != TBD_WireCodec.FIELD_MARK)
			faults.Insert("verdict");

		if (got.m_aSides.Count() != 1)
		{
			faults.Insert("sideCount");
			return;
		}

		TBD_LobbySide back = got.m_aSides[0];
		if (!back.m_sKey.IsEmpty() || back.m_sName != "~")
			faults.Insert("side");

		if (back.m_aGroups.Count() != 2)
		{
			faults.Insert("groupCount");
			return;
		}

		// The empty-callsign squad keeps its identity and exactly its own two seats.
		TBD_LobbyGroup first = back.m_aGroups[0];
		if (!first.m_sCallsign.IsEmpty() || first.m_aSlots.Count() != 2)
		{
			faults.Insert("group");
		}
		else
		{
			if (first.m_aSlots[0].m_sKey != "a.b" || !first.m_aSlots[0].m_sRole.IsEmpty()
				|| !first.m_aSlots[0].IsOpen() || !first.m_aSlots[0].m_sHolder.IsEmpty() || first.m_aSlots[0].m_bIsOwn)
				faults.Insert("slotOpen");

			// A slot key of `~` read back as empty would be a seat nobody can claim and a DEPLOY that
			// stays disabled for its holder.
			if (first.m_aSlots[1].m_sKey != "~" || first.m_aSlots[1].m_sRole != "SL"
				|| first.m_aSlots[1].m_sState != TBD_LobbyService.STATE_HELD || first.m_aSlots[1].m_sHolder != "Bob" || !first.m_aSlots[1].m_bIsOwn)
				faults.Insert("slotHeld");
		}

		TBD_LobbyGroup second = back.m_aGroups[1];
		if (second.m_sCallsign != "BRAVO" || second.m_aSlots.Count() != 1)
		{
			faults.Insert("group2");
		}
		else if (second.m_aSlots[0].m_sKey != "c" || !second.m_aSlots[0].IsDead())
		{
			faults.Insert("slotDead");
		}

		// Recount derives the reader's seat from the slots: DEPLOY lights for the holder of `~`.
		if (got.m_sOwnKey != "~")
			faults.Insert("ownKey");
	}

	//! Phase 2: parse a wire with a truncated `G` and a truncated `F`, each followed by rows, and append a fault when an orphan row lands under the squad above it.
	protected static void CheckOrphanRows(notnull array<string> faults)
	{
		array<string> bad = {};
		bad.Insert(TBD_WireCodec.Record(2, "F", "us", "US Army", string.Empty, string.Empty, string.Empty));
		bad.Insert(TBD_WireCodec.Record(1, "G", "ALPHA", string.Empty, string.Empty, string.Empty, string.Empty));
		bad.Insert(TBD_WireCodec.Record(5, "S", "k1", "SL", TBD_LobbyService.STATE_OPEN, TBD_WireCodec.Flag(false), string.Empty));

		// A truncated squad record, then its seat: the seat must not land in ALPHA.
		bad.Insert("G");
		bad.Insert(TBD_WireCodec.Record(5, "S", "k2", "RFL", TBD_LobbyService.STATE_OPEN, TBD_WireCodec.Flag(false), string.Empty));

		// A truncated side record, then a squad and a seat under it: all dropped.
		bad.Insert("F");
		bad.Insert(TBD_WireCodec.Record(1, "G", "GHOST", string.Empty, string.Empty, string.Empty, string.Empty));
		bad.Insert(TBD_WireCodec.Record(5, "S", "k3", "RFL", TBD_LobbyService.STATE_OPEN, TBD_WireCodec.Flag(false), string.Empty));

		TBD_LobbyRoster got = TBD_LobbyRosterWire.Parse(TBD_WireCodec.Join(bad, TBD_LobbyRosterWire.MAX_PAYLOAD_LINES, TBD_LobbyService.CH_LOBBY, TBD_LobbyRosterWire.CLIP_WARNING));

		if (got.m_aSides.Count() != 1 || got.m_aSides[0].m_aGroups.Count() != 1)
		{
			faults.Insert("orphanShape");
			return;
		}

		TBD_LobbyGroup only = got.m_aSides[0].m_aGroups[0];
		if (only.m_sCallsign != "ALPHA" || only.m_aSlots.Count() != 1 || only.m_aSlots[0].m_sKey != "k1")
			faults.Insert("orphanRows");
	}
}
