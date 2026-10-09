//------------------------------------------------------------------------------------------------
// TBD_UnderbarrelLauncherExtractor.c
//
// Reads the secondary weapon an underbarrel device adds: its own muzzle, the magazine wells it
// feeds from, the projectile it launches, and the zeroing distances its sights offer.
//
// An underbarrel launcher is a weapon mounted on a weapon, so it declares a full muzzle of its own
// rather than modifying the host's. Zeroing distances arrive unordered and duplicated across the
// ancestry, which is why they are collected uniquely and sorted before export.
//------------------------------------------------------------------------------------------------

class TBD_UnderbarrelLauncherExtractor
{
	//------------------------------------------------------------------------------------------------
	//! Extract launcher / secondary weapon system properties directly from BaseMuzzleComponent,
	//! SCR_MuzzleComponent, and SCR_SightsComponent:
	//!   - is_launcher: true if firing muzzle components are present
	//!   - magazine_wells: compatible magazine well class names
	//!   - chamber_capacity: number of rounds chambered or null if unconfigured
	//!   - zeroing_distances: genuine zeroing steps from quadrant/leaf sights or null
	static void ExtractLauncher(map<string, ref array<BaseContainer>> comps, TBD_UnderbarrelLauncherInfo outLauncher)
	{
		bool hasMuzzle = false;
		ref array<BaseContainer> muzzleComps = {};

		// 1. Discover firing muzzle components
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (cls.Contains("MuzzleEffect"))
				continue;

			if (cls.EndsWith("MuzzleComponent") || cls.EndsWith("MuzzleInMagComponent"))
			{
				hasMuzzle = true;
				foreach (BaseContainer mc : bucket)
					muzzleComps.Insert(mc);
			}
		}

		outLauncher.m_bIsLauncher = hasMuzzle;

		// 2. Magazine Wells introspection
		ref array<string> magWells = {};
		foreach (BaseContainer muz : muzzleComps)
		{
			BaseContainer curMuz = muz;
			while (curMuz && magWells.IsEmpty())
			{
				// Single MagazineWell object
				BaseContainer wellObj = curMuz.GetObject("MagazineWell");
				if (wellObj)
				{
					string wCls = wellObj.GetClassName();
					if (!wCls.IsEmpty())
						TBD_EquipmentComponentGraph.AddUniqueType(magWells, wCls);
				}

				// MagazineWells object array
				BaseContainerList wellList = curMuz.GetObjectArray("MagazineWells");
				if (!wellList)
					wellList = curMuz.GetObjectArray("m_aMagazineWells");

				if (wellList)
				{
					for (int wi = 0, win = wellList.Count(); wi < win; wi++)
					{
						BaseContainer wElem = wellList.Get(wi);
						if (wElem)
						{
							string wCls2 = wElem.GetClassName();
							if (!wCls2.IsEmpty())
								TBD_EquipmentComponentGraph.AddUniqueType(magWells, wCls2);
						}
					}
				}

				curMuz = curMuz.GetAncestor();
			}
		}

		// Also check weapon components if magWells is still empty
		if (magWells.IsEmpty())
		{
			foreach (string wCompCls, array<BaseContainer> wCompBucket : comps)
			{
				if (!wCompCls.EndsWith("WeaponComponent"))
					continue;

				foreach (BaseContainer wc : wCompBucket)
				{
					BaseContainer curWc = wc;
					while (curWc && magWells.IsEmpty())
					{
						BaseContainer wellObj2 = curWc.GetObject("MagazineWell");
						if (wellObj2)
						{
							string wCls3 = wellObj2.GetClassName();
							if (!wCls3.IsEmpty())
								TBD_EquipmentComponentGraph.AddUniqueType(magWells, wCls3);
						}

						BaseContainerList wellList3 = curWc.GetObjectArray("MagazineWells");
						if (!wellList3)
							wellList3 = curWc.GetObjectArray("m_aMagazineWells");

						if (wellList3)
						{
							for (int wj = 0, wjn = wellList3.Count(); wj < wjn; wj++)
							{
								BaseContainer wElem2 = wellList3.Get(wj);
								if (wElem2)
								{
									string wCls4 = wElem2.GetClassName();
									if (!wCls4.IsEmpty())
										TBD_EquipmentComponentGraph.AddUniqueType(magWells, wCls4);
								}
							}
						}

						curWc = curWc.GetAncestor();
					}
				}
			}
		}

		outLauncher.m_aMagazineWells = magWells;

