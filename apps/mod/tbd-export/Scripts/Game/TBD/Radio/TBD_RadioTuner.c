//! T-181.40 - the ENGINE half: actually putting a player's radio on the mission's frequency.
//!
//! == WHAT IS AND IS NOT REACHABLE FROM SCRIPT - MEASURED, NOT REMEMBERED =====================
//! `TBD_MOD_DESIGN.md` S6 says radio is wanted but NOT via CRF's route, because CRF depends on the
//! external CVON workshop mod and TBD must not. The open question that made that a risk was
//! whether Reforger's radio is drivable from script at all without a partner mod. It is. The whole
//! chain is `proto external` - native, script-callable, and compile-proved on this lane with a
//! failing negative control (a fabricated `SetFrequencyTbdDoesNotExist` and a fabricated
//! `EGadgetType.RADIO_TBD_DOES_NOT_EXIST` both error, so the real ones passing means something):
//!
//!   SCR_GadgetManagerComponent.GetGadgetManager(body)
//!     -> GetGadgetsByType(EGadgetType.RADIO | EGadgetType.RADIO_BACKPACK)   the player's radios
//!     -> SCR_RadioComponent.GetRadioComponent()                             -> BaseRadioComponent
//!     -> BaseRadioComponent.TransceiversCount() / GetTransceiver(i)         -> BaseTransceiver
//!     -> BaseTransceiver.SetFrequency(int kHz) / GetFrequency()             tune + READ BACK
//!     -> BaseRadioComponent.SetEncryptionKey(string) / IsPowered() / SetPower(bool)
//!
//! `BaseTransceiver.SetFrequency` is documented "Supports proxies and server" and takes kHz;
//! `BaseRadioComponent.SetTransceiverFrequency` is the client-origin variant ("and sync with
//! server"). The server-authoritative path used here is the former.
//!
//! == T-941.7 - SCRIPT FALLBACK WHEN THE BACKBONE IS ABSENT ==================================
//! The engine still emits this on every boot of `Missions/TBD_Dev_POC.conf` the first time a
//! `BaseRadioComponent` is created (a transmitter tower in Eden):
//!
//!     DEFAULT (W): World doesn't contain RadioManagerEntity to support any BaseRadioComponent.
//!
//! `worlds/TBD_Dev_POC.ent` still does not place a `RadioManagerEntity` - operator deferred that
//! world edit 2026-09-04. `ChimeraWorld.GetRadioManager()` remains the runtime question, but a
//! null answer is no longer a refuse-to-tune. `FallbackChannelTable` supplies the mission
//! `radioPlan` frequencies when the caller already resolved them, else a script-side default
//! pair, and `TunePlayer` still drives `SetFrequency` + read-back. The boot warning names the
//! world, the entity to add, and which table is in use. Placing `RadioManagerEntity` stays on
//! the operator checklist; it is not this slice.
//!
//! == THE RULE THIS FILE EXISTS TO ENFORCE ===================================================
//! **Never report a tune that did not happen.** Every tune is verified by reading the frequency
//! back off the same transceiver and comparing. A log line saying a player is on ALPHA while no
//! radio changed is worse than no radio feature at all, because it would be believed - and this
//! program has repeatedly been bitten by things that looked like they worked. If the player
//! carries no radio, or if the read-back disagrees, the outcome says so and the net list is
//! still DELIVERED and DISPLAYED. Assignment and display do not depend on any of this; only
//! the tuning does.

