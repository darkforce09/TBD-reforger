/**
 * @file TBD_WireCodec.c
 * @brief The tab-and-newline record codec the lobby, briefing and admin snapshots ship over RPC.
 *
 * Role: builds and reads the one-string payload format: records joined by `LINE_SEP`, each a bare
 * kind token followed by fields written as `<FIELD_SEP><FIELD_MARK><value>`.  Position: called
 * by the lobby, briefing and admin snapshot services on the server (build) and their clients
 * (parse).
 * State: none.  Invariants: the marker makes every field a non-empty token, so
 * `Unmark(Field(x)) == Sanitise(x)` for every `x`, the empty string included; `Sanitise` is
 * idempotent; `Join` never emits more than `maxLines` records and warns when it clips.
 */

//! Stateless record codec. `string.Replace` mutates its receiver in place and returns a count, so
//! every replacement below is a statement on a copy.
class TBD_WireCodec
{
	static const string FIELD_SEP = "\t"; //!< separates the fields of one record
	static const string LINE_SEP = "\n"; //!< separates records
	static const string FIELD_MARK = "."; //!< prefixes every field value so no token is empty

	//! One field: separator, marker, value.
	//! @param value the field text
	//! @param sanitise true strips separators from `value` first (lobby and briefing); false writes
	//! it as given (admin snapshot, whose rows are sanitised when built)
	//! @return the encoded field, never empty
	static string Field(string value, bool sanitise = true)
	{
		if (sanitise)
			return FIELD_SEP + FIELD_MARK + Sanitise(value);

		return FIELD_SEP + FIELD_MARK + value;
	}

	//! Strip the marker off a parsed field. A token of one character or less reads as empty rather
	//! than as an error, so a truncated wire renders what it can.
	//! @return the field value
	static string Unmark(string field)
	{
		int length = field.Length();
		if (length <= 1)
			return string.Empty;

		return field.Substring(1, length - 1);
	}

	//! A marked boolean. Anything but a marked `1` reads false, so a corrupt token fails safe.
	static bool IsSet(string field)
	{
		return Unmark(field) == "1";
	}

	//! `1` or `0`, the boolean field text `IsSet` reads back.
	static string Flag(bool value)
	{
		if (value)
			return "1";

		return "0";
	}

	//! Replace the field separator, the line separator and carriage returns with spaces, so
	//! authored text or a player name cannot shift the fields of its record.
	//! @return a sanitised copy; the empty string comes back unchanged
	static string Sanitise(string value)
	{
		if (value.IsEmpty())
			return value;

		string clean = value;
		clean.Replace(FIELD_SEP, " ");
		clean.Replace(LINE_SEP, " ");
		clean.Replace("\r", " ");
		return clean;
	}

	//! Join records with `LINE_SEP`, keeping at most `maxLines` of them.
	//! @param lines the records, in order
	//! @param maxLines the most records written
	//! @param channel the `TBD_Log` channel for the clip warning
	//! @param clipWarningFormat the warning text, with `%1` replaced by `maxLines`
	//! @return the joined payload; a clipped payload logs one warning
	static string Join(notnull array<string> lines, int maxLines, string channel, string clipWarningFormat)
	{
		int shown = lines.Count();
		bool clipped = false;
		if (shown > maxLines)
		{
			shown = maxLines;
			clipped = true;
		}

		string result;
		for (int i = 0; i < shown; i++)
		{
			if (i > 0)
				result = result + LINE_SEP;

			result = result + lines[i];
		}

		if (clipped)
			TBD_Log.Warn(channel, string.Format(clipWarningFormat, maxLines));

		return result;
	}

	//! A record of kind `kind` with one field. The kind token is written bare: it is never empty
	//! and never authored. `sanitise` is passed to `Field`.
	static string Record1(string kind, string a, bool sanitise = true)
	{
		return kind + Field(a, sanitise);
	}

	//! A record with two fields; see `Record1`.
	static string Record2(string kind, string a, string b, bool sanitise = true)
	{
		return kind + Field(a, sanitise) + Field(b, sanitise);
	}

	//! A record with three fields; see `Record1`.
	static string Record3(string kind, string a, string b, string c, bool sanitise = true)
	{
		string line = kind + Field(a, sanitise) + Field(b, sanitise);
		return line + Field(c, sanitise);
	}

	//! A record with four fields; see `Record1`. Appended in steps: a long `+` chain trips the
	//! engine's `Formula too complex` ceiling.
	static string Record4(string kind, string a, string b, string c, string d, bool sanitise = true)
	{
		string line = kind + Field(a, sanitise) + Field(b, sanitise);
		return line + Field(c, sanitise) + Field(d, sanitise);
	}

	//! A record of `fields` sanitised fields taken from `a` to `e` in order; the rest are not
	//! written, so a record is never padded.
	//! @param fields how many of `a` to `e` are real, 0 to 5
	//! @return the encoded record
	static string Record(int fields, string kind, string a, string b, string c, string d, string e)
	{
		string line = kind;

		if (fields >= 1)
			line = line + Field(a);

		if (fields >= 2)
			line = line + Field(b);

		if (fields >= 3)
			line = line + Field(c);

		if (fields >= 4)
			line = line + Field(d);

		if (fields >= 5)
			line = line + Field(e);

		return line;
	}
}
