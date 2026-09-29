/**
 * @file TBD_GameRuntimeAnswer.c
 * @brief The answer to one game-runtime request, classified for its sender.
 *
 * Role: turns the status, transport result and body the engine reported for one request into an
 * answer, reading the error envelope and classifying the outcome.  Position: built on the main
 * thread by `TBD_GameRuntimeHttp` from the request's `TBD_GameRuntimeRestCallback`, or by its
 * watchdog, and delivered to the request's `TBD_GameRuntimeCall.OnAnswered`.
 * State: none beyond each answer's own fields.  Invariants: answers are classified by HTTP status
 * and by the machine-readable `details.code` of a 409, never by message text; a 204 is its own
 * `NO_CONTENT` outcome, neither a success with a body nor a failure; the `details.code` of any
 * other error status (404 `NO_DEPLOYMENT`, 403 and 422 refusals) is carried in `m_sErrorCode`; an
 * answer carries a body only when a status arrived with it or the success handler fired, because
 * `RestCallback.GetData()` returns the request body on a transport failure and
 * `TBD_GameRuntimeRestCallback` keeps no body then.
 */

//! How a finished game-runtime request is handled.
enum TBD_EGameRuntimeOutcome
{
	SUCCESS,    //!< 200, 201 or 202, or a status-less success handler: the body is the answer.
	NO_CONTENT, //!< 204: the platform answered and has nothing to return; only the fleet command claim answers it.
	REFUSED,    //!< 409 carrying a fence code in `details.code`.
	TRANSIENT, //!< No answer, a timeout or a server-side failure: the same request may succeed later.
	PERMANENT, //!< A client-error status: repeating the same request cannot succeed.
}

//! `details` of a refused game-runtime request: `STALE_GENERATION` (with `generation`),
//! `STALE_SEQUENCE` (with `last_sequence`), `RUNTIME_SESSION_ENDED` (with `end_reason`), or a fleet
//! command report's `STALE_FENCING_TOKEN` / `COMMAND_NOT_CLAIMED` / `COMMAND_NOT_EXECUTING` (with the
//! command's `state`). Field names are the JSON keys.
class TBD_GameRuntimeRefusalDetails
{
	string code; //!< JSON key `code`; the machine-readable refusal code
	string end_reason; //!< JSON key `end_reason`, with `RUNTIME_SESSION_ENDED`
	int last_sequence; //!< JSON key `last_sequence`, with `STALE_SEQUENCE`
	int generation; //!< JSON key `generation`, with `STALE_GENERATION`
	string state; //!< JSON key `state`; the fleet command's state
}

//! The backend error envelope, `{"error": message, "details": {...}}`.
class TBD_GameRuntimeErrorBody
{
	string error; //!< JSON key `error`; the human-readable message, never classified on
	ref TBD_GameRuntimeRefusalDetails details; //!< JSON key `details`, or null
}

//! One classified game-runtime answer.
//! @authority server
class TBD_GameRuntimeAnswer
{
	protected static const int LOGGED_BODY_MAX_BYTES = 400; //!< cap of a logged body, in bytes; `Print` drops a line over 1024 bytes
	protected static const int HTTP_STATUS_NO_CONTENT = 204; //!< the raw status the engine reports for a 204, through the success handler; `HttpCode` names no 204 member

	TBD_EGameRuntimeOutcome m_eOutcome; //!< how the sender handles the answer
	HttpCode m_eCode; //!< the HTTP status; `HTTP_CODE_NULL` when none arrived
	string m_sBody; //!< the response body; empty when no status arrived
	ref TBD_GameRuntimeRefusalDetails m_Refusal; //!< the 409 fence details, or null
	string m_sErrorCode; //!< `details.code` of an error answer of any status, or empty
	string m_sDetail; //!< one log line: the status and response body, or the transport result

	//! The answer the engine reported for one request, as its callback recorded it.
	//! @param code the HTTP status; `HTTP_CODE_NULL` when none arrived
	//! @param restResult the engine's transport result
	//! @param body the response body; empty when no status arrived and the error handler fired
	//! @param arrivedOnSuccess which handler fired; a success handler that reports no status still
	//! delivered its body
	//! @return the classified answer
	static TBD_GameRuntimeAnswer Reported(HttpCode code, ERestResult restResult, string body, bool arrivedOnSuccess)
	{
		TBD_GameRuntimeAnswer answer = new TBD_GameRuntimeAnswer();
		answer.m_eCode = code;
		answer.m_sBody = body;
		answer.m_eOutcome = answer.Classify(arrivedOnSuccess);

		// The status goes out by its `HttpCode` name; a status the enum does not name, 204 among
		// them, reads `unknown`.
		string status = typename.EnumToString(HttpCode, answer.m_eCode);
		if (answer.m_eCode == HttpCode.HTTP_CODE_NULL)
			answer.m_sDetail = status + " rest=" + typename.EnumToString(ERestResult, restResult);
		else
			answer.m_sDetail = status + " response=" + LoggableBody(answer.m_sBody);

		return answer;
	}

