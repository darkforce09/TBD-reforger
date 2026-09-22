/**
 * TBD_EquipmentExportJson.c
 *
 * JSON string escaping, checked file writes, and UTC timestamps. Every catalog
 * and every _meta.json sidecar the exporter writes is serialized through this
 * class, so escaping and the generation timestamp have exactly one definition.
 */

class TBD_EquipmentExportJson
{
	//------------------------------------------------------------------------------------------------
	//! Escape special characters for valid JSON strings.
	static string Escape(string s)
	{
		s.Replace("\\", "\\\\");
		s.Replace("\"", "\\\"");
		s.Replace("\n", "\\n");
		s.Replace("\r", "\\r");
		s.Replace("\t", "\\t");
		return s;
	}

	//------------------------------------------------------------------------------------------------
	//! Checked write helper that logs an error and aborts on failure.
	static bool Write(FileHandle f, string data, string logTag)
	{
		if (data.IsEmpty())
			return true;

		int wrote = f.Write(data);
		if (wrote <= 0)
		{
			Print(string.Format("%1 FileHandle.Write failed (wrote=%2) - aborting write.", logTag, wrote), LogLevel.ERROR);
			return false;
		}
		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! Helper for formatting current UTC time in ISO 8601 format (YYYY-MM-DDTHH:MM:SSZ).
	static string IsoNowUtc()
	{
		int y, mo, d, h, mi, s;
		System.GetYearMonthDayUTC(y, mo, d);
		System.GetHourMinuteSecondUTC(h, mi, s);
		return string.Format("%1-%2-%3T%4:%5:%6Z", y, Pad2(mo), Pad2(d), Pad2(h), Pad2(mi), Pad2(s));
	}

	//------------------------------------------------------------------------------------------------
	static string Pad2(int v)
	{
		if (v < 10)
			return "0" + v.ToString();
		return v.ToString();
	}
}
