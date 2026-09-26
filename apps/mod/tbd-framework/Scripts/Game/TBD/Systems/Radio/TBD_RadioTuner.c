/**
 * @file TBD_RadioTuner.c
 * @brief Tunes a player's carried radios into their mission nets and verifies each tune.
 *
 * Role: drives the native radio chain from script (no partner mod): gadget manager ->
 * `GetGadgetsByType(RADIO | RADIO_BACKPACK)` -> `SCR_RadioComponent.GetRadioComponent` ->
 * `BaseRadioComponent.GetTransceiver` -> `BaseTransceiver.SetFrequency(kHz)` and `GetFrequency`
 * read-back.  Position: `TBD_RadioService.Build` calls `TunePlayer` on the server;
 * `TBD_RadioComponent` asks `IsBackboneAvailable`, `WorldFileName` and `FallbackSourceName`.
 * State: none.  Invariants: a tune counts only when the same transceiver reads the frequency back;
 * a world without a `RadioManagerEntity` still tunes, from `FallbackChannelTable` (the player's
 * `radioPlan` nets, else a default pair only when the mission has no plan), and the report says
 * so; the net list is delivered and shown whatever the tune outcome.
 */

//! Outcome of one tune attempt, for the log and the player's tune line.
class TBD_RadioTuneReport
{
	TBD_ERadioTuneResult m_eResult; //!< the outcome
	int m_iRequested; //!< nets the attempt tried to place
	int m_iTuned; //!< nets whose frequency read back correct
	int m_iRadios; //!< radios found on the player
	string m_sDetail; //!< free-text detail; may be empty

	//! @return the outcome's enum name, as it crosses the wire
	string ResultName()
	{
		return typename.EnumToString(TBD_ERadioTuneResult, m_eResult);
	}
}

//! One radio the player carries and how many of its transceivers are still free.
class TBD_RadioSet
{
	BaseRadioComponent m_Radio; //!< the radio
	bool m_bLongRange; //!< true for RADIO_BACKPACK (long range), false for a handheld
	int m_iNextFree; //!< index of the next unassigned transceiver; default 0
	int m_iCount; //!< cached `TransceiversCount()`
}

//! The frequencies `TunePlayer` sets: the caller's nets, or the default pair on a world without a
//! backbone and a mission without a plan.
class TBD_RadioFallbackTable
{
	ref array<int> m_aFreqKHz; //!< frequency in kHz per channel
	ref array<int> m_aLongRange; //!< 1 long range, 0 handheld, per channel
	string m_sSource; //!< `radioPlan` or `defaults`, named in the boot warning; empty when neither
}

//! Server-side radio tuning.
//! @authority server
class TBD_RadioTuner
{
	static const int FALLBACK_DEFAULT_SHORT_KHZ = 42000; //!< handheld default channel in kHz (42.000 MHz)
	static const int FALLBACK_DEFAULT_LONG_KHZ = 41000; //!< long-range default channel in kHz (41.000 MHz)

	//! @return the running world's `RadioManagerEntity`, or null when the world places none
	static RadioManagerEntity GetBackbone()
	{
		ChimeraWorld world = ChimeraWorld.CastFrom(GetGame().GetWorld());
		if (!world)
			return null;

		return world.GetRadioManager();
	}

	//! @return true when the world has a radio backbone
	static bool IsBackboneAvailable()
	{
		return GetBackbone() != null;
	}

	//! @return the world file the running mission header names, or `worlds/TBD_Dev_POC.ent` when
	//! the header has none; named in the missing-backbone warning
	static string WorldFileName()
	{
		MissionHeader header = GetGame().GetMissionHeader();
		if (!header)
			return "worlds/TBD_Dev_POC.ent";

		string path = header.GetWorldPath();
		if (path.IsEmpty())
			return "worlds/TBD_Dev_POC.ent";

		return path;
	}

	//! @return `radioPlan` when the loaded mission has accepted nets, else `defaults`: the table the
	//! missing-backbone path uses
	static string FallbackSourceName()
	{
		if (TBD_MissionLoader.IsValid() && TBD_RadioPlan.GetTotalNetCount() > 0)
			return "radioPlan";

		return "defaults";
	}

