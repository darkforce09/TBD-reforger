/**
 * TBD_EquipmentResourceNames.c
 *
 * Turns the resource strings a prefab declares into the forms the catalog exports.
 *
 * A prefab reference reaches an extractor in one of two shapes: a `{GUID}path`
 * already resolved by the engine, or a bare `$Addon:Prefabs/...` path that only
 * means something to the addon that declared it. ResolveCanonicalResourceName
 * loads the resource to recover the GUID form; NormalizePathSeparators only
 * squares up separators and is what the attachment domains apply to a value they
 * export as written. The two are separate because they answer separate questions,
 * and the catalogs depend on which one a given field went through.
 */

class TBD_EquipmentResourceNames
{
	//------------------------------------------------------------------------------------------------
	//! Resolve a resource reference to its canonical {GUID}path form.
	//! Returns the input unchanged when the resource does not load or declares no GUID.
	static string ResolveCanonicalResourceName(string resName)
	{
		if (resName.IsEmpty())
			return string.Empty;
		if (resName.StartsWith("{"))
			return resName;
		ResourceName rn = resName;
		Resource res = Resource.Load(rn);
		if (!res || !res.IsValid())
			return resName;
		BaseResourceObject obj = res.GetResource();
		if (!obj)
			return resName;
		BaseContainer root = obj.ToBaseContainer();
		if (!root)
			return resName;
		string crn = root.GetResourceName();
		if (crn.StartsWith("{"))
			return crn;
		return resName;
	}

	//------------------------------------------------------------------------------------------------
	//! Square up path separators on a resource string, leaving the reference itself alone.
	static string NormalizePathSeparators(string raw)
	{
		if (raw.IsEmpty())
			return string.Empty;

		raw.Replace("\\", "/");
		return raw;
	}

	//------------------------------------------------------------------------------------------------
	//! Derive a catalog id from a prefab path: the filename, lowercased, punctuation folded to underscores.
	static string GenerateSlug(string filePath)
	{
		string s = filePath;
		int slash = s.LastIndexOf("/");
		if (slash >= 0)
			s = s.Substring(slash + 1, s.Length() - slash - 1);
		if (s.EndsWith(".et"))
			s = s.Substring(0, s.Length() - 3);
		s.ToLower();
		s.Replace("-", "_");
		s.Replace(" ", "_");
		return s;
	}
}
