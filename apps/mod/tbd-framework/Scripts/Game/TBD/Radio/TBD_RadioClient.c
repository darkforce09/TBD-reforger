//! T-181.40 — the CLIENT half of the radio plan: asking for your nets, and reading them.
//!
//! ── Where a player actually reads this ──────────────────────────────────────────────────────
//! A vanilla HINT (`SCR_HintManagerComponent.ShowCustomHint`), with a popup
//! (`SCR_PopUpNotification`) as the fallback when the player has hints switched off. Both are
//! vanilla HUD elements and neither needs a `.layout` or a `.conf`, which is not a stylistic
//! preference — **all five TBD menu presets currently fail to resolve** (`GUI (E): Menu preset
//! 'TBD_UIBriefing' not found!` on every boot) because `resourceDatabase.rdb` is stale, so a
//! TBD screen is a screen nobody can open. The same reasoning is already recorded on
//! `TBD_SafestartManager.NotifyLocalSafestartUI`, which shows its countdown the same way.
//!
//! A hint is chosen over a popup as the PRIMARY surface because a net list is reference material:
//! `duration = 0` leaves it on screen until the player dismisses it, so somebody who alt-tabs
//! during BRIEFING has not lost their frequencies. The popup path is transient and is only used
//! when `CanShowHints()` says the player turned hints off.
//!
//! When the briefing screen can open again, `GetNets()` / `GetNetLine()` below are the accessors it
//! needs — no new wire, no second request. The exact additive lines for `UI/TBD_BriefingData.c`
//! are reported to the command center rather than written here; that file belongs to another slice.
//!
//! ── Late joiners (the defect this slice refuses to repeat) ──────────────────────────────────
//! T-181.28 records the briefing shipping push-only and therefore silently missing anyone who
//! joins while the round is already running. Nets are PULL-driven, with two independent triggers,
//! either of which alone is enough:
//!   1. a poll that runs until the server gives an authoritative answer, and then stops. A player
//!      who joins unslotted keeps asking; the moment they take a seat, the next tick serves them.
//!   2. every time the player OPENS THE MAP — which is also exactly when somebody wants to know
//!      what they are on, and what picks up an admin mission switch or a re-slot to the other side.
//! The server's own stage sweep (`TBD_RadioService.OnStageChanged`) is a THIRD trigger and pushes
//! independently, so no single mechanism is load-bearing.
//!
//! The poll is deliberate, not laziness: the client has no locally readable signal for "my slot was
//! assigned" — `m_mPlayerSlot` is a plain map on the server, not an `RplProp` — so there is nothing
//! to catch up ON.
//!
//! ── This never claims a radio was tuned ─────────────────────────────────────────────────────
//! The text a player sees is built from `m_iTuned`, which the server only increments after reading
//! the frequency back off the transceiver. With no `RadioManagerEntity` in the world the count is
//! zero and the hint says the frequencies must be dialled in by hand, because that is the truth.
class TBD_RadioClient
{
	//! How often an unserved client re-asks. Stops entirely once served, so the steady state of a
	//! full server is ZERO radio traffic.
	static const int POLL_MS = 5000;

	//! Floor between two map-open requests, so hammering the map key cannot spam the server.
	static const float MAP_REQUEST_MIN_GAP_MS = 3000;

	//! Title on the hint. `·` and `—` are in the proven glyph set for shipped TBD screens; `->` is
	//! used instead of `->`-shaped arrows anywhere load-bearing, per the recorded glyph landmine.
	static const string HINT_TITLE = "RADIO NETS";

	//! Set once `Start()` has armed the poll and the map hook, so a double start is a no-op.
	protected static bool s_bRunning;

	//! The server has given an authoritative answer at least once for the CURRENT mission+side.
	//! True even when the answer was "your side authored no nets" — that is an answer.
	protected static bool s_bServed;

	protected static ref array<string> s_aId;
	protected static ref array<string> s_aLabel;
	protected static ref array<int> s_aFreqKHz;
	protected static ref array<bool> s_aLongRange;

	protected static string s_sMissionId;
	protected static string s_sTuneResult;
	protected static int s_iTuned;

	//! World time of the last map-open-triggered request, for MAP_REQUEST_MIN_GAP_MS.
	protected static float s_fLastMapRequestMs;

	//! Fingerprint of what has already been SHOWN, so a poll or a map open does not re-open a hint
	//! the player already dismissed. Only a genuinely different answer re-displays.
	protected static string s_sShownFingerprint;