	//! The channels to tune: a copy of the caller's nets when there are any; otherwise, only on a
	//! world without a backbone, the default pair, or nothing when the mission has a plan (another
	//! side's frequencies are never borrowed).
	//! @param freqKHz the player's net frequencies in kHz
	//! @param longRange the player's long-range flags; a missing flag reads 0
	//! @param backboneMissing true when the world has no `RadioManagerEntity`
	//! @return the table, never null
	static TBD_RadioFallbackTable FallbackChannelTable(notnull array<int> freqKHz, notnull array<int> longRange, bool backboneMissing)
	{
		TBD_RadioFallbackTable table = new TBD_RadioFallbackTable();
		table.m_aFreqKHz = {};
		table.m_aLongRange = {};

		if (!freqKHz.IsEmpty())
		{
			int n = freqKHz.Count();
			int nRange = longRange.Count();
			for (int i = 0; i < n; i++)
			{
				table.m_aFreqKHz.Insert(freqKHz[i]);
				int flag = 0;
				if (i < nRange)
					flag = longRange[i];

				table.m_aLongRange.Insert(flag);
			}

			table.m_sSource = "radioPlan";
			return table;
		}

		if (backboneMissing && TBD_MissionLoader.IsValid() && TBD_RadioPlan.GetTotalNetCount() > 0)
		{
			// A plan without nets for this player: an empty table, so `TunePlayer` reports NO_NETS.
			table.m_sSource = "radioPlan";
			return table;
		}

		if (backboneMissing)
		{
			table.m_aFreqKHz.Insert(FALLBACK_DEFAULT_SHORT_KHZ);
			table.m_aLongRange.Insert(0);
			table.m_aFreqKHz.Insert(FALLBACK_DEFAULT_LONG_KHZ);
			table.m_aLongRange.Insert(1);
			table.m_sSource = "defaults";
			return table;
		}

		return table;
	}

	//! Put one player's carried radios on their nets, one transceiver per net, powering each tuned
	//! radio, and count only frequencies that read back correct.
	//! @param playerId the player
	//! @param freqKHz frequency in kHz per net, in the order `TBD_RadioService` resolved them
	//! @param longRange 1 long range, 0 handheld, per net (int because the same array is an RPC
	//! parameter)
	//! @return the report, never null
	//! @authority server
	static TBD_RadioTuneReport TunePlayer(int playerId, notnull array<int> freqKHz, notnull array<int> longRange)
	{
		TBD_RadioTuneReport report = new TBD_RadioTuneReport();

		bool backboneMissing = !IsBackboneAvailable();
		TBD_RadioFallbackTable table = FallbackChannelTable(freqKHz, longRange, backboneMissing);
		array<int> useFreq = table.m_aFreqKHz;
		array<int> useRange = table.m_aLongRange;
		report.m_iRequested = useFreq.Count();

		if (useFreq.IsEmpty())
		{
			report.m_eResult = TBD_ERadioTuneResult.NO_NETS;
			return report;
		}

		PlayerManager players = GetGame().GetPlayerManager();
		if (!players)
		{
			report.m_eResult = TBD_ERadioTuneResult.NO_BODY;
			return report;
		}

		IEntity body = players.GetPlayerControlledEntity(playerId);
		if (!body)
		{
			report.m_eResult = TBD_ERadioTuneResult.NO_BODY;
			return report;
		}

		SCR_GadgetManagerComponent gadgets = SCR_GadgetManagerComponent.GetGadgetManager(body);
		if (!gadgets)
		{
			report.m_eResult = TBD_ERadioTuneResult.NO_GADGET_MANAGER;
			return report;
		}

		array<ref TBD_RadioSet> sets = CollectRadios(gadgets);
		report.m_iRadios = sets.Count();
		if (sets.IsEmpty())
		{
			report.m_eResult = TBD_ERadioTuneResult.NO_RADIO;
			report.m_sDetail = "player carries no radio";
			return report;
		}

		int mismatches = 0;
		int noRoom = 0;

		for (int i = 0; i < useFreq.Count(); i++)
		{
			TBD_RadioSet radioSet = PickRadio(sets, useRange[i] == 1);
			if (!radioSet)
			{
				noRoom++;
				continue;
			}

			BaseTransceiver transceiver = radioSet.m_Radio.GetTransceiver(radioSet.m_iNextFree);
			radioSet.m_iNextFree = radioSet.m_iNextFree + 1;
			if (!transceiver)
			{
				noRoom++;
				continue;
			}

			int wanted = Constrain(transceiver, useFreq[i]);

			// The server-side setter; `SetTransceiverFrequency` is the client-origin variant.
			transceiver.SetFrequency(wanted);

			// Only a read-back off the same transceiver counts as tuned.
			if (transceiver.GetFrequency() != wanted)
			{
				mismatches++;
				continue;
			}

			// Only a powered radio carries traffic.
			if (!radioSet.m_Radio.IsPowered())
				radioSet.m_Radio.SetPower(true);

			report.m_iTuned = report.m_iTuned + 1;
		}

		if (report.m_iTuned > 0)
		{
			report.m_eResult = TBD_ERadioTuneResult.TUNED;
			if (mismatches > 0 || noRoom > 0)
			{
				report.m_sDetail = string.Format("%1 read-back mismatch, %2 with no free transceiver",
					mismatches, noRoom);
			}
		}
		else if (mismatches > 0)
		{
			report.m_eResult = TBD_ERadioTuneResult.READBACK_MISMATCH;
			report.m_sDetail = string.Format("%1 transceiver(s) did not hold the frequency we set", mismatches);
		}
		else
		{
			report.m_eResult = TBD_ERadioTuneResult.NO_TRANSCEIVER;
			report.m_sDetail = "no free transceiver on any carried radio";
		}

		if (backboneMissing)
		{
			string note = string.Format("script-side fallback (%1); no RadioManagerEntity", table.m_sSource);
			if (report.m_sDetail.IsEmpty())
				report.m_sDetail = note;
			else
				report.m_sDetail = report.m_sDetail + "; " + note;
		}

		return report;
	}

