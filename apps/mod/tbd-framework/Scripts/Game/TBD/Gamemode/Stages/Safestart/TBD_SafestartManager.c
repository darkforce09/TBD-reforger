/**
 * @file TBD_SafestartManager.c
 * @brief The game mode component that keeps everyone unhurt from LOBBY until the round goes LIVE.
 *
 * Role: arms the safe start shield on LOBBY, BRIEFING and SAFE_START, runs the replicated countdown
 * on SAFE_START, and lifts the shield on any other stage.  Position: a component on
 * TBD_GameMode.et (frozen class name); TBD_FrameworkManager.SetStage calls OnStageChanged on every
 * transition, TBD_AdminService drives StatusLine, GoLive and AdminSetSeconds; the per-body work
 * lives in TBD_SafestartProtection and the post-lift repair in TBD_SafestartWatchdog.
 * State: the replicated countdown, the armed flag, the configured length, the last countdown the
 * local UI showed, and the two helpers; every mutation runs on the server.
 * Invariants: the lift clears the armed flag before touching any body, so a leaked handler is
 * inert; each body gets back the damage value it had, never `true` by default; the countdown runs
 * on SAFE_START only; the shield covers characters, cannot holster a weapon, and leaves melee,
 * falls and vehicle impacts to the damage switch alone.
 */

//! Editor class of TBD_SafestartManager.
[ComponentEditorProps(category: "TBD/Framework", description: "TBD safestart -- damage-off warmup between BRIEFING and LIVE.")]
class TBD_SafestartManagerClass : SCR_BaseGameModeComponentClass {}

//! Safe start of a framework world: the shield, the countdown and the lift.
class TBD_SafestartManager : SCR_BaseGameModeComponent
{
	static const int DEFAULT_COUNTDOWN_SECONDS = 300; //!< countdown length when the mission sets none (s)
	static const int MIN_COUNTDOWN_SECONDS = 5; //!< shortest accepted countdown (s)
	static const int MAX_COUNTDOWN_SECONDS = 3600; //!< longest accepted countdown (s)
	static const int NOT_RUNNING = -1; //!< countdown value while off; negative so an unreplicated 0 never reads as "about to go"

	//! @replicated m_iSecondsRemaining
	[RplProp(onRplName: "OnCountdownReplicated")]
	protected int m_iSecondsRemaining = -1; //!< countdown seconds, NOT_RUNNING when off; server-owned

	protected bool m_bArmed; //!< the suppression flag every handler reads; clearing it disarms everything at once
	protected int m_iConfiguredSeconds = 300; //!< countdown length for the next arm (s)
	protected int m_iLocalLastSeen = -1; //!< last countdown this machine's UI acted on, so a repeated value re-pops nothing

	protected ref TBD_SafestartProtection m_Protection; //!< per-body shield
	protected ref TBD_SafestartWatchdog m_Watchdog; //!< post-lift repair

	//! Create the helpers.
	void TBD_SafestartManager(IEntityComponentSource src, IEntity ent, IEntity parent)
	{
		m_Protection = new TBD_SafestartProtection(this);
		m_Watchdog = new TBD_SafestartWatchdog(this, m_Protection);
	}

	//! The safe start on the loaded world, resolved off the live game mode on every call, because
	//! SetStage refuses SAFE_START on its answer and a static would outlive its world.
	//! @return the component, or null when the world has none
	static TBD_SafestartManager GetInstance()
	{
		SCR_BaseGameMode gameMode = SCR_BaseGameMode.Cast(GetGame().GetGameMode());
		if (!gameMode)
			return null;

		return TBD_SafestartManager.Cast(gameMode.FindComponent(TBD_SafestartManager));
	}

	//! Set the default countdown length on the server.
	//! @param owner the game mode entity
	//! @authority server
	override void OnPostInit(IEntity owner)
	{
		super.OnPostInit(owner);

		if (TBD_Authority.IsClient())
			return;

		m_iConfiguredSeconds = DEFAULT_COUNTDOWN_SECONDS;
	}

