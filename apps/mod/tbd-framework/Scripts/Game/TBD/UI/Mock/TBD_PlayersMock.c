//! Briefing pass (2026-09-14) — the mock catalog behind the PLAYERS modal.
//!
//! The players_panel mockup: BLUFOR Defending 36 / 40, OPFOR Attacking 48 / 50, 4 spectators,
//! 6 unslotted, TOTAL 94. The mockup draws the first nine names of each lane; the rest are
//! generated so the counts, the scrollbars and the nav badge are real. Consumed only through
//! `TBD_PlayersCatalog.Get()`.
class TBD_PlayersMock
{
	//------------------------------------------------------------------------------------------------
	static TBD_PlayersCatalog Build()
	{
		TBD_PlayersCatalog c = new TBD_PlayersCatalog();
		c.m_mCapacity.Insert("BLUFOR", 40);
		c.m_mCapacity.Insert("OPFOR", 50);

		array<string> blufor = {"Miller, J.", "Davis, R.", "Stone, B.", "Ramos, C.", "Clark, H.", "Vance, D.", "Kowalski, P.", "Jones, T.", "Smith, K."};
		array<int> bluforPing = {18, 24, 22, 31, 19, 27, 35, 62, 29};
		Lane(c, "BLUFOR", blufor, bluforPing, 36, "BLU");

		array<string> opfor = {"Ales [XOF]", "Vikhr [L-13]", "ProrocK [L-13]", "Kolin [L-13]", "gHosT", "Azmir [FDX]", "Eben [LG]", "western_fog [TERA]", "teran [TERA]"};
		array<int> opforPing = {21, 32, 19, 25, 17, 28, 30, 38, 41};
		Lane(c, "OPFOR", opfor, opforPing, 48, "RED");

		c.m_aPlayers.Insert(new TBD_PlayerInfo("Caster_Broadcaster", "", 14, TBD_EPlayerState.SPECTATOR));
		c.m_aPlayers.Insert(new TBD_PlayerInfo("Referee_Admin", "", 18, TBD_EPlayerState.SPECTATOR, "ADMIN"));
		c.m_aPlayers.Insert(new TBD_PlayerInfo("TacticalReviewer", "", 34, TBD_EPlayerState.SPECTATOR));
		c.m_aPlayers.Insert(new TBD_PlayerInfo("ShadowObserver", "", 58, TBD_EPlayerState.SPECTATOR));

		c.m_aPlayers.Insert(new TBD_PlayerInfo("ReconRanger", "", 22, TBD_EPlayerState.UNSLOTTED));
		c.m_aPlayers.Insert(new TBD_PlayerInfo("Delta_Sniper", "", 29, TBD_EPlayerState.UNSLOTTED));
		c.m_aPlayers.Insert(new TBD_PlayerInfo("IronWolf", "", 44, TBD_EPlayerState.UNSLOTTED));
		c.m_aPlayers.Insert(new TBD_PlayerInfo("Nightjar", "", 51, TBD_EPlayerState.UNSLOTTED));
		c.m_aPlayers.Insert(new TBD_PlayerInfo("Quill_7", "", 36, TBD_EPlayerState.UNSLOTTED));
		c.m_aPlayers.Insert(new TBD_PlayerInfo("Vector_Actual", "", 27, TBD_EPlayerState.UNSLOTTED));
		return c;
	}

	//------------------------------------------------------------------------------------------------
	//! The named rows, then generated ones up to `slotted`.
	protected static void Lane(TBD_PlayersCatalog c, string faction, array<string> names, array<int> pings, int slotted, string stem)
	{
		int i;
		for (i = 0; i < slotted; i++)
		{
			string name;
			int ping;
			if (i < names.Count())
			{
				name = names[i];
				ping = pings[i];
			}
			else
			{
				name = string.Format("%1_%2", stem, i + 1);
				ping = 16 + ((i * 7) % 60);
			}

			c.m_aPlayers.Insert(new TBD_PlayerInfo(name, faction, ping, TBD_EPlayerState.SLOTTED));
		}
	}
}
