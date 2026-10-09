/**
 * @file TBD_SlotLoadoutSettle.c
 * @brief The spawn boundary: waits for every slot loadout pass and opens or refuses spawning.
 *
 * Role: tracks the TBD_LoadoutApplication of every slot body and assesses the lineup once all are
 * done.  Position: armed by TBD_SlotBodyMaterializer.MaterializeSlotBodies; read by the deploy path
 * (per body), by TBD_FrameworkManager (settle pending) and by the LOBBY stage gate.
 * State: the in-flight applications (CallLater holds no reference), the pending and refused flags
 * and the poll counter; server only.
 * Invariants: spawn opens only when every application is done and none is unplayable; a timeout
 * (40 x 250 ms) refuses, because an unfinished pass never audited its body; a shortfall (playable,
 * not what was authored) opens spawn and is reported to the console and the admin trail.
 */

//! Loadout settle of the slot lineup.
class TBD_SlotLoadoutSettle : Managed
{
	protected const int LOADOUT_SETTLE_TICK_MS = 250; //!< poll period (ms)
	protected const int LOADOUT_SETTLE_MAX_TICKS = 40; //!< polls before a timeout refuses spawn (10 s)

	protected TBD_SpawnManager m_Spawn; //!< owning manager
	protected ref array<ref TBD_LoadoutApplication> m_aLoadoutApps = {}; //!< in-flight and finished applications; done ones are pruned
	protected bool m_bLoadoutSettlePending; //!< true while the poll waits on applications
	protected bool m_bLoadoutDeliveryRefused; //!< true once a slot body is unplayable or the poll timed out
	protected int m_iLoadoutSettleTicks; //!< polls since the settle was armed

	//! Bind the settle to its manager.
	void TBD_SlotLoadoutSettle(TBD_SpawnManager spawn)
	{
		m_Spawn = spawn;
	}

	//! Stop the poll.
	void CancelCallbacks()
	{
		GetGame().GetCallqueue().Remove(TickLoadoutSettle);
	}

	//! True while the settle poll waits on loadout applications.
	bool IsPending()
	{
		return m_bLoadoutSettlePending;
	}

	//! True after the spawn boundary refused the lineup.
	bool IsRefused()
	{
		return m_bLoadoutDeliveryRefused;
	}

	//! Keep a strong reference to `app` until it is done.
	void Track(notnull TBD_LoadoutApplication app)
	{
		m_aLoadoutApps.Insert(app);
	}

	//! Start polling the tracked applications.
	//! @authority server
	void Arm()
	{
		m_bLoadoutSettlePending = true;
		m_iLoadoutSettleTicks = 0;
		Print(string.Format("[TBD][Slots] loadout settle armed -- waiting for %1 application(s) to finish before spawn opens",
			m_aLoadoutApps.Count()));
		GetGame().GetCallqueue().CallLater(TickLoadoutSettle, LOADOUT_SETTLE_TICK_MS, true);
	}

