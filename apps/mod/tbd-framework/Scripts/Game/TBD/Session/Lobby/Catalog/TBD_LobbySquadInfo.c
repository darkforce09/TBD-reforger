/**
 * @file TBD_LobbySquadInfo.c
 * @brief One squad card of the Lobby screen: callsign, vehicle and its seats.
 *
 * Role: the presentation of one squad and the seat factory.  Position: built by TBD_LobbyMock into
 * TBD_LobbyCatalog; read by TBD_LobbySquadCardComponent, TBD_LobbyRosterPanel and
 * TBD_KitInspectorPanel.
 * State: plain data.  Invariants: seat keys are `<callsign>:<index>` with 1-based indices in
 * insertion order; Filled counts every seat that is not OPEN.
 */

//! One squad: callsign chip, vehicle chip, `filled/total` and its seat rows.
class TBD_LobbySquadInfo
{
	string m_sCallsign; //!< e.g. `Alpha 2-1`
	string m_sVehicle; //!< e.g. `BMP-2`; empty = no chip
	ref array<ref TBD_LobbySlotInfo> m_aSlots; //!< seats in squad order

	//! Create an empty squad.
	void TBD_LobbySquadInfo(string callsign, string vehicle)
	{
		m_sCallsign = callsign;
		m_sVehicle = vehicle;
		m_aSlots = {};
	}

	//! Append a seat keyed `<callsign>:<index>`.
	//! @param weapons comma-separated weapon chips
	//! @param tags comma-separated role tags
	//! @param holder a holder name makes the seat HELD
	//! @return the new seat
	TBD_LobbySlotInfo AddSlot(string role, string kitKey, string weapons = "", string tags = "", string holder = "")
	{
		int index = m_aSlots.Count() + 1;
		string key = string.Format("%1:%2", m_sCallsign, index);
		TBD_LobbySlotInfo slot = new TBD_LobbySlotInfo(key, index, role, kitKey);
		if (!weapons.IsEmpty())
			weapons.Split(",", slot.m_aWeapons, true);
		if (!tags.IsEmpty())
			tags.Split(",", slot.m_aTags, true);
		if (!holder.IsEmpty())
		{
			slot.m_sHolder = holder;
			slot.m_sState = "HELD";
		}

		m_aSlots.Insert(slot);
		return slot;
	}

	//! @return seats that are not OPEN
	int Filled()
	{
		int filled;
		foreach (TBD_LobbySlotInfo slot : m_aSlots)
		{
			if (!slot.IsOpen())
				filled++;
		}

		return filled;
	}

	//! @return true when the viewer holds a seat in this squad
	bool HasOwn()
	{
		foreach (TBD_LobbySlotInfo slot : m_aSlots)
		{
			if (slot.m_bOwn)
				return true;
		}

		return false;
	}
}
