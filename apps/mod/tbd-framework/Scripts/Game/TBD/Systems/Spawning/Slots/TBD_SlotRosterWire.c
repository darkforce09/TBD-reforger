/**
 * @file TBD_SlotRosterWire.c
 * @brief The lobby's plain-text view of the slot roster.
 *
 * Role: one tab-separated line per mission slot with its holder and whether it can be taken.
 * Position: built from TBD_SlotClaimBook and the one-life ledger on TBD_SpawnManager.BuildSlotRoster;
 * read by TBD_LobbyService and TBD_LobbyData.
 * State: the set of authored values already reported as replaced (static, per process).
 * Invariants: every line has exactly six fields, `<slotKey>\t<faction>\t<group>\t<role>\t<state>\t<holderPlayerId>`;
 * state is OPEN, HELD or DEAD (a spent life, connected or departed, holder -1 when departed);
 * authored fields never carry a tab, carriage return or newline.
 */

//! Builds the slot roster wire of the spawn manager.
class TBD_SlotRosterWire
{
	protected static ref map<string, bool> s_mRosterRewrites; //!< authored values already reported as replaced; null until first rewrite

	//! One roster line per mission slot, in slot order.
	//! @return the lines; empty without mission slots
	static array<string> Build(notnull TBD_SpawnManager spawn)
	{
		map<int, ref TBD_MissionSlotStruct> playerSlots = spawn.GetSlots().GetPlayerSlots();
		array<string> roster = {};
		array<ref TBD_MissionSlotStruct> slots = TBD_MissionLoader.GetSlots();
		if (!slots)
			return roster;

		foreach (TBD_MissionSlotStruct slot : slots)
		{
			if (!slot)
				continue;

			int holder = -1;
			foreach (int playerId, TBD_MissionSlotStruct assigned : playerSlots)
			{
				if (assigned && assigned.Key() == slot.Key())
				{
					holder = playerId;
					break;
				}
			}

			string state = "OPEN";
			if (holder > 0)
			{
				if (spawn.IsPlayerDead(holder))
					state = "DEAD";
				else
					state = "HELD";
			}
			else if (spawn.GetSlots().IsSeatDeparted(slot.Key()))
			{
				state = "DEAD";
			}

			roster.Insert(string.Format("%1\t%2\t%3\t%4\t%5\t%6",
				RosterField(slot.Key()), RosterField(slot.faction),
				RosterField(slot.groupCallsign), RosterField(slot.role), state, holder));
		}
		return roster;
	}

	//! `value` with every tab, carriage return and newline replaced by a space. A rewrite is logged
	//! once per distinct value, so the author learns the mission has an unrenderable field.
	static string RosterField(string value)
	{
		if (!value.Contains("\t") && !value.Contains("\n") && !value.Contains("\r"))
			return value;

		string cleaned = value;
		cleaned.Replace("\t", " ");
		cleaned.Replace("\r", " ");
		cleaned.Replace("\n", " ");

		if (!s_mRosterRewrites)
			s_mRosterRewrites = new map<string, bool>();

		if (!s_mRosterRewrites.Contains(value))
		{
			s_mRosterRewrites.Set(value, true);
			PrintFormat("[TBD][Spawn] roster field contained a separator and was replaced: '%1' -> '%2'. Fix the mission; the schema permits this but the wire cannot carry it.",
				value, cleaned, level: LogLevel.WARNING);
		}

		return cleaned;
	}
}