	//! One poll: wait while any application runs (up to the ceiling), then refuse on a timeout or
	//! an unplayable body, else open spawn and report every shortfall. Kicks the BRIEFING holder
	//! deploy when the round already entered BRIEFING.
	//! @authority server
	protected void TickLoadoutSettle()
	{
		if (!m_bLoadoutSettlePending)
		{
			GetGame().GetCallqueue().Remove(TickLoadoutSettle);
			return;
		}

		m_iLoadoutSettleTicks++;

		int pending = 0;
		foreach (TBD_LoadoutApplication app : m_aLoadoutApps)
		{
			if (app && !app.IsDone())
				pending++;
		}

		if (pending > 0 && m_iLoadoutSettleTicks < LOADOUT_SETTLE_MAX_TICKS)
			return;

		GetGame().GetCallqueue().Remove(TickLoadoutSettle);
		m_bLoadoutSettlePending = false;

		if (pending > 0)
		{
			m_bLoadoutDeliveryRefused = true;
			Print(string.Format("[TBD][Slots] loadout settle TIMED OUT -- %1 application(s) still in flight after %2 ms -- spawn REFUSED (they never finished, so nothing has assessed those bodies)",
				pending, m_iLoadoutSettleTicks * LOADOUT_SETTLE_TICK_MS), LogLevel.ERROR);
			return;
		}

		int unplayable = 0;
		string blockingSlots;
		int shortfall = 0;
		string shortfallSlots;
		foreach (TBD_LoadoutApplication app : m_aLoadoutApps)
		{
			if (!app)
				continue;
			if (app.HasBlockingFailure())
			{
				unplayable++;
				if (!blockingSlots.IsEmpty())
					blockingSlots += "; ";
				blockingSlots += string.Format("%1 [%2]", app.GetLabel(), app.BlockingSummary());
			}
			else if (app.HasShortfall())
			{
				shortfall++;
				if (!shortfallSlots.IsEmpty())
					shortfallSlots += "; ";
				shortfallSlots += string.Format("%1 [%2]", app.GetLabel(), app.ShortfallBrief());
			}
		}

		if (unplayable > 0)
		{
			m_bLoadoutDeliveryRefused = true;
			Print(string.Format("[TBD][Slots] loadout delivery REFUSED at spawn boundary -- %1 slot body(ies) are UNPLAYABLE -- LOBBY/deploy will not open: %2",
				unplayable, blockingSlots), LogLevel.ERROR);
			TBD_AdminAudit.Record(string.Format("LOADOUT: session REFUSED -- %1 slot(s) unplayable: %2",
				unplayable, blockingSlots), true);
			return;
		}

		m_Spawn.GetBodies().MarkMaterialized();
		Print(string.Format("[TBD][Slots] loadout settle complete -- %1 application(s), 0 unplayable, %2 with a shortfall -- spawn open",
			m_aLoadoutApps.Count(), shortfall));
		if (shortfall > 0)
		{
			Print(string.Format("[TBD][Slots] loadout SHORTFALL on %1 of %2 slot(s) -- the session IS open and these players are playable, but they are NOT carrying what the mission authored. Fix the mission's cargo or the kit: %3",
				shortfall, m_aLoadoutApps.Count(), shortfallSlots), LogLevel.WARNING);
			TBD_AdminAudit.Record(string.Format("LOADOUT: %1 slot(s) did not get the authored loadout (session opened anyway): %2",
				shortfall, shortfallSlots), true);
		}

		PruneDoneLoadoutApps();
		if (m_Spawn.GetStage() == TBD_EGameStage.BRIEFING)
			m_Spawn.GetDeployWaves().ScheduleDeployClaimedHolders();
	}

	//! Drop every finished application.
	void PruneDoneLoadoutApps()
	{
		for (int i = m_aLoadoutApps.Count() - 1; i >= 0; i--)
		{
			if (m_aLoadoutApps[i].IsDone())
				m_aLoadoutApps.Remove(i);
		}
	}

	//! The application dressing `body`, in flight or just finished, or null.
	TBD_LoadoutApplication FindLoadoutAppFor(IEntity body)
	{
		if (!body)
			return null;

		foreach (TBD_LoadoutApplication app : m_aLoadoutApps)
		{
			if (app && app.GetCharacter() == body)
				return app;
		}
		return null;
	}

	//! Cancel any pass still dressing a body that is being abandoned, so it stops spawning items and
	//! its log lines cannot be confused with the replacement's.
	void CancelLoadoutAppsFor(IEntity body)
	{
		if (!body)
			return;

		foreach (TBD_LoadoutApplication app : m_aLoadoutApps)
		{
			if (!app.IsDone() && app.GetCharacter() == body)
				app.Cancel("slot body superseded by a fresh spawn");
		}
		PruneDoneLoadoutApps();
	}

	//! Why LOBBY must not open (an unplayable lineup, or the settle still pending), logged; empty
	//! when it may.
	//! @authority server
	string LobbyRefusalReason()
	{
		if (m_bLoadoutDeliveryRefused)
		{
			Print("[TBD][Spawn] LOBBY REFUSED -- one or more slot bodies are UNPLAYABLE (see the loadout delivery REFUSED lines above); staying in LOADING", LogLevel.ERROR);
			return "LOBBY refused -- one or more slot bodies are unplayable (blocking loadout failure)";
		}

		if (m_bLoadoutSettlePending)
		{
			Print("[TBD][Spawn] LOBBY REFUSED -- loadout settle still pending; staying in LOADING", LogLevel.ERROR);
			return "LOBBY refused -- loadout settle still pending";
		}

		return string.Empty;
	}
}