	//! @return every radio the player carries with at least one transceiver, handhelds first so a
	//! handheld net lands on the radio everybody has
	protected static array<ref TBD_RadioSet> CollectRadios(notnull SCR_GadgetManagerComponent gadgets)
	{
		array<ref TBD_RadioSet> sets = {};

		AppendRadios(sets, gadgets.GetGadgetsByType(EGadgetType.RADIO), false);
		AppendRadios(sets, gadgets.GetGadgetsByType(EGadgetType.RADIO_BACKPACK), true);

		return sets;
	}

	//! Append each found radio gadget that has transceivers.
	//! @param sets the list to append to
	//! @param found the gadgets of one type; may be null
	//! @param longRange true for backpack radios
	protected static void AppendRadios(notnull array<ref TBD_RadioSet> sets, array<SCR_GadgetComponent> found, bool longRange)
	{
		if (!found)
			return;

		foreach (SCR_GadgetComponent gadget : found)
		{
			SCR_RadioComponent radioGadget = SCR_RadioComponent.Cast(gadget);
			if (!radioGadget)
				continue;

			BaseRadioComponent radio = radioGadget.GetRadioComponent();
			if (!radio)
				continue;

			int count = radio.TransceiversCount();
			if (count <= 0)
				continue;

			TBD_RadioSet radioSet = new TBD_RadioSet();
			radioSet.m_Radio = radio;
			radioSet.m_bLongRange = longRange;
			radioSet.m_iCount = count;
			radioSet.m_iNextFree = 0;
			sets.Insert(radioSet);
		}
	}

	//! Pick the radio for a net: the first of the wanted class with a free transceiver, else any
	//! radio with one, so a long-range net on a handheld-only player is not dropped.
	//! @param wantLongRange true for a long-range net
	//! @return the radio, or null when every transceiver is taken
	protected static TBD_RadioSet PickRadio(notnull array<ref TBD_RadioSet> sets, bool wantLongRange)
	{
		foreach (TBD_RadioSet radioSet : sets)
		{
			if (radioSet.m_bLongRange != wantLongRange)
				continue;

			if (radioSet.m_iNextFree < radioSet.m_iCount)
				return radioSet;
		}

		foreach (TBD_RadioSet fallback : sets)
		{
			if (fallback.m_iNextFree < fallback.m_iCount)
				return fallback;
		}

		return null;
	}

	//! The nearest frequency the transceiver can hold: snapped to `GetFrequencyResolution`, then
	//! clamped to the band, whose ends are ordered by value (the engine's min and max docs are
	//! transposed). A band that reads 0..0 does not clamp.
	//! @param freqKHz the wanted frequency in kHz
	//! @return the frequency to set in kHz
	protected static int Constrain(notnull BaseTransceiver transceiver, int freqKHz)
	{
		int step = transceiver.GetFrequencyResolution();
		int value = freqKHz;

		if (step > 0)
		{
			int rest = value % step;
			if (rest != 0)
			{
				value = value - rest;
				if (rest * 2 >= step)
					value = value + step;
			}
		}

		int a = transceiver.GetMinFrequency();
		int b = transceiver.GetMaxFrequency();
		int low = a;
		int high = b;
		if (b < a)
		{
			low = b;
			high = a;
		}

		// A band of 0..0 means the transceiver did not answer; do not clamp everything to zero.
		if (high <= 0)
			return value;

		if (value < low)
			return low;

		if (value > high)
			return high;

		return value;
	}
}
