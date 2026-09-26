/**
 * @file TBD_SpectatorHostReporter.c
 * @brief Sends the spectator camera position to the server so the streaming host follows the view.
 *
 * Role: every second poll, sends the camera position through the player controller's
 * TBD_ReportSpectatorCamera when it has moved far enough since the last report.
 * Position: TBD_SpectatorController.Tick calls Report while spectating, and Enter, Leave and
 * Shutdown call Reset; the modded SCR_PlayerController carries the position to
 * TBD_SpectatorHost.MoveTo.
 * State: the tick counter and the last reported position (static, client).
 * Invariants: fire and forget, the client never mirrors whether a host exists (the server owns that
 * and drops a report with no host); the first report after Reset is always sent, even near the map
 * origin; a report under HOST_REPORT_MIN_MOVE_M from the last is not sent.
 */

//! Client camera-position reporter for the spectator streaming host. Static.
class TBD_SpectatorHostReporter
{
	static const int HOST_REPORT_EVERY_TICKS = 2; //!< polls per report; 2 x 250 ms = twice a second
	static const float HOST_REPORT_MIN_MOVE_M = 2.0; //!< metres; above TBD_SpectatorHost.MIN_MOVE_M, so no packet is spent on jitter

	protected static int s_iHostReportTicks; //!< polls since the last report attempt
	protected static bool s_bHostReported; //!< a report was sent since Reset, so the distance test applies
	protected static vector s_vHostReported; //!< world metres; the last reported position

	//! Report the camera position on every HOST_REPORT_EVERY_TICKS-th call when it moved at least
	//! HOST_REPORT_MIN_MOVE_M since the last report. Returns silently without a camera or a local
	//! player controller.
	//! @param camera the live spectator camera, or null
	//! @authority client
	static void Report(TBD_SpectatorCamera camera)
	{
		if (!camera)
			return;

		s_iHostReportTicks++;
		if (s_iHostReportTicks < HOST_REPORT_EVERY_TICKS)
			return;

		s_iHostReportTicks = 0;

		vector position = camera.GetPosition();
		if (s_bHostReported && vector.Distance(position, s_vHostReported) < HOST_REPORT_MIN_MOVE_M)
			return;

		SCR_PlayerController controller = SCR_PlayerController.Cast(GetGame().GetPlayerController());
		if (!controller)
			return;

		s_bHostReported = true;
		s_vHostReported = position;
		controller.TBD_ReportSpectatorCamera(position);
	}

	//! Forget the last report, so the next entry always sends a first report.
	static void Reset()
	{
		s_iHostReportTicks = 0;
		s_bHostReported = false;
		s_vHostReported = vector.Zero;
	}
}
