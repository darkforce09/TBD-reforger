/**
 * @file TBD_RadioClient.c
 * @brief Client half of the radio plan: asks for this player's nets and shows them.
 *
 * Role: pulls the player's side-scoped net list and shows it as a vanilla hint that stays until
 * dismissed (`SCR_HintManagerComponent.ShowCustomHint`, duration 0), or as a popup
 * (`SCR_PopUpNotification`) when the player turned hints off; neither needs a layout or a menu
 * preset.  Position: `TBD_RadioComponent` calls `Start` and `Shutdown`; answers arrive through the
 * modded `SCR_PlayerController` (reply RPC, stage-sweep push, or in place on a listen host).
 * State: static pull state and the last served answer, on the client.
 * Invariants: three independent triggers serve a late joiner: a poll every `POLL_MS` until served
 * (the slot map is not replicated, so there is no local signal to wait on), every map open at
 * most once per `MAP_REQUEST_MIN_GAP_MS`, and the server's stage sweep push; an unserved reply
 * caches nothing; the display re-opens only for a different answer; the tune line is driven by
 * the server's read-back count, so it never claims a tune that did not happen.
 */

//! Client-side radio net pull and display.
//! @authority client
class TBD_RadioClient
{
	static const int POLL_MS = 5000; //!< re-ask period in milliseconds while unserved
	static const float MAP_REQUEST_MIN_GAP_MS = 3000; //!< floor between two map-open requests in milliseconds
	static const string HINT_TITLE = "RADIO NETS"; //!< title of the hint and the popup

	protected static bool s_bRunning; //!< true once `Start` armed the poll and the map hook
	protected static bool s_bServed; //!< true once the server answered for the current mission and side, nets or not

	protected static ref array<string> s_aId; //!< served net ids
	protected static ref array<string> s_aLabel; //!< served net labels
	protected static ref array<int> s_aFreqKHz; //!< served frequencies in kHz
	protected static ref array<int> s_aLongRange; //!< served long-range flags, 1 or 0

	protected static string s_sMissionId; //!< mission id of the served answer
	protected static string s_sTuneResult; //!< the server's tune outcome name
	protected static int s_iTuned; //!< nets the server read back off a transceiver

	protected static float s_fLastMapRequestMs; //!< world time in ms of the last map-open request; 0 before the first
	protected static string s_sShownFingerprint; //!< fingerprint of the answer last shown; empty before the first

	//! Arm the pull: subscribe to map open, start the poll and ask once now (a listen host answers
	//! synchronously). A second call is a no-op.
	//! @authority client
	static void Start()
	{
		if (s_bRunning)
			return;

		s_bRunning = true;

		SCR_MapEntity.GetOnMapOpen().Insert(OnMapOpen);
		ArmPoll();
		Request();
	}

	//! Release the poll, the map hook and every cached answer; statics outlive a world inside one
	//! process, so a world restart without this would keep a poll firing against a dead world.
	//! @authority client
	static void Shutdown()
	{
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
			queue.Remove(Tick);

		// Called directly: `GetOnMapOpen()` returns `ScriptInvokerBase<MapConfigurationInvoker>`,
		// which does not bind to a plain `ScriptInvoker` local.
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

	//! Take the server's answer: cache a served one, stop the poll and show it when it differs
	//! from the last shown. An unserved answer is ignored and the poll keeps running.
	//! @param ids net id per net
	//! @param labels net label per net
	//! @param freqKHz frequency in kHz per net
	//! @param longRange 1 for a long-range net, 0 otherwise, per net
	//! @param missionId the mission the nets belong to
	//! @param tuneResult the server's tune outcome name
	//! @param tuned how many nets the server read back off a transceiver
	//! @param served false when the server had no authoritative answer
	//! @authority owner
	static void Accept(array<string> ids, array<string> labels, array<int> freqKHz,
		array<int> longRange, string missionId, string tuneResult, int tuned, bool served)
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

		// Served: stop polling; the map hook and the stage sweep still refresh it.
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
			queue.Remove(Tick);

		ShowIfChanged();
	}

	//! @return the served nets as display lines (see `GetNetLine`); empty, never null, while
	//! unserved
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