	//! Cancel the countdown and every helper poll, so no timer fires into the next world.
	//! @param owner the game mode entity
	override void OnDelete(IEntity owner)
	{
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
			queue.Remove(TickCountdown);

		m_Protection.CancelCallbacks();
		m_Watchdog.CancelCallbacks();

		super.OnDelete(owner);
	}

	//! Whether damage is suppressed now; the one question other systems ask.
	//! @return true while armed
	bool IsArmed()
	{
		return m_bArmed;
	}

	//! The replicated countdown.
	//! @return seconds left, or NOT_RUNNING
	int GetSecondsRemaining()
	{
		return m_iSecondsRemaining;
	}

	//! Bodies the last restore could not verify; non-zero after a lift means players may be invulnerable.
	//! @return the unrestored count
	int GetUnrestoredCount()
	{
		return m_Protection.UnrestoredCount();
	}

	//! Stage hook for every transition: LOBBY, BRIEFING and SAFE_START arm (idempotently), any other
	//! stage lifts, so an admin jumping to END cannot leave players invulnerable.
	//! @param stage the new stage
	//! @authority server
	void OnStageChanged(TBD_EGameStage stage)
	{
		if (TBD_Authority.IsClient())
			return;

		if (stage == TBD_EGameStage.LOBBY || stage == TBD_EGameStage.BRIEFING || stage == TBD_EGameStage.SAFE_START)
		{
			Arm();
			return;
		}

		Lift(typename.EnumToString(TBD_EGameStage, stage));
	}

	//! Arm the shield on the first call of a run (sweep, counters, handlers); on SAFE_START also start
	//! the countdown when it is not running. Logs the arm and bodies already found invulnerable.
	//! @authority server
	protected void Arm()
	{
		bool first = !m_bArmed;
		int covered = 0;
		if (first)
		{
			m_bArmed = true;
			m_Watchdog.Reset();
			covered = m_Protection.BeginArm();
		}

		// The countdown runs on SAFE_START only; its stage-drift check would lift a LOBBY arm.
		TBD_FrameworkManager framework = TBD_FrameworkManager.GetInstance();
		if (framework)
		{
			if (framework.GetStage() == TBD_EGameStage.SAFE_START)
			{
				if (m_iSecondsRemaining == NOT_RUNNING)
				{
					SetCountdown(m_iConfiguredSeconds);

					string msg = "[TBD] SAFESTART -- damage OFF, weapons cold. Live in ";
					msg += TBD_ClockText.FormatClock(m_iSecondsRemaining);
					msg += ".";
					TBD_PlayerChat.Broadcast(TBD_Log.CH_SAFESTART, msg);

					GetGame().GetCallqueue().Remove(TickCountdown);
					GetGame().GetCallqueue().CallLater(TickCountdown, 1000, true);
				}
			}
		}

		if (first)
		{
			TBD_Log.Kv(TBD_Log.CH_SAFESTART, "armed",
				string.Format("seconds=%1 bodies=%2 foundAlreadyOff=%3",
					m_iSecondsRemaining, covered, m_Protection.FoundDisabledCount()));

			// Said during the warmup: these bodies stay invulnerable at lift.
			int foundDisabled = m_Protection.FoundDisabledCount();
			if (foundDisabled > 0)
			{
				string alreadyOff = "found ";
				alreadyOff += foundDisabled.ToString();
				alreadyOff += " body(s) with damage handling ALREADY off -- safestart will leave them off at lift, not force them on.";
				TBD_Log.Warn(TBD_Log.CH_SAFESTART, alreadyOff);
			}
		}
	}

