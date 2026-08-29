/**
 * TBD_ExportJson.c
 *
 * High-performance JSON escaping and stream-writing utilities for tbd-export.
 */

class TBD_RoadExportJson
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
	//! Checked write helper that logs an error and returns false on failure.
	static bool Write(FileHandle f, string data, string logTag = "[TBD-EXPORT]")
	{
		if (data.IsEmpty())
			return true;

		int wrote = f.Write(data);
		if (wrote <= 0)
		{
			Print(string.Format("%1 FileHandle.Write failed (wrote=%2)", logTag, wrote), LogLevel.ERROR);
			return false;
		}
		return true;
	}
}