	//! The TRANSIENT answer to a request the engine never reported.
	//! @param detail why, for the log line
	//! @return the answer
	static TBD_GameRuntimeAnswer Unanswered(string detail)
	{
		TBD_GameRuntimeAnswer answer = new TBD_GameRuntimeAnswer();
		answer.m_eOutcome = TBD_EGameRuntimeOutcome.TRANSIENT;
		answer.m_eCode = HttpCode.HTTP_CODE_NULL;
		answer.m_sDetail = detail;
		return answer;
	}

	//! A response body made safe for one log line: capped, and named when empty.
	//! @return `<empty>`, the body, or its first `LOGGED_BODY_MAX_BYTES` bytes marked truncated
	static string LoggableBody(string body)
	{
		if (body.IsEmpty())
			return "<empty>";

		if (body.Length() > LOGGED_BODY_MAX_BYTES)
			return body.Substring(0, LOGGED_BODY_MAX_BYTES) + "...<truncated>";

		return body;
	}

	//! Classify this answer: 200, 201, 202, or a status-less success handler, is SUCCESS; 204 is
	//! NO_CONTENT; no status otherwise is TRANSIENT; a 409 with `details.code` is REFUSED; a client
	//! error is PERMANENT; anything else is TRANSIENT. Sets `m_sErrorCode` from any `details.code`
	//! and `m_Refusal` for a 409.
	//! @return the outcome
	protected TBD_EGameRuntimeOutcome Classify(bool arrivedOnSuccess)
	{
		if (m_eCode == HttpCode.HTTP_CODE_200 || m_eCode == HttpCode.HTTP_CODE_201 || m_eCode == HttpCode.HTTP_CODE_202)
			return TBD_EGameRuntimeOutcome.SUCCESS;

		if (m_eCode == HTTP_STATUS_NO_CONTENT)
			return TBD_EGameRuntimeOutcome.NO_CONTENT;

		if (m_eCode == HttpCode.HTTP_CODE_NULL)
		{
			if (arrivedOnSuccess)
				return TBD_EGameRuntimeOutcome.SUCCESS;

			return TBD_EGameRuntimeOutcome.TRANSIENT;
		}

		TBD_GameRuntimeRefusalDetails details = ParseErrorDetails(m_sBody);
		if (details)
			m_sErrorCode = details.code;

		if (m_eCode == HttpCode.HTTP_CODE_409)
		{
			m_Refusal = details;
			if (m_Refusal)
				return TBD_EGameRuntimeOutcome.REFUSED;

			return TBD_EGameRuntimeOutcome.PERMANENT;
		}

		if (IsClientError(m_eCode))
			return TBD_EGameRuntimeOutcome.PERMANENT;

		return TBD_EGameRuntimeOutcome.TRANSIENT;
	}

	//! Redirects and client errors other than 409 (classified on its own): the request itself is
	//! wrong for this backend, so sending it again unchanged cannot succeed. 408 (request timeout) is
	//! not among them. Every other status - server errors, gateway failures, anything unrecognised -
	//! is worth another attempt.
	//! @return true for a status that cannot succeed on repeat
	protected static bool IsClientError(HttpCode code)
	{
		if (code == HttpCode.HTTP_CODE_300 || code == HttpCode.HTTP_CODE_301)
			return true;
		if (code == HttpCode.HTTP_CODE_302 || code == HttpCode.HTTP_CODE_303)
			return true;
		if (code == HttpCode.HTTP_CODE_400 || code == HttpCode.HTTP_CODE_401)
			return true;
		if (code == HttpCode.HTTP_CODE_403 || code == HttpCode.HTTP_CODE_404)
			return true;
		if (code == HttpCode.HTTP_CODE_405 || code == HttpCode.HTTP_CODE_412)
			return true;
		if (code == HttpCode.HTTP_CODE_413 || code == HttpCode.HTTP_CODE_418)
			return true;
		if (code == HttpCode.HTTP_CODE_422)
			return true;

		return false;
	}

	//! The `details` of an error envelope.
	//! @return the details, or null when the body carries no `details.code`
	protected static TBD_GameRuntimeRefusalDetails ParseErrorDetails(string body)
	{
		if (body.IsEmpty())
			return null;

		JsonLoadContext context = new JsonLoadContext();
		if (!context.LoadFromString(body))
			return null;

		TBD_GameRuntimeErrorBody envelope = new TBD_GameRuntimeErrorBody();
		if (!context.ReadValue("", envelope))
			return null;

		if (!envelope.details || envelope.details.code.IsEmpty())
			return null;

		return envelope.details;
	}
}