	//! End safe start and give every body its damage back. The order is the safety argument: clear
	//! the armed flag, stop the timers and the countdown, restore each body verified by read-back,
	//! then start the watchdog when anything did not verify.
	//! @param reason the log label (a stage name, `stage-drift` or a go-live reason)
	//! @authority server
	void Lift(string reason)
	{
		if (TBD_Authority.IsClient())
			return;

		if (!m_bArmed && m_Protection.HeldCount() == 0)
			return;

		m_bArmed = false;

		GetGame().GetCallqueue().Remove(TickCountdown);
		m_Protection.CancelCallbacks();

		SetCountdown(NOT_RUNNING);

		int owed = m_Protection.HeldCount();
		int unrestored = m_Protection.Restore();

		string kv = string.Format("reason=%1 bodies=%2 unrestored=%3", reason, owed, unrestored);
		kv += string.Format(" leftDisabled=%1", m_Protection.LeftDisabledCount());
		kv += string.Format(" suppressedShots=%1 suppressedThrows=%2", m_Protection.SuppressedShots(), m_Protection.SuppressedThrows());
		TBD_Log.Kv(TBD_Log.CH_SAFESTART, "lift", kv);

		if (unrestored == 0)
		{
			TBD_PlayerChat.Broadcast(TBD_Log.CH_SAFESTART, "[TBD] SAFESTART OVER -- WEAPONS LIVE. Damage is ON, and you have ONE life.");
			return;
		}

		m_Watchdog.OnLiftFailed();
	}

	//! Countdown reached zero or an admin said go: ask the stage machine for LIVE, which lifts
	//! through OnStageChanged; when it is gone or past SAFE_START, lift directly.
	//! @param reason the log label
	//! @authority server
	void GoLive(string reason)
	{
		if (TBD_Authority.IsClient())
			return;

		TBD_FrameworkManager framework = TBD_FrameworkManager.GetInstance();
		if (framework && framework.GetStage() == TBD_EGameStage.SAFE_START)
		{
			framework.SetStage(TBD_EGameStage.LIVE);
			return;
		}

		// No transition will arrive, and damage still has to come back.
		TBD_Log.Warn(TBD_Log.CH_SAFESTART,
			"go-live could not drive the stage machine -- lifting directly (reason=" + reason + ")");
		Lift(reason);
	}

	//! One countdown second: lifts on stage drift, goes live at zero, broadcasts chat milestones.
	//! @authority server
	protected void TickCountdown()
	{
		if (!m_bArmed)
		{
			GetGame().GetCallqueue().Remove(TickCountdown);
			return;
		}

		// The stage left SAFE_START without OnStageChanged reaching here: lift now.
		TBD_FrameworkManager framework = TBD_FrameworkManager.GetInstance();
		if (framework && framework.GetStage() != TBD_EGameStage.SAFE_START)
		{
			TBD_Log.Warn(TBD_Log.CH_SAFESTART, "stage left SAFE_START without notifying safestart -- lifting now");
			Lift("stage-drift");
			return;
		}

		int next = m_iSecondsRemaining - 1;
		if (next < 0)
			next = 0;

		SetCountdown(next);

		if (next <= 0)
		{
			GoLive("countdown expired");
			return;
		}

		if (TBD_ClockText.IsCountdownChatMilestone(next))
		{
			string msg = "[TBD] SAFESTART -- live in ";
			msg += TBD_ClockText.FormatClock(next);
			msg += ". Weapons cold, damage off.";
			TBD_PlayerChat.Broadcast(TBD_Log.CH_SAFESTART, msg);
		}
	}

	//! The one writer of m_iSecondsRemaining: replicate, then drive the local UI (a listen host
	//! never receives its own replication hook).
	//! @param seconds the new countdown, or NOT_RUNNING
	//! @authority server
	protected void SetCountdown(int seconds)
	{
		m_iSecondsRemaining = seconds;
		Replication.BumpMe();
		NotifyLocalSafestartUI();
	}

