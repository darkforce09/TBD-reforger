/**
 * @file TBD_SafestartWatchdog.c
 * @brief Keeps repairing a safe start lift that did not verify, and keeps saying so until it does.
 *
 * Role: after a lift leaves bodies unrestored, repeats TBD_SafestartProtection.Restore every 5 s and
 * reports the failure: a banner and a chat warning once, ERROR each pass for the first minute, then
 * once a minute.  Position: owned by TBD_SafestartManager, which calls OnLiftFailed from Lift and
 * Reset on each new arm.  State: the running flag, the announced flag, the pass counter and one
 * call-queue poll; server only.
 * Invariants: repairs are never throttled, only the log volume; stops itself when nothing is owed
 * and stands down while the manager is armed again.
 */

//! Post-lift repair loop of the safe start.
class TBD_SafestartWatchdog : Managed
{
	protected static const int WATCHDOG_MS = 5000; //!< repair period (ms)
	protected static const int LOUD_WATCHDOG_PASSES = 12; //!< passes logged at ERROR each; after that, one line per this many passes

	protected TBD_SafestartManager m_Manager; //!< owning manager
	protected TBD_SafestartProtection m_Protection; //!< the shield whose restore is repeated
	protected bool m_bWatchdogRunning; //!< true while the repair poll runs
	protected bool m_bLiftFailureAnnounced; //!< true once this arm's failure banner went out
	protected int m_iWatchdogPasses; //!< repair passes since the poll started

	//! Bind the watchdog to its manager and shield.
	//! @param manager the owning safe start manager
	//! @param protection the manager's shield
	void TBD_SafestartWatchdog(TBD_SafestartManager manager, TBD_SafestartProtection protection)
	{
		m_Manager = manager;
		m_Protection = protection;
	}

	//! Stop the repair poll.
	void CancelCallbacks()
	{
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
			queue.Remove(TickWatchdog);
		m_bWatchdogRunning = false;
	}

	//! Re-arm the failure banner for a new arm.
	void Reset()
	{
		m_bLiftFailureAnnounced = false;
	}

	//! A lift left bodies unrestored: announce it and start the repair poll.
	//! @authority server
	void OnLiftFailed()
	{
		AnnounceLiftFailure();
		StartWatchdog();
	}

	//! Start the repair poll unless it already runs.
	//! @authority server
	protected void StartWatchdog()
	{
		if (m_bWatchdogRunning)
			return;

		m_bWatchdogRunning = true;
		m_iWatchdogPasses = 0;
		GetGame().GetCallqueue().CallLater(TickWatchdog, WATCHDOG_MS, true);
	}

	//! One repair pass: stands down if the manager is armed again, stops with a recovery banner once
	//! every body verifies, else reports the failure.
	//! @authority server
	protected void TickWatchdog()
	{
		if (m_Manager.IsArmed())
		{
			// Re-armed: the arm owns the bodies again.
			StopWatchdog();
			return;
		}

		m_iWatchdogPasses++;
		if (m_Protection.Restore() == 0)
		{
			StopWatchdog();
			TBD_Log.Banner(TBD_Log.CH_SAFESTART, "SAFESTART LIFT RECOVERED -- every body verified damage-ON", false);
			TBD_PlayerChat.Broadcast(TBD_Log.CH_SAFESTART, "[TBD] Safestart lift recovered -- damage is ON for everyone.");
			return;
		}

		AnnounceLiftFailure();
	}

	//! Stop the repair poll.
	//! @authority server
	protected void StopWatchdog()
	{
		m_bWatchdogRunning = false;
		GetGame().GetCallqueue().Remove(TickWatchdog);
	}

	//! Report unrestored bodies: a banner and a chat warning the first time per arm, then ERROR each
	//! pass for LOUD_WATCHDOG_PASSES passes, then every LOUD_WATCHDOG_PASSES passes. Bounded because
	//! a read-back that never agrees would otherwise bury the event log.
	//! @authority server
	protected void AnnounceLiftFailure()
	{
		string detail = string.Format("bodies=%1 -- THESE PLAYERS MAY BE INVULNERABLE", m_Protection.UnrestoredCount());

		if (!m_bLiftFailureAnnounced)
		{
			m_bLiftFailureAnnounced = true;
			TBD_Log.Banner(TBD_Log.CH_SAFESTART, "SAFESTART FAILED TO LIFT -- " + detail, true);
			TBD_PlayerChat.Broadcast(TBD_Log.CH_SAFESTART, "[TBD] !! SAFESTART FAILED TO LIFT for some players -- damage may still be OFF. Tell an admin NOW.");
			return;
		}

		if (m_iWatchdogPasses <= LOUD_WATCHDOG_PASSES)
		{
			TBD_Log.Error(TBD_Log.CH_SAFESTART, "still not lifted -- " + detail);
			return;
		}

		if (m_iWatchdogPasses % LOUD_WATCHDOG_PASSES != 0)
			return;

		string quieter = "still not lifted after ";
		quieter += m_iWatchdogPasses.ToString();
		quieter += " repair passes -- " + detail;
		quieter += ". If this never clears, suspect the damage read-back itself and check a body by hand.";
		TBD_Log.Error(TBD_Log.CH_SAFESTART, quieter);
	}
}
