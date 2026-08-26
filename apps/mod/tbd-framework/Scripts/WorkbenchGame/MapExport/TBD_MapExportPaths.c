/**
 * TBD_MapExportPaths.c
 *
 * Shared path normalization, native OS path resolution, and JSON serialization
 * helpers for the TBD Workbench Map Data Exporter.
 */

class TBD_MapExportPaths
{
	//! Native Proton-prefix location of the Workbench profile dir.
	//! MapDataExporter and System.MakeScreenshot require native OS paths rather than VFS $profile: prefixes.
	static const string PROFILE_WIN = "C:/Users/steamuser/Documents/My Games/ArmaReforgerWorkbench/profile/";

	//------------------------------------------------------------------------------------------------
	//! Normalize a directory path to ensure trailing slash and clean slashes.
	static string NormalizeDirPath(string dir)
	{
		if (dir.IsEmpty())
			return "$profile:TBD_Export/";

		dir.Replace("\\", "/");
		if (!dir.EndsWith("/"))
			dir += "/";
		return dir;
	}

	//------------------------------------------------------------------------------------------------
	//! Ensure the destination directory exists before creating files.
	static void EnsureDestinationDir(string dir)
	{
		string normDir = NormalizeDirPath(dir);
		if (normDir.StartsWith("$profile:"))
		{
			string sub = normDir;
			if (sub.EndsWith("/"))
				sub = sub.Substring(0, sub.Length() - 1);
			FileIO.MakeDirectory(sub);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Resolve a filename against a destination directory.
	static string BuildPath(string dir, string filename)
	{
		EnsureDestinationDir(dir);
		string normDir = NormalizeDirPath(dir);
		return normDir + filename;
	}

	//------------------------------------------------------------------------------------------------
	//! Resolve a destination file to a native Windows OS path (needed for MapDataExporter / MakeScreenshot).
	static string ResolveNativeOsPath(string dir, string filename)
	{
		string normDir = NormalizeDirPath(dir);
		if (normDir.StartsWith("$profile:"))
		{
			string sub = normDir.Substring(9, normDir.Length() - 9); // strip "$profile:"
			if (sub.StartsWith("/"))
				sub = sub.Substring(1, sub.Length() - 1);
			return PROFILE_WIN + sub + filename;
		}
		return normDir + filename;
	}
}

//! JSON writing and string escaping helpers.
class TBD_MapExportJson
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
			Print(logTag + " FileHandle.Write failed (wrote=" + wrote.ToString() + ") — aborting export.", LogLevel.ERROR);
			return false;
		}
		return true;
	}
}

//! Backwards compatibility aliases for non-map callers (e.g. TBD_RegistryItemsExportPlugin).
class TBD_ExportPaths : TBD_MapExportPaths {}
class TBD_ExportJson : TBD_MapExportJson {}
