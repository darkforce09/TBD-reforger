/**
 * @file TBD_BackendText.c
 * @brief Text helpers for backend payloads and backend log lines.
 *
 * Role: JSON string escaping, UUID recognition, RFC 3339 UTC timestamps, two-digit padding, and
 * the backend description printed in armed lines.  Position: called by the results reporter, the
 * match telemetry reports, the identity link, the runtime status readings and the game-runtime
 * HTTP client; reads `TBD_BackendConfig` and the engine's UTC clock.
 * State: none.  Invariants: `DescribeBackend` never prints the credential; `JsonEscape` never mutates
 * the caller's string and always yields a value safe inside a JSON double-quoted scalar.
 */

//! Stateless backend text helpers.
class TBD_BackendText
{
	//! The backend for a log line with the machine credential's state, never the credential.
	//! @param noneText what to print when no backend URL is configured (`none` or `(none)`)
	//! @return `noneText`, or `TBD_GameRuntimeHttp.DescribeBackend`
	static string DescribeBackend(string noneText = "none")
	{
		if (TBD_BackendConfig.GetBackendUrl().IsEmpty())
			return noneText;

		return TBD_GameRuntimeHttp.DescribeBackend();
	}

	//! The current UTC time as RFC 3339, for example `2026-07-25T16:31:28Z`, the shape the
	//! backend's `DateTime<Utc>` parses. Date and clock are two engine reads, so a call that
	//! straddles midnight can pair the next date with `23:59:59`.
	//! @return the timestamp; never fails
	static string UtcNowIso8601()
	{
		int year, month, day;
		int hour, minute, second;
		System.GetYearMonthDayUTC(year, month, day);
		System.GetHourMinuteSecondUTC(hour, minute, second);

		string date = string.Format("%1-%2-%3", year, Pad2(month), Pad2(day));
		string time = string.Format("%1:%2:%3", Pad2(hour), Pad2(minute), Pad2(second));
		return date + "T" + time + "Z";
	}

	//! `value` zero-padded to two digits; `string.Format` has no width specifier.
	//! @return `0<value>` below 10, else the plain number
	static string Pad2(int value)
	{
		if (value < 10)
			return string.Format("0%1", value);

		return string.Format("%1", value);
	}

	//! Whether `value` is a canonical UUID: 36 characters, hyphens at 8, 13, 18 and 23, hex digits
	//! elsewhere. Optional wire fields typed `format: uuid` are sent only when this holds.
	//! @return true for a UUID
	static bool IsUuid(string value)
	{
		if (value.Length() != 36)
			return false;

		for (int i = 0; i < 36; i++)
		{
			int code = value.ToAscii(i);
			if (i == 8 || i == 13 || i == 18 || i == 23)
			{
				if (code != 45)
					return false;

				continue;
			}

			bool digit = code >= 48 && code <= 57;
			bool lowerHex = code >= 97 && code <= 102;
			bool upperHex = code >= 65 && code <= 70;
			if (!digit && !lowerHex && !upperHex)
				return false;
		}

		return true;
	}

	//! A copy of `value` safe inside a JSON double-quoted scalar. Backslashes are escaped first so
	//! the quote escapes are not escaped again; newline, carriage return and tab become spaces so
	//! the payload stays one log line. The copy comes from `string.Format` because
	//! `string.Replace` mutates its receiver in place.
	//! @return the escaped copy
	static string JsonEscape(string value)
	{
		string escaped = string.Format("%1", value);
		escaped.Replace("\\", "\\\\");
		escaped.Replace("\"", "\\\"");
		escaped.Replace("\n", " ");
		escaped.Replace("\r", " ");
		escaped.Replace("\t", " ");
		return escaped;
	}
}
