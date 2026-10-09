/**
 * @file TBD_BriefingText.c
 * @brief Byte-safe text helpers the briefing builder uses on authored strings.
 *
 * Role: splits orders into paragraphs, trims spaces, clips at a word boundary, shortens resource
 * names and humanises snake_case keys.  Position: called by TBD_BriefingService on the server.
 * State: none; pure functions.  Invariants: `string.Length()` counts bytes and `Substring` is
 * byte-indexed, so every cut lands on a space or a byte offset the caller chose; `SplitLines` turns
 * N newlines into exactly N+1 parts, empty parts included, without depending on the native
 * `string.Split`; `string.Replace` and `ToUpper` mutate in place and return a count, never the
 * new string.
 */

//! Static text helpers for the briefing builder.
class TBD_BriefingText
{
	protected static const int MAX_LINE_SCAN = 512; //!< newlines scanned per field before `SplitLines` stops

	//! Break `raw` on newlines with `IndexOf` and `Substring` rather than `string.Split`, so the
	//! part count is fixed by this code: N newlines yield N+1 parts, empty ones included. Each pass
	//! removes at least one character; MAX_LINE_SCAN is a backstop.
	//! @param raw the text to split
	//! @return the parts in order; the caller drops blank ones
	static array<string> SplitLines(string raw)
	{
		array<string> parts = {};
		string rest = raw;

		for (int guard = 0; guard < MAX_LINE_SCAN; guard++)
		{
			int nl = rest.IndexOf(TBD_WireCodec.LINE_SEP);
			if (nl < 0)
			{
				parts.Insert(rest);
				return parts;
			}

			parts.Insert(rest.Substring(0, nl));
			rest = rest.Substring(nl + 1, rest.Length() - nl - 1);
		}

		return parts;
	}

	//! Strip leading and trailing spaces. It runs after TBD_WireCodec.Sanitise has turned tabs and
	//! carriage returns into spaces, so testing for the space character alone is sufficient.
	//! @param value the text to trim
	//! @return `value` without leading or trailing spaces; empty when it holds only spaces
	static string TrimSpaces(string value)
	{
		int length = value.Length();

		int first = 0;
		while (first < length && value.Substring(first, 1) == " ")
		{
			first++;
		}

		int last = length - 1;
		while (last >= first && value.Substring(last, 1) == " ")
		{
			last--;
		}

		if (last < first)
			return string.Empty;

		return value.Substring(first, last - first + 1);
	}

	//! Cut `value` to at most `limit` bytes, backing off to the last space. A space byte never sits
	//! inside a multi-byte UTF-8 sequence, so that cut always lands on a character boundary. With no
	//! space within the limit the cut is blind and can split a multi-byte character.
	//! @param value the text to clip
	//! @param limit the byte limit
	//! @return `value` when it fits, else the clipped head; empty when `limit` is 0 or less
	static string ClipToWord(string value, int limit)
	{
		if (limit <= 0)
			return string.Empty;

		if (value.Length() <= limit)
			return value;

		string head = value.Substring(0, limit);

		int lastSpace = head.LastIndexOf(" ");
		if (lastSpace > 0)
			return head.Substring(0, lastSpace);

		return head;
	}

	//! The short name of a resource: `{ABC123}Prefabs/Weapons/Rifles/M4A1.et` gives `M4A1`.
	//! @param resource the resource name
	//! @return the sanitised name without GUID, folders or extension; the sanitised input when that
	//! leaves nothing
	static string PrettyResourceName(string resource)
	{
		string s = TBD_WireCodec.Sanitise(resource);

		int close = s.IndexOf("}");
		if (close >= 0)
			s = s.Substring(close + 1, s.Length() - close - 1);

		int slash = s.LastIndexOf("/");
		if (slash >= 0)
			s = s.Substring(slash + 1, s.Length() - slash - 1);

		int dot = s.LastIndexOf(".");
		if (dot > 0)
			s = s.Substring(0, dot);

		if (s.IsEmpty())
			return TBD_WireCodec.Sanitise(resource);

		return s;
	}

	//! A snake_case key as prose: `objective_capture` gives `Objective capture`.
	//! @param key the key to humanise
	//! @return the sanitised key with underscores as spaces and the first byte upper-cased
	static string Humanise(string key)
	{
		if (key.IsEmpty())
			return key;

		string s = TBD_WireCodec.Sanitise(key);
		s.Replace("_", " "); // in place; returns a count

		string head = s.Substring(0, 1);
		head.ToUpper(); // in place, like Replace
		string tail = s.Substring(1, s.Length() - 1);
		return head + tail;
	}
}
