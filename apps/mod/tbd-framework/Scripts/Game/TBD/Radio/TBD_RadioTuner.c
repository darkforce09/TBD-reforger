//! T-181.40 — the ENGINE half: actually putting a player's radio on the mission's frequency.
//!
//! ══ WHAT IS AND IS NOT REACHABLE FROM SCRIPT — MEASURED, NOT REMEMBERED ═════════════════════
//! `TBD_MOD_DESIGN.md` §6 says radio is wanted but NOT via CRF's route, because CRF depends on the
//! external CVON workshop mod and TBD must not. The open question that made that a risk was
//! whether Reforger's radio is drivable from script at all without a partner mod. It is. The whole
//! chain is `proto external` — native, script-callable, and compile-proved on this lane with a
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
//! ══ THE ONE THING THAT IS NOT REACHABLE, AND IT IS NOT AN API ══════════════════════════════
//! **The world has no `RadioManagerEntity`, and without it the engine supports NO radio at all.**
//! Measured on every boot of `Missions/TBD_Dev_POC.conf`, emitted by the engine itself the first
//! time a `BaseRadioComponent` is created (a transmitter tower in Eden):
//!
//!     DEFAULT (W): World doesn't contain RadioManagerEntity to support any BaseRadioComponent.
//!
//! `worlds/TBD_Dev_POC.ent` is a 62-byte bare `SubScene` of vanilla `Eden.ent` and places nothing
//! of its own, so there is nothing in the TBD world to host the radio backbone. This is the same
//! CLASS of blocker as `resourceDatabase.rdb` gating the five menu presets: not a script problem,
//! not fixable from the fast lane, and settled by one Workbench pass. `ChimeraWorld.GetRadioManager()`
//! is the runtime question this file asks on every boot so the answer is a FACT IN THE LOG rather
//! than an assumption in a comment — see `TBD_RadioComponent.ReportBackbone()`.
//!
//! ══ THE RULE THIS FILE EXISTS TO ENFORCE ═══════════════════════════════════════════════════
//! **Never report a tune that did not happen.** Every tune is verified by reading the frequency
//! back off the same transceiver and comparing. A log line saying a player is on ALPHA while no
//! radio changed is worse than no radio feature at all, because it would be believed — and this
//! program has repeatedly been bitten by things that looked like they worked. If the backbone is
//! absent, if the player carries no radio, or if the read-back disagrees, the outcome says so and
//! the net list is still DELIVERED and DISPLAYED. Assignment and display do not depend on any of
//! this; only the tuning does.

//! What happened when we tried to put one player on their nets. Ordered roughly worst to best so
//! a reader can tell a blocker from a nuance at a glance.
enum TBD_ERadioTuneResult
{
	//! The world has no `RadioManagerEntity`. Nothing radio-related can work; not our bug to fix
	//! from script. This is the CURRENT state of `TBD_Dev_POC`.
	NO_BACKBONE,
	//! The player has no controlled entity yet (lobby, dead, mid-possess). Ordinary, not an error.
	NO_BODY,
	//! The body has no gadget manager — it is not a character, or not a fully built one.
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

class TBD_RadioTuner
{
	//------------------------------------------------------------------------------------------------
	//! The world's radio backbone, or null when this world has none.
	//!
	//! `ChimeraWorld.GetRadioManager()` is `proto external` on the world the game is actually
	//! running (compile-proved here; the fabricated `GetRadioManagerTbdNotReal` fails). A null
	//! answer is not an API problem — it is the world file not placing the entity.
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
	//! @authority server — put one player on their nets, and PROVE it or say it did not happen.
	//!
	//! `freqKHz` and `longRange` are parallel: element i of each describes net i, in the order
	//! `TBD_RadioService` resolved them for this player's side. Never returns null.
	static TBD_RadioTuneReport TunePlayer(int playerId, notnull array<int> freqKHz, notnull array<bool> longRange)
	{
		TBD_RadioTuneReport report = new TBD_RadioTuneReport();
		report.m_iRequested = freqKHz.Count();

		if (freqKHz.IsEmpty())
		{
			report.m_eResult = TBD_ERadioTuneResult.NO_NETS;
			return report;
		}

		if (!IsBackboneAvailable())
		{
			report.m_eResult = TBD_ERadioTuneResult.NO_BACKBONE;
			report.m_sDetail = "world has no RadioManagerEntity";
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

		for (int i = 0; i < freqKHz.Count(); i++)
		{
			TBD_RadioSet radioSet = PickRadio(sets, longRange[i]);
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

			int wanted = Constrain(transceiver, freqKHz[i]);

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

			return report;
		}

		if (mismatches > 0)
		{
			report.m_eResult = TBD_ERadioTuneResult.READBACK_MISMATCH;
			report.m_sDetail = string.Format("%1 transceiver(s) did not hold the frequency we set", mismatches);
			return report;
		}

		report.m_eResult = TBD_ERadioTuneResult.NO_TRANSCEIVER;
		report.m_sDetail = "no free transceiver on any carried radio";
		return report;
	}

	//------------------------------------------------------------------------------------------------
	//! Every radio the player is carrying, handhelds first so a `range: any` net lands on the radio
	//! everybody has rather than on a backpack only the RTO carries.
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
	//! This is where `net.range` stops being a label and becomes hardware: a `long` net wants the
	//! backpack set, a `short` net wants a handheld, and `any` takes whatever is free. The
	//! preference is a PREFERENCE — a long-range net on a player who carries only a handheld goes
	//! into the handheld rather than being dropped, because a squad that can hear command badly is
	//! better off than one that cannot hear it at all. Enfusion has no ternary operator, so the two
	//! passes are written out.
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
	//!     of by name — the numbers are trusted, the doc strings are not.
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