	//------------------------------------------------------------------------------------------------
	//! @authority client — arm the pull. Called by `TBD_RadioComponent` on any machine with a
	//! workspace (a dedicated server has none and has nobody to show anything to).
	static void Start()
	{
		if (s_bRunning)
			return;

		s_bRunning = true;

		SCR_MapEntity.GetOnMapOpen().Insert(OnMapOpen);
		ArmPoll();

		// Ask immediately as well as on the timer: on a listen host the answer is synchronous, so
		// a slotted host player sees their nets without waiting out a poll interval.
		Request();
	}

	//------------------------------------------------------------------------------------------------
	//! Statics outlive a world inside one process (a recorded landmine in this program), so every
	//! timer and invoker has to be released here. Without this an in-process scenario restart
	//! leaves a poll firing against a dead world.
	static void Shutdown()
	{
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
			queue.Remove(Tick);

		// Called directly, not through a local: `GetOnMapOpen()` returns
		// `ScriptInvokerBase<MapConfigurationInvoker>`, which is unrelated to plain `ScriptInvoker`
		// and will not bind to one. Same shape `TBD_MarkerClient.Shutdown` uses.
		SCR_MapEntity.GetOnMapOpen().Remove(OnMapOpen);

		s_bRunning = false;
		s_bServed = false;
		s_aId = null;
		s_aLabel = null;
		s_aFreqKHz = null;
		s_aLongRange = null;
		s_sMissionId = string.Empty;
		s_sTuneResult = string.Empty;
		s_iTuned = 0;
		s_fLastMapRequestMs = 0;
		s_sShownFingerprint = string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! @authority owner — the server's answer, whichever path it arrived by (RPC on a dedicated
	//! client, direct call on a listen host).
	//!
	//! `served == false` is not an error: it means the server has nothing authoritative for this
	//! player yet, and the poll keeps running. Nothing is cached from an unserved reply, so a
	//! refusal can never overwrite a good answer with an empty one.
	static void Accept(array<string> ids, array<string> labels, array<int> freqKHz,
		array<bool> longRange, string missionId, string tuneResult, int tuned, bool served)
	{
		if (!served)
			return;

		s_bServed = true;
		s_aId = ids;
		s_aLabel = labels;
		s_aFreqKHz = freqKHz;
		s_aLongRange = longRange;
		s_sMissionId = missionId;
		s_sTuneResult = tuneResult;
		s_iTuned = tuned;

		// Served means the question is answered; stop asking. The map hook and the server's stage
		// sweep still refresh this if anything changes.
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
			queue.Remove(Tick);

		ShowIfChanged();
	}

	//------------------------------------------------------------------------------------------------
	//! The nets this player is on, as display lines. Public so the briefing screen can render them
	//! when menu presets resolve again, without a second request or a second wire.
	//! Never returns null.
	static array<string> GetNetLines()
	{
		array<string> lines = {};
		if (!s_bServed || !s_aId)
			return lines;

		for (int i = 0; i < s_aId.Count(); i++)
		{
			lines.Insert(GetNetLine(i));
		}

		return lines;
	}

	//------------------------------------------------------------------------------------------------
	//! `"Alpha Squad · 42.500 MHz · SR"` — one net, formatted for a human.
	//!
	//! The frequency text is derived from the INTEGER kHz that was sent to the transceiver, so what
	//! the player reads and what the radio was set to cannot drift apart.
	static string GetNetLine(int index)
	{
		if (!s_aId || index < 0 || index >= s_aId.Count())
			return string.Empty;

		string band = "SR";
		if (s_aLongRange && index < s_aLongRange.Count() && s_aLongRange[index])
			band = "LR";

		// Appended in steps — a long `+` chain trips `Formula too complex`, whose second
		// diagnostic is a misleading `Incompatible parameter`.
		string line = s_aLabel[index];
		line = line + " · ";
		line = line + TBD_RadioPlan.FormatMHz(s_aFreqKHz[index]);
		line = line + " · ";
		line = line + band;
		return line;
	}

	//------------------------------------------------------------------------------------------------
	//! How many nets this player is on. `0` after being served is a real answer, not "not yet".
	static int GetNetCount()
	{
		if (!s_bServed || !s_aId)
			return 0;

		return s_aId.Count();
	}

	//------------------------------------------------------------------------------------------------
	//! True once the server has answered authoritatively at least once.
	static bool IsServed()
	{
		return s_bServed;
	}

	//------------------------------------------------------------------------------------------------
	//! Show the current net list on demand, whether or not it changed. The entry point a future
	//! briefing panel or an admin command can call.
	static void ShowNow()
	{
		if (!s_bServed)
			return;

		s_sShownFingerprint = Fingerprint();
		Display();
	}

	//------------------------------------------------------------------------------------------------
	//! (Re)start the unserved poll. `Remove` first so arming twice cannot stack two timers —
	//! `ScriptCallQueue.Remove` cancels by FUNCTION, which is precisely the semantics wanted here.
	protected static void ArmPoll()
	{
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (!queue)
			return;

		queue.Remove(Tick);
		queue.CallLater(Tick, POLL_MS, true);
	}

	//------------------------------------------------------------------------------------------------
	protected static void Tick()
	{
		if (s_bServed)
		{
			ScriptCallQueue queue = GetGame().GetCallqueue();
			if (queue)
				queue.Remove(Tick);

			return;
		}

		Request();
	}

	//------------------------------------------------------------------------------------------------
	//! Re-ask whenever the map opens. Cheap when idle, and it is the moment a player wants to know
	//! their frequencies anyway. Rate-limited so holding the map key cannot spam the server.
	protected static void OnMapOpen(MapConfiguration config)
	{
		float now = GetGame().GetWorld().GetWorldTime();
		if (s_fLastMapRequestMs > 0 && now - s_fLastMapRequestMs < MAP_REQUEST_MIN_GAP_MS)
			return;

		s_fLastMapRequestMs = now;
		Request();
	}

	//------------------------------------------------------------------------------------------------
	protected static void Request()
	{
		PlayerController pc = GetGame().GetPlayerController();
		if (!pc)
			return;

		SCR_PlayerController spc = SCR_PlayerController.Cast(pc);
		if (!spc)
			return;

		spc.TBD_RequestRadioNets();
	}

	//------------------------------------------------------------------------------------------------
	//! Display only when the answer is genuinely different from the last one shown, so a poll, a
	//! map open and a server stage sweep cannot between them re-open a hint the player dismissed.
	protected static void ShowIfChanged()
	{
		string fingerprint = Fingerprint();
		if (fingerprint == s_sShownFingerprint)
			return;

		s_sShownFingerprint = fingerprint;
		Display();
	}

	//------------------------------------------------------------------------------------------------
	//! Everything that would make the on-screen text different. Built in steps rather than one long
	//! `+` chain (`Formula too complex`).
	protected static string Fingerprint()
	{
		string fp = s_sMissionId;
		fp = fp + "|";
		fp = fp + s_sTuneResult;
		fp = fp + "|";
		fp = fp + s_iTuned.ToString();

		array<string> lines = GetNetLines();
		foreach (string line : lines)
		{
			fp = fp + "|";
			fp = fp + line;
		}

		return fp;
	}

	//------------------------------------------------------------------------------------------------
	//! Put the net list on screen.
	protected static void Display()
	{
		// No workspace = dedicated server. It has no screen and must never try to drive one.
		if (!GetGame().GetWorkspace())
			return;

		string body = BuildBody();

		// A hint stays until dismissed (`duration = 0`), which is what a reference list wants.
		// `isSilent = true`: this is information, not an alert, and it fires again on a side change.
		if (SCR_HintManagerComponent.CanShowHints())
		{
			SCR_HintManagerComponent.ShowCustomHint(body, HINT_TITLE, 0, true);
			return;
		}

		// The player turned hints off. Fall back to the transient surface rather than showing them
		// nothing — the safestart countdown already uses this one.
		SCR_PopUpNotification popup = SCR_PopUpNotification.GetInstance();
		if (!popup)
			return;

		popup.PopupMsg(HINT_TITLE, 12, body);
	}

	//------------------------------------------------------------------------------------------------
	//! The text itself: one line per net, then one line saying whether anything was actually tuned.
	protected static string BuildBody()
	{
		array<string> lines = GetNetLines();

		if (lines.IsEmpty())
			return "Your side has no radio nets in this mission.";

		string body = string.Empty;
		foreach (string line : lines)
		{
			body = body + line;
			body = body + "\n";
		}

		body = body + "\n";
		body = body + TuneLine();
		return body;
	}

	//------------------------------------------------------------------------------------------------
	//! The honesty line. This is the sentence that must never lie: it is driven by `m_iTuned`,
	//! which the server increments only after reading the frequency back off the transceiver.
	protected static string TuneLine()
	{
		if (s_iTuned > 0)
		{
			string ok = "Your radio is tuned — ";
			ok = ok + s_iTuned.ToString();
			ok = ok + " of ";
			ok = ok + GetNetCount().ToString();
			ok = ok + " net(s) set automatically.";
			return ok;
		}

		if (s_sTuneResult == "NO_BACKBONE")
			return "Radio tuning is unavailable on this world — dial these in by hand.";

		if (s_sTuneResult == "NO_RADIO")
			return "You are not carrying a radio — dial these in on one you find.";

		if (s_sTuneResult == "NO_BODY")
			return "Frequencies only; your radio will be set once you are in a body.";

		return "Not tuned automatically — dial these in by hand.";
	}
}