//! What happened when we tried to put one player on their nets. Ordered roughly worst to best so
//! a reader can tell a blocker from a nuance at a glance.
enum TBD_ERadioTuneResult
{
	//! World has no `RadioManagerEntity`. T-941.7 no longer refuses to tune on this path -
	//! `FallbackChannelTable` is used instead. Kept so the wire/client contract stays stable.
	NO_BACKBONE,
	//! The player has no controlled entity yet (lobby, dead, mid-possess). Ordinary, not an error.
	NO_BODY,
	//! The body has no gadget manager - it is not a character, or not a fully built one.
	NO_GADGET_MANAGER,
	//! The player carries no radio. Their kit simply has none; they still SEE their nets.
	NO_RADIO,
	//! A radio with zero transceivers, or every transceiver already used by an earlier net.
	NO_TRANSCEIVER,
	//! We asked, and the read-back disagreed. Treated as a FAILURE, never rounded up to success.
	READBACK_MISMATCH,
	//! Nothing to do: this player's side authored no nets.
	NO_NETS,
	//! At least one net is verifiably tuned into a real transceiver.
	TUNED
}

//! Server-side outcome of one tune attempt, for logging and for the honest text the player reads.
class TBD_RadioTuneReport
{
	TBD_ERadioTuneResult m_eResult;
	int m_iRequested;   //!< Nets we tried to place.
	int m_iTuned;       //!< Nets whose frequency READ BACK correct.
	int m_iRadios;      //!< Radios found on the player.
	string m_sDetail;   //!< Human-readable nuance; may be empty.

	//------------------------------------------------------------------------------------------------
	string ResultName()
	{
		return typename.EnumToString(TBD_ERadioTuneResult, m_eResult);
	}
}

//! One radio the player is carrying, plus how many of its transceivers are still free.
class TBD_RadioSet
{
	BaseRadioComponent m_Radio;
	bool m_bLongRange;   //!< True for RADIO_BACKPACK (long-range), false for a handheld.
	int m_iNextFree;     //!< Index of the next unassigned transceiver.
	int m_iCount;        //!< `TransceiversCount()`, cached.
}

//! T-941.7 - frequencies `TunePlayer` will actually set when the world has no RadioManagerEntity
//! (and the copy-through of the caller's arrays when it does). `m_sSource` is `radioPlan` or
//! `defaults` so the boot warning can name the table in use.
class TBD_RadioFallbackTable
{
	ref array<int> m_aFreqKHz;
	ref array<int> m_aLongRange;
	string m_sSource;
}

class TBD_RadioTuner
{
	//! Handheld default when the mission authored no radioPlan. 42.000 MHz, schema band 30..512.
	static const int FALLBACK_DEFAULT_SHORT_KHZ = 42000;
	//! Long-range default pair. 41.000 MHz - same band as golden `net:cmd`.
	static const int FALLBACK_DEFAULT_LONG_KHZ = 41000;

	//------------------------------------------------------------------------------------------------
	//! The world's radio backbone, or null when this world has none.
	//!
	//! `ChimeraWorld.GetRadioManager()` is `proto external` on the world the game is actually
	//! running (compile-proved here; the fabricated `GetRadioManagerTbdNotReal` fails). A null
	//! answer is not an API problem - it is the world file not placing the entity.
	static RadioManagerEntity GetBackbone()
	{
		ChimeraWorld world = ChimeraWorld.CastFrom(GetGame().GetWorld());
		if (!world)
			return null;

		return world.GetRadioManager();
	}

	//------------------------------------------------------------------------------------------------
	//! True when the engine can support `BaseRadioComponent` on this world at all.
	static bool IsBackboneAvailable()
	{
		return GetBackbone() != null;
	}

	//------------------------------------------------------------------------------------------------
	//! World file the running mission header names, or `worlds/TBD_Dev_POC.ent` when the header
	//! has not answered. Named in the once-per-boot warning so the operator knows WHICH world to
	//! edit.
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

	//------------------------------------------------------------------------------------------------
	//! Which script-side table the missing-backbone path will use on this boot: `radioPlan` when
	//! the loaded mission has accepted nets, else `defaults`.
	static string FallbackSourceName()
	{
		if (TBD_MissionLoader.IsValid() && TBD_RadioPlan.GetTotalNetCount() > 0)
			return "radioPlan";

		return "defaults";
	}

