/**
 * @file TBD_GameRuntimeRestCallback.c
 * @brief The engine callback of one game-runtime request and the answer the engine reports into it.
 *
 * Role: the `RestCallback` every machine-credential request is sent with; its two engine handlers
 * copy the HTTP status, the transport result and the response body into this object's own fields
 * and raise `m_bReported` last.  Position: created by `TBD_GameRuntimeHttp.Track` for each send
 * and owned by the `TBD_GameRuntimeCall` it answers; read on the main thread by the transport's
 * delivery pass and watchdog, which build the `TBD_GameRuntimeAnswer` from it.
 * State: the reported status, transport result, body and handler, written once by the engine's
 * REST callback thread and read by the main thread once `m_bReported` is true.
 * Invariants: the engine runs the `SetOnSuccess` and `SetOnError` handlers on its REST callback
 * thread, not on the main thread, so `CaptureSuccess` and `CaptureError` are the only code that runs
 * there, and they write nothing but the fields of the callback they are handed: no container, no
 * call queue, no log, no entity, no player; `m_bReported` is written after every other field and
 * never cleared, so a main-thread reader that sees it true reads a complete answer; the body is kept
 * only when a status arrived or the success handler fired, because `GetData` returns the request
 * body on a transport failure.
 */

//! One request's engine callback and the answer the engine reported through it.
class TBD_GameRuntimeRestCallback : RestCallback
{
	HttpCode m_eReportedCode; //!< the HTTP status the engine reported; `HTTP_CODE_NULL` until one arrives
	ERestResult m_eReportedRestResult; //!< the engine's transport result; `EREST_EMPTY` until reported
	string m_sReportedBody; //!< the response body; empty unless a status arrived or the success handler fired
	bool m_bReportedOnSuccess; //!< true when the success handler fired
	bool m_bReported; //!< raised last by an engine handler: every field above is final

	//! A callback with both engine handlers set, ready to be sent with one request.
	//! @return the callback
	static TBD_GameRuntimeRestCallback Create()
	{
		TBD_GameRuntimeRestCallback callback = new TBD_GameRuntimeRestCallback();
		callback.SetOnSuccess(CaptureSuccess);
		callback.SetOnError(CaptureError);
		return callback;
	}

	//! Engine success handler of every game-runtime request; runs on the engine's REST callback
	//! thread and records the answer in `callback` only.
	//! @param callback the callback the engine answered through
	static void CaptureSuccess(RestCallback callback)
	{
		TBD_GameRuntimeRestCallback reported = TBD_GameRuntimeRestCallback.Cast(callback);
		if (reported)
			reported.Capture(true);
	}

	//! Engine error handler of every game-runtime request; runs on the engine's REST callback thread
	//! and records the answer in `callback` only.
	//! @param callback the callback the engine answered through
	static void CaptureError(RestCallback callback)
	{
		TBD_GameRuntimeRestCallback reported = TBD_GameRuntimeRestCallback.Cast(callback);
		if (reported)
			reported.Capture(false);
	}

	//! Copy what the engine reported into this object's fields, `m_bReported` last; a second report
	//! of the same request changes nothing.
	//! @param onSuccess true when the success handler fired
	protected void Capture(bool onSuccess)
	{
		if (m_bReported)
			return;

		m_eReportedCode = GetHttpCode();
		m_eReportedRestResult = GetRestResult();
		if (m_eReportedCode != HttpCode.HTTP_CODE_NULL || onSuccess)
			m_sReportedBody = GetData();

		m_bReportedOnSuccess = onSuccess;
		m_bReported = true;
	}
}
