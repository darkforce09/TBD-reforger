/**
 * TBD_ExportPaths.c - shared constants + JSON string escaping for the TBD Workbench
 * export plugins (T-130.4 F1-18/F1-20).
 */

//! Shared OS-path constants for exporters whose engine calls need a REAL OS path
//! (MapDataExporter.ExportRasterization, System.MakeScreenshot — neither resolves the
//! Enfusion `$profile:` VFS prefix; FileIO does).
class TBD_ExportPaths
{
	//! Native Proton-prefix location of the Workbench profile dir (Workbench runs under
	//! Proton on this rig, so `$profile:` == this Windows path inside the prefix). Single
	//! source of truth (T-130.4 F1-20) — EnfScript exposes no env-var read in Workbench
	//! plugins, so a different rig/user edits THIS constant (plugins log every attempted
	//! path, so a wrong value fails loudly with the candidates listed).
	static const string PROFILE_WIN = "C:/Users/steamuser/Documents/My Games/ArmaReforgerWorkbench/profile/";
}

//! JSON writing helpers for the exporters' hand-built meta/JSONL payloads.
class TBD_ExportJson
{
	//------------------------------------------------------------------------------------------------
	//! Escape a value for embedding inside a JSON string literal: backslashes, quotes,
	//! newlines/CR/tabs (T-130.4 F1-20). Backslash first so escapes aren't double-escaped.
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
	//! Checked FileHandle.Write: logs an ERROR with the plugin tag on a failed/empty write
	//! so no exporter can leave a silent partial file (T-130.4 F1-18). Empty data is a no-op
	//! success (flush loops legitimately pass "").
	static bool Write(FileHandle f, string data, string logTag)
	{
		if (data.IsEmpty())
			return true;

		int wrote = f.Write(data);
		if (wrote <= 0)
		{
			Print(logTag + " FileHandle.Write failed (wrote=" + wrote.ToString() + ") — aborting export, output file is incomplete.", LogLevel.ERROR);
			return false;
		}
		return true;
	}
}