		// 3. Chamber Capacity introspection
		// Checks genuine container variables; omitted or unconfigured serializes as null
		foreach (BaseContainer mzc : muzzleComps)
		{
			BaseContainer curCap = mzc;
			while (curCap)
			{
				int cap = 0;
				if (curCap.IsVariableSet("m_iChamberCapacity") && curCap.Get("m_iChamberCapacity", cap))
				{
					outLauncher.m_iChamberCapacity = cap;
					outLauncher.m_bHasChamberCapacity = true;
					break;
				}
				if (curCap.IsVariableSet("ChamberCapacity") && curCap.Get("ChamberCapacity", cap))
				{
					outLauncher.m_iChamberCapacity = cap;
					outLauncher.m_bHasChamberCapacity = true;
					break;
				}
				if (curCap.IsVariableSet("m_iBarrelsCount") && curCap.Get("m_iBarrelsCount", cap))
				{
					outLauncher.m_iChamberCapacity = cap;
					outLauncher.m_bHasChamberCapacity = true;
					break;
				}
				if (curCap.IsVariableSet("BarrelsCount") && curCap.Get("BarrelsCount", cap))
				{
					outLauncher.m_iChamberCapacity = cap;
					outLauncher.m_bHasChamberCapacity = true;
					break;
				}
				if (curCap.IsVariableSet("m_iMaxAmmoCount") && curCap.Get("m_iMaxAmmoCount", cap))
				{
					outLauncher.m_iChamberCapacity = cap;
					outLauncher.m_bHasChamberCapacity = true;
					break;
				}
				if (curCap.IsVariableSet("MaxAmmoCount") && curCap.Get("MaxAmmoCount", cap))
				{
					outLauncher.m_iChamberCapacity = cap;
					outLauncher.m_bHasChamberCapacity = true;
					break;
				}
				if (curCap.IsVariableSet("m_iChamberSize") && curCap.Get("m_iChamberSize", cap))
				{
					outLauncher.m_iChamberCapacity = cap;
					outLauncher.m_bHasChamberCapacity = true;
					break;
				}
				if (curCap.IsVariableSet("ChamberSize") && curCap.Get("ChamberSize", cap))
				{
					outLauncher.m_iChamberCapacity = cap;
					outLauncher.m_bHasChamberCapacity = true;
					break;
				}

				curCap = curCap.GetAncestor();
			}

			if (outLauncher.m_bHasChamberCapacity)
				break;
		}

		// 4. Zeroing Distances from quadrant / leaf sights (SCR_SightsComponent, SightsComponent)
		ref array<int> zeroing = {};
		foreach (string sCls, array<BaseContainer> sBucket : comps)
		{
			if (!sCls.EndsWith("SightsComponent") && !sCls.EndsWith("Sights") && !sCls.EndsWith("OpticsComponent"))
				continue;

			foreach (BaseContainer sc : sBucket)
			{
				BaseContainer curSight = sc;
				while (curSight)
				{
					// Check SightsRanges
					BaseContainerList sightRanges = curSight.GetObjectArray("SightsRanges");
					if (sightRanges)
					{
						for (int r = 0, rn = sightRanges.Count(); r < rn; r++)
						{
							BaseContainer sri = sightRanges.Get(r);
							if (!sri)
								continue;

							vector rVec;
							if (sri.Get("Range", rVec))
							{
								int distM = Math.Round(rVec[1]);
								if (distM > 0)
								{
									AddUniqueInt(zeroing, distM);
									continue;
								}
							}

							string rStr = "";
							if (sri.GetDefaultAsString("Range", rStr) && !rStr.IsEmpty())
							{
								array<string> parts = {};
								rStr.Split(" ", parts, false);
								if (!parts.IsEmpty())
								{
									string lastPart = parts[parts.Count() - 1];
									int distVal = lastPart.ToInt();
									if (distVal > 0)
										AddUniqueInt(zeroing, distVal);
								}
							}
						}
					}

					// Check m_aZeroingDistances array
					array<float> ranges = {};
					if (curSight.Get("m_aZeroingDistances", ranges) && ranges)
					{
						foreach (float fltRange : ranges)
						{
							int rInt = Math.Round(fltRange);
							if (rInt > 0)
								AddUniqueInt(zeroing, rInt);
						}
					}

					// Check single m_fZeroingDistance
					float singleZero;
					if (curSight.Get("m_fZeroingDistance", singleZero) && singleZero > 0)
					{
						int szInt = Math.Round(singleZero);
						if (szInt > 0)
							AddUniqueInt(zeroing, szInt);
					}

					curSight = curSight.GetAncestor();
				}
			}
		}

		// Also check muzzle containers for zeroing distances if still empty
		if (zeroing.IsEmpty())
		{
			foreach (BaseContainer mzc2 : muzzleComps)
			{
				BaseContainer curMuzSight = mzc2;
				while (curMuzSight)
				{
					BaseContainerList mSightRanges = curMuzSight.GetObjectArray("SightsRanges");
					if (mSightRanges)
					{
						for (int mr = 0, mrn = mSightRanges.Count(); mr < mrn; mr++)
						{
							BaseContainer msri = mSightRanges.Get(mr);
							if (!msri)
								continue;

							vector mrVec;
							if (msri.Get("Range", mrVec))
							{
								int mdistM = Math.Round(mrVec[1]);
								if (mdistM > 0)
									AddUniqueInt(zeroing, mdistM);
							}
						}
					}
					curMuzSight = curMuzSight.GetAncestor();
				}
			}
		}

		if (zeroing.Count() > 0)
		{
			SortIntsAscending(zeroing);
			outLauncher.m_aZeroingDistances = zeroing;
			outLauncher.m_bHasZeroingDistances = true;
		}
		else
		{
			outLauncher.m_bHasZeroingDistances = false;
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Append an integer to an array only if it does not already exist.
	protected static void AddUniqueInt(notnull array<int> list, int item)
	{
		if (list.Find(item) == -1)
			list.Insert(item);
	}

	//------------------------------------------------------------------------------------------------
	//! In-place ascending sort for integer arrays.
	protected static void SortIntsAscending(notnull array<int> list)
	{
		int n = list.Count();
		for (int i = 0; i < n - 1; i++)
		{
			for (int j = 0; j < n - i - 1; j++)
			{
				if (list[j] > list[j + 1])
				{
					int tmp = list[j];
					list[j] = list[j + 1];
					list[j + 1] = tmp;
				}
			}
		}
	}
}
