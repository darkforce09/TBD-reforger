//! The answer to one game-runtime request, read from the RestCallback the engine answered through
//! (TBD_GameRuntimeHttp delivers it to the request's TBD_GameRuntimeCall).
//!
//! Answers are classified by HTTP status and by the machine-readable `details.code` of a 409, never
//! by message text; the `details.code` of any other error status (404 `NO_DEPLOYMENT`, 403 and 422
//! refusals) is carried in `m_sErrorCode` for the caller to act on. `RestCallback.GetData()` returns
//! the REQUEST body on a transport failure, so an answer carries a body only when a status arrived
//! with it.
//! @authority server

//! How a finished game-runtime request is handled.
enum TBD_EGameRuntimeOutcome
{
	SUCCESS,   //!< 2xx: the body is the answer.
	REFUSED,   //!< 409 carrying a fence code in `details.code`.
	TRANSIENT, //!< No answer, a timeout or a server-side failure: the same request may succeed later.
	PERMANENT, //!< A client-error status: repeating the same request cannot succeed.
}

//! `details` of a refused game-runtime request: `STALE_GENERATION` (with `generation`),
//! `STALE_SEQUENCE` (with `last_sequence`), `RUNTIME_SESSION_ENDED` (with `end_reason`), or a fleet
//! command report's `STALE_FENCING_TOKEN` / `COMMAND_NOT_CLAIMED` / `COMMAND_NOT_EXECUTING` (with the
//! command's `state`). Field names are the JSON keys.
class TBD_GameRuntimeRefusalDetails
{
	string code;
	string end_reason;
	int last_sequence;
	int generation;
	string state;
}

//! The backend error envelope, `{"error": message, "details": {...}}`.
class TBD_GameRuntimeErrorBody
{
	string error;
	ref TBD_GameRuntimeRefusalDetails details;
}

class TBD_GameRuntimeAnswer
{
	//! `Print` drops a line longer than 1024 bytes entirely, so a logged body is capped well below.
	protected static const int LOGGED_BODY_MAX_BYTES = 400;

	TBD_EGameRuntimeOutcome m_eOutcome;
	HttpCode m_eCode;
	//! The response body; empty when no status arrived.
	string m_sBody;
	//! The 409 fence details, or null.
	ref TBD_GameRuntimeRefusalDetails m_Refusal;
	//! `details.code` of an error answer of any status, or empty.
	string m_sErrorCode;
	//! One log line: the status and response body, or the transport result.
	string m_sDetail;

	//------------------------------------------------------------------------------------------------
	//! The answer the engine reported through `callback`. `arrivedOnSuccess` says which RestCallback
	//! handler fired: a success handler that reports no status still delivered its body.
	static TBD_GameRuntimeAnswer Read(notnull RestCallback callback, bool arrivedOnSuccess)
	{
		TBD_GameRuntimeAnswer answer = new TBD_GameRuntimeAnswer();
		answer.m_eCode = callback.GetHttpCode();
		if (answer.m_eCode != HttpCode.HTTP_CODE_NULL || arrivedOnSuccess)
			answer.m_sBody = callback.GetData();

		answer.m_eOutcome = answer.Classify(arrivedOnSuccess);

		// The enum's ordinal is not the HTTP status, so the status goes out by name.
		string status = typename.EnumToString(HttpCode, answer.m_eCode);
		if (answer.m_eCode == HttpCode.HTTP_CODE_NULL)
			answer.m_sDetail = status + " rest=" + typename.EnumToString(ERestResult, callback.GetRestResult());
		else
			answer.m_sDetail = status + " response=" + LoggableBody(answer.m_sBody);

		return answer;
	}

	//------------------------------------------------------------------------------------------------
	//! The answer to a request the engine never reported: `detail` says why.
	static TBD_GameRuntimeAnswer Unanswered(string detail)
	{
		TBD_GameRuntimeAnswer answer = new TBD_GameRuntimeAnswer();
		answer.m_eOutcome = TBD_EGameRuntimeOutcome.TRANSIENT;
		answer.m_eCode = HttpCode.HTTP_CODE_NULL;
		answer.m_sDetail = detail;
		return answer;
	}

	//------------------------------------------------------------------------------------------------
	//! A response body made safe for one log line: capped, and named when empty.
	static string LoggableBody(string body)
	{
		if (body.IsEmpty())
			return "<empty>";

		if (body.Length() > LOGGED_BODY_MAX_BYTES)
			return body.Substring(0, LOGGED_BODY_MAX_BYTES) + "...<truncated>";

		return body;
	}

	//------------------------------------------------------------------------------------------------
	//! Sets m_sErrorCode for an error answer that carries `details.code`, and m_Refusal for such a 409.
	protected TBD_EGameRuntimeOutcome Classify(bool arrivedOnSuccess)
	{
		if (m_eCode == HttpCode.HTTP_CODE_200 || m_eCode == HttpCode.HTTP_CODE_201 || m_eCode == HttpCode.HTTP_CODE_202)
			return TBD_EGameRuntimeOutcome.SUCCESS;

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

	//------------------------------------------------------------------------------------------------
	//! Redirects and client errors other than 409 (classified on its own): the request itself is
	//! wrong for this backend, so sending it again unchanged cannot succeed. 408 (request timeout) is
	//! not among them. Every other status - server errors, gateway failures, anything unrecognised -
	//! is worth another attempt.
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

	//------------------------------------------------------------------------------------------------
	//! The `details` of an error envelope, or null when the body carries no `details.code`.
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
