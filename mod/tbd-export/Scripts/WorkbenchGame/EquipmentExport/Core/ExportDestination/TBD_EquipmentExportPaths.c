/**
 * TBD_EquipmentExportPaths.c
 *
 * Path normalization and recursive directory creation for equipment export
 * destinations. Every catalog file written under $profile:TBD_Export/equipment/
 * resolves its path through this class.
 */

class TBD_EquipmentExportPaths
{
	//------------------------------------------------------------------------------------------------
	//! Normalize a directory path to ensure trailing slash and uniform forward slashes.
	static string NormalizeDirPath(string dir)
	{
		if (dir.IsEmpty())
			return "$profile:TBD_Export/equipment/";

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
			string rel = normDir.Substring(9, normDir.Length() - 9); // strip "$profile:"
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
	//! Ensure the destination directory exists before creating files.
	static void EnsureDestinationDir(string dir)
	{
		EnsureDirRecursive(dir);
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
	//! Resolve a filename against a category subfolder (e.g. $profile:TBD_Export/equipment/weapons/rifles.json).
	static string BuildCategoryPath(string baseDir, string category, string filename)
	{
		string normBase = NormalizeDirPath(baseDir);
		string cleanCat = category;
		cleanCat.ToLower();
		cleanCat.Trim();
		if (cleanCat.StartsWith("/"))
			cleanCat = cleanCat.Substring(1, cleanCat.Length() - 1);
		if (cleanCat.EndsWith("/"))
			cleanCat = cleanCat.Substring(0, cleanCat.Length() - 1);

		string catDir = normBase;
		if (!cleanCat.IsEmpty())
			catDir = normBase + cleanCat + "/";

		EnsureDirRecursive(catDir);
		return catDir + filename;
	}
}
