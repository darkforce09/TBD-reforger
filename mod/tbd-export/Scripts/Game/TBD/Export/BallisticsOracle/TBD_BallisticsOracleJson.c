/**
 * @file TBD_BallisticsOracleJson.c
 * @brief JSON text pieces for the ballistics oracle outputs: numbers, strings, vectors, UTC time.
 *
 * Role: formats every value the oracle writes, so numbers and escaping have one definition.
 * Position: called by the forward-angle writer, the simulation component and the output file
 * sidecar.
 * State: none; pure functions.  Invariants: numbers go through the engine JSON writer, which keeps
 * the float's own digits; NaN and infinities become `null`; strings are escaped for JSON.
 */

//! Formats the values of the ballistics oracle outputs as JSON text.
class TBD_BallisticsOracleJson
{
	protected static const float LARGEST_FINITE = 3.0e38; //!< magnitudes above this are written as null

	//! Returns `value` as a JSON number through the engine JSON writer, or `null` when it is NaN,
	//! infinite, or the writer returns no value envelope.
	static string Number(float value)
	{
		if (value != value || Math.AbsFloat(value) > LARGEST_FINITE)
			return "null";

		JsonSaveContainer writer = new JsonSaveContainer();
		SaveContainerContext context = new SaveContainerContext(false);
		context.SetContainer(writer);
		context.WriteValue("v", value);
		string encoded = writer.SaveToString();
		int colon = encoded.IndexOf(":");
		int closingBrace = encoded.LastIndexOf("}");
		if (colon < 0 || closingBrace <= colon + 1)
			return "null";

		string text = encoded.Substring(colon + 1, closingBrace - colon - 1);
		text.TrimInPlace();
		return text;
	}

	//! Returns `value` as a JSON integer.
	static string Integer(int value)
	{
		return value.ToString();
	}

	//! Returns `true` or `false`.
	static string Boolean(bool value)
	{
		if (value)
			return "true";

		return "false";
	}

	//! Returns `text` as a quoted, escaped JSON string.
	static string Quote(string text)
	{
		text.Replace("\\", "\\\\");
		text.Replace("\"", "\\\"");
		text.Replace("\n", "\\n");
		text.Replace("\r", "\\r");
		text.Replace("\t", "\\t");
		return "\"" + text + "\"";
	}

	//! Returns `text` quoted, or `null` when it is empty.
	static string QuoteOrNull(string text)
	{
		if (text.IsEmpty())
			return "null";

		return Quote(text);
	}

	//! Returns the vector as a three-number array in the engine's axis order (x, y, z).
	static string Vector3(vector value)
	{
		return "[" + Number(value[0]) + "," + Number(value[1]) + "," + Number(value[2]) + "]";
	}

	//! Returns the current UTC time as `YYYY-MM-DDTHH:MM:SSZ`.
	static string IsoNowUtc()
	{
		int year, month, day, hour, minute, second;
		System.GetYearMonthDayUTC(year, month, day);
		System.GetHourMinuteSecondUTC(hour, minute, second);
		return string.Format("%1-%2-%3T%4:%5:%6Z", year, Pad2(month), Pad2(day), Pad2(hour), Pad2(minute), Pad2(second));
	}

	//! Returns `value` as at least two decimal digits.
	protected static string Pad2(int value)
	{
		if (value < 10)
			return "0" + value.ToString();

		return value.ToString();
	}
}