	//! Format one net as `<label> - <MHz> - SR|LR`, the frequency from the integer kHz the radio
	//! was set to.
	//! @param index the net's index
	//! @return the line, or empty for an index out of range
	static string GetNetLine(int index)
	{
		if (!s_aId || index < 0 || index >= s_aId.Count())
			return string.Empty;

		string band = "SR";
		if (s_aLongRange && index < s_aLongRange.Count() && s_aLongRange[index] == 1)
			band = "LR";

		// Appended in steps: a long `+` chain trips `Formula too complex`.
		string line = s_aLabel[index];
		line = line + " - ";
		line = line + TBD_RadioPlan.FormatMHz(s_aFreqKHz[index]);
		line = line + " - ";
		line = line + band;
		return line;
	}

	//! @return the served net count; 0 while unserved, and 0 is also a served answer
	static int GetNetCount()
	{
		if (!s_bServed || !s_aId)
			return 0;

		return s_aId.Count();
	}

	//! @return true once the server answered authoritatively
	static bool IsServed()
	{
		return s_bServed;
	}

	//! Show the served net list now, whether or not it changed. Does nothing while unserved.
	static void ShowNow()
	{
		if (!s_bServed)
			return;

		s_sShownFingerprint = Fingerprint();
		Display();
	}

	//! Start the unserved poll; any armed poll is removed first, so two calls never stack timers.
	protected static void ArmPoll()
	{
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (!queue)
			return;

		queue.Remove(Tick);
		queue.CallLater(Tick, POLL_MS, true);
	}

	//! One poll step: re-ask while unserved, cancel the poll once served.
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

	//! Re-ask on map open, at most once per `MAP_REQUEST_MIN_GAP_MS`.
	//! @param config the opened map's configuration; unused
	protected static void OnMapOpen(MapConfiguration config)
	{
		float now = GetGame().GetWorld().GetWorldTime();
		if (s_fLastMapRequestMs > 0 && now - s_fLastMapRequestMs < MAP_REQUEST_MIN_GAP_MS)
			return;

		s_fLastMapRequestMs = now;
		Request();
	}

	//! Ask the server for this player's nets through the local player controller; does nothing
	//! without one.
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

	//! Display only an answer that differs from the last one shown, so repeats never re-open a
	//! dismissed hint.
	protected static void ShowIfChanged()
	{
		string fingerprint = Fingerprint();
		if (fingerprint == s_sShownFingerprint)
			return;

		s_sShownFingerprint = fingerprint;
		Display();
	}

	//! @return mission id, tune result, tuned count and every net line, joined with `|`
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

	//! Show the net list as a silent hint that stays until dismissed, or as a popup when hints are
	//! off. Does nothing without a workspace.
	protected static void Display()
	{
		if (!GetGame().GetWorkspace())
			return;

		string body = BuildBody();

		// duration 0 keeps it until dismissed; isSilent true.
		if (SCR_HintManagerComponent.CanShowHints())
		{
			SCR_HintManagerComponent.ShowCustomHint(body, HINT_TITLE, 0, true);
			return;
		}

		// Hints are off: fall back to the transient popup.
		SCR_PopUpNotification popup = SCR_PopUpNotification.GetInstance();
		if (!popup)
			return;

		popup.PopupMsg(HINT_TITLE, 12, body);
	}

	//! @return one line per net, a blank line and the tune line; or the no-nets sentence
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

	//! @return the sentence saying whether the radio was tuned, driven by the server's read-back
	//! count and tune outcome
	protected static string TuneLine()
	{
		if (s_iTuned > 0)
		{
			string ok = "Your radio is tuned -- ";
			ok = ok + s_iTuned.ToString();
			ok = ok + " of ";
			ok = ok + GetNetCount().ToString();
			ok = ok + " net(s) set automatically.";
			return ok;
		}

		if (s_sTuneResult == "NO_BACKBONE")
			return "Radio tuning is unavailable on this world -- dial these in by hand.";

		if (s_sTuneResult == "NO_RADIO")
			return "You are not carrying a radio -- dial these in on one you find.";

		if (s_sTuneResult == "NO_BODY")
			return "Frequencies only; your radio will be set once you are in a body.";

		return "Not tuned automatically -- dial these in by hand.";
	}
}