	//! One line an admin reads at a glance; safe in any stage.
	//! @return off with the next length and any unrestored or found-invulnerable bodies, or on with
	//! time left, bodies and suppressed rounds
	string StatusLine()
	{
		if (!m_bArmed)
		{
			string idle = string.Format("TBD safestart: OFF (next arm = %1). ", TBD_ClockText.FormatClock(m_iConfiguredSeconds));
			int unrestored = m_Protection.UnrestoredCount();
			if (unrestored > 0)
				return idle + string.Format("!! %1 body(s) NOT restored -- damage may be off for them.", unrestored);

			idle += "Damage is live.";
			// Appended: damage is live for every body safe start turned it off for.
			int leftDisabled = m_Protection.LeftDisabledCount();
			if (leftDisabled > 0)
			{
				idle += string.Format(" (%1 body(s) were already invulnerable before safestart and were left that way.)",
					leftDisabled);
			}
			return idle;
		}

		string armed = string.Format("TBD safestart: ON, live in %1. ", TBD_ClockText.FormatClock(m_iSecondsRemaining));
		armed += string.Format("bodies=%1 suppressed shots=%2 grenades=%3",
			m_Protection.HeldCount(), m_Protection.SuppressedShots(), m_Protection.SuppressedThrows());
		int foundDisabled = m_Protection.FoundDisabledCount();
		if (foundDisabled > 0)
			armed += string.Format(" -- %1 of those were ALREADY damage-off when swept", foundDisabled);
		return armed;
	}

	//! Set the countdown length: at once when armed (with a chat line), else for the next arm.
	//! @param seconds the length, MIN_COUNTDOWN_SECONDS to MAX_COUNTDOWN_SECONDS
	//! @param ok set true when accepted
	//! @return the reply for the admin
	//! @authority server
	string AdminSetSeconds(int seconds, out bool ok)
	{
		ok = false;

		if (TBD_Authority.IsClient())
			return "TBD: safestart is server-side only.";

		if (seconds < MIN_COUNTDOWN_SECONDS || seconds > MAX_COUNTDOWN_SECONDS)
		{
			return string.Format("TBD: safestart length must be %1-%2 seconds.",
				MIN_COUNTDOWN_SECONDS, MAX_COUNTDOWN_SECONDS);
		}

		m_iConfiguredSeconds = seconds;
		ok = true;

		if (!m_bArmed)
			return string.Format("TBD: safestart length set to %1 (applies when SAFE_START is entered).", TBD_ClockText.FormatClock(seconds));

		SetCountdown(seconds);
		string msg = "[TBD] SAFESTART extended -- live in ";
		msg += TBD_ClockText.FormatClock(seconds);
		msg += ".";
		TBD_PlayerChat.Broadcast(TBD_Log.CH_SAFESTART, msg);
		return string.Format("TBD: safestart now ends in %1.", TBD_ClockText.FormatClock(seconds));
	}

	//! Replication hook of m_iSecondsRemaining: drive this machine's pop-ups.
	//! @authority client
	void OnCountdownReplicated()
	{
		NotifyLocalSafestartUI();
	}

	//! Show the countdown on this machine as SCR_PopUpNotification banners (a vanilla HUD element;
	//! the durable channel is the server's chat line). Called from OnCountdownReplicated and from
	//! SetCountdown, because a listen host never receives its own replication hook; a dedicated
	//! server has no workspace and shows nothing.
	protected void NotifyLocalSafestartUI()
	{
		// No workspace: a dedicated server.
		if (!GetGame().GetWorkspace())
			return;

		int now = m_iSecondsRemaining;
		int last = m_iLocalLastSeen;
		if (now == last)
			return;

		m_iLocalLastSeen = now;

		SCR_PopUpNotification popup = SCR_PopUpNotification.GetInstance();
		if (!popup)
			return;

		// Lifted: was running, now is not.
		if (now < 0)
		{
			if (last >= 0)
				popup.PopupMsg("SAFESTART OVER -- WEAPONS LIVE", 8, "Damage is ON. You have ONE life.");
			return;
		}

		// First value this machine has seen: safe start just armed, or this client joined into it.
		if (last < 0)
		{
			popup.PopupMsg("SAFESTART -- WEAPONS COLD", 8,
				"No damage, rounds are suppressed. Live in " + TBD_ClockText.FormatClock(now) + ".");
			return;
		}

		if (TBD_ClockText.IsCountdownPopupMilestone(now))
			popup.PopupMsg("SAFESTART -- LIVE IN " + TBD_ClockText.FormatClock(now), 3, "Weapons cold, damage off");
	}
}