	//------------------------------------------------------------------------------------------------
	//! Script-side channel table. Caller-resolved `radioPlan` frequencies win when present;
	//! otherwise, and only when the backbone is missing, the default pair. Never returns null.
	//!
	//! Renaming this function is the T-941.7 perturbation: `TunePlayer` calls it by this name.
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
			// Plan exists but this player was given no nets. Do not invent defaults and do not
			// leak another side's frequencies - `TunePlayer` returns NO_NETS on the empty table.
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

	//------------------------------------------------------------------------------------------------
	//! @authority server - put one player on their nets, and PROVE it or say it did not happen.
	//!
	//! `freqKHz` and `longRange` are parallel: element i of each describes net i, in the order
	//! `TBD_RadioService` resolved them for this player's side. Never returns null.
	//!
	//! `longRange` is `array<int>` carrying 0/1 rather than `array<bool>`, because these same two
	//! arrays are the RPC parameters in `TBD_RadioController`: `array<int>` and `array<string>` are
	//! the only array element types that appear in EITHER oracle's replicated methods (8 and 4 uses
	//! in CRF, 1 in the carved vanilla source), and `array<bool>` appears in neither. It compiles,
	//! but compiling is not the same as crossing the wire, and this program has been bitten by that
	//! distinction repeatedly. The proven type costs nothing here.
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

			// The authoritative setter. Documented "Supports proxies and server"; the sibling
			// `BaseRadioComponent.SetTransceiverFrequency` is the client-origin variant that syncs
			// UP to the server, which is not the direction wanted here.
			transceiver.SetFrequency(wanted);

			// THE HONESTY GATE. An unverified `SetFrequency` would let this file log a player onto
			// a net while nothing changed. Read it back off the same object and believe only that.
			if (transceiver.GetFrequency() != wanted)
			{
				mismatches++;
				continue;
			}

			// Powered radios only actually carry traffic. Turning it on is part of "the player is
			// on this net"; leaving it off would be another way to look tuned and not be.
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

	//------------------------------------------------------------------------------------------------
	//! Every radio the player is carrying, handhelds first so a `range: short` net (and the
	//! flag-0 path in general) lands on the radio everybody has rather than on a backpack only
	//! the RTO carries. T-292 retired schema `any` - it was never a third hardware class.
	protected static array<ref TBD_RadioSet> CollectRadios(notnull SCR_GadgetManagerComponent gadgets)
	{
		array<ref TBD_RadioSet> sets = {};

		AppendRadios(sets, gadgets.GetGadgetsByType(EGadgetType.RADIO), false);
		AppendRadios(sets, gadgets.GetGadgetsByType(EGadgetType.RADIO_BACKPACK), true);

		return sets;
	}

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	//! The radio a net of this range class should go into, or null when every transceiver is spoken
	//! for.
	//!
	//! This is where `net.range` stops being a label and becomes hardware: a `long` net (flag 1)
	//! wants the backpack set; a `short` net (flag 0) wants a handheld. Enfusion has only those
	//! two gadget classes - there is no third `any` hardware path (T-292 narrowed the schema).
	//! The preference is a PREFERENCE - a long-range net on a player who carries only a handheld
	//! goes into the handheld rather than being dropped, because a squad that can hear command
	//! badly is better off than one that cannot hear it at all. Enfusion has no ternary operator,
	//! so the two passes are written out.
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

	//------------------------------------------------------------------------------------------------
	//! The nearest frequency this transceiver can actually hold.
	//!
	//! Two corrections, both from the transceiver itself rather than from an assumption about what
	//! Reforger radios do:
	//!   * SNAP to `GetFrequencyResolution()`. A mission may author `42.5` MHz on a radio whose
	//!     step is 25 kHz; setting an unrepresentable value is how a read-back check would
	//!     otherwise fail for a reason that is not a bug.
	//!   * CLAMP to the tunable band. BI's own doc comments for `GetMinFrequency` / `GetMaxFrequency`
	//!     are transposed (each describes the other), so the two are ordered here by VALUE instead
	//!     of by name - the numbers are trusted, the doc strings are not.
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
