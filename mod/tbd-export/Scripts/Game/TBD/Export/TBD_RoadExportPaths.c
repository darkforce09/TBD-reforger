/**
 * TBD_ExportPaths.c
 *
 * Path normalization and directory management helpers for tbd-export runtime mod.
 * Resolves $profile:TBD_Export/<mapName>/... destinations.
 */

class TBD_RoadExportPaths
{
	//------------------------------------------------------------------------------------------------
	//! Normalize a directory path to ensure forward slashes and a trailing slash.
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
	//! Recursively ensure every directory segment in a destination path exists.
	static void EnsureDirRecursive(string dir)
	{
		string normDir = NormalizeDirPath(dir);
		if (normDir.StartsWith("$profile:"))
		{
			string rel = normDir.Substring(9, normDir.Length() - 9);
			array<string> parts = {};
			rel.Split("/", parts, false);
			string current = "$profile:";
			for (int i = 0; i < parts.Count(); i++)
			{
				string p = parts[i];
				if (p.IsEmpty())
					continue;
				if (current != "$profile:")
					current += "/";
				current += p;
				FileIO.MakeDirectory(current);
			}
		}
		else
		{
			array<string> nonProfileParts = {};
			normDir.Split("/", nonProfileParts, false);
			string curPath = "";
			for (int j = 0; j < nonProfileParts.Count(); j++)
			{
				string np = nonProfileParts[j];
				if (np.IsEmpty())
				{
					if (j == 0) curPath = "/";
					continue;
				}
				if (!curPath.IsEmpty() && !curPath.EndsWith("/"))
					curPath += "/";
				curPath += np;
				FileIO.MakeDirectory(curPath);
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Build canonical directory path for a map category (e.g. $profile:TBD_Export/everon/roads/).
	static string GetCategoryDir(string baseDir, string mapName, string category = "")
	{
		string normBase = NormalizeDirPath(baseDir);
		string cleanMap = mapName;
		cleanMap.ToLower();
		cleanMap.Trim();
		if (cleanMap.IsEmpty())
			cleanMap = "everon";

		string cleanCat = category;
		cleanCat.ToLower();
		cleanCat.Trim();
		if (cleanCat.IsEmpty())
			return normBase + cleanMap + "/";

		return normBase + cleanMap + "/" + cleanCat + "/";
	}

	//------------------------------------------------------------------------------------------------
	//! Resolve a filename against a scoped map category directory.
	static string BuildCategoryPath(string baseDir, string mapName, string category, string filename)
	{
		string catDir = GetCategoryDir(baseDir, mapName, category);
		EnsureDirRecursive(catDir);
		return catDir + filename;
	}
}
