/**
 * TBD_EquipmentComponentGraph.c
 *
 * Flattens a prefab into the set of components it declares.
 *
 * A Workbench prefab reaches its components two ways: directly, through nested
 * `components` arrays, and by inheritance, through its ancestor chain. Reading a
 * property off the prefab alone misses everything an ancestor contributes, so
 * every extractor starts by collecting both into one map keyed by component class
 * name.
 *
 * Both walks are capped so a deep or cyclic prefab cannot hang the export.
 * ANCESTOR_CAP bounds the inheritance chain and is the same for every caller.
 * The nesting cap is not: a per-hardware extractor stops at 4, while the
 * discovery sweep and the M16 compatibility scan need 8 to reach components
 * buried inside nested slots. That makes the nesting depth a property of the scan
 * rather than of the walk, so each caller passes its own and declares it beside
 * the scan it governs.
 */

class TBD_EquipmentComponentGraph
{
	protected static const int ANCESTOR_CAP = 16;

	//------------------------------------------------------------------------------------------------
	//! Collect every component declared by a prefab and by each of its ancestors.
	static void CollectComponentChain(BaseContainer prefabRoot, notnull map<string, ref array<BaseContainer>> outComps, int componentDepthCap)
	{
		BaseContainer cur = prefabRoot;
		int hops = 0;
		while (cur && hops < ANCESTOR_CAP)
		{
			CollectComponentsRec(cur, outComps, 0, componentDepthCap);
			cur = cur.GetAncestor();
			hops++;
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Collect one container's components, descending into nested component arrays.
	protected static void CollectComponentsRec(BaseContainer holder, notnull map<string, ref array<BaseContainer>> outComps, int depth, int componentDepthCap)
	{
		if (depth > componentDepthCap)
			return;

		BaseContainerList comps = holder.GetObjectArray("components");
		if (!comps)
			return;

		for (int i = 0, n = comps.Count(); i < n; i++)
		{
			BaseContainer comp = comps.Get(i);
			if (!comp)
				continue;

			string cls = comp.GetClassName();
			array<BaseContainer> bucket = outComps.Get(cls);
			if (!bucket)
			{
				bucket = {};
				outComps.Insert(cls, bucket);
			}
			bucket.Insert(comp);

			CollectComponentsRec(comp, outComps, depth + 1, componentDepthCap);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Test whether the collected set holds a component whose class name ends with a suffix.
	static bool HasCompSuffix(map<string, ref array<BaseContainer>> comps, string suffix)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (cls.EndsWith(suffix))
				return true;
		}
		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! Append a class name to an accumulator, skipping empties and repeats.
	static void AddUniqueType(notnull array<string> list, string item)
	{
		if (item.IsEmpty())
			return;

		if (list.Find(item) == -1)
			list.Insert(item);
	}
}
