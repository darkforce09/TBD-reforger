//------------------------------------------------------------------------------------------------
// TBD_WeaponExtractor.c
//
// Core introspection engine for extracting complete, deep component graphs from any weapon prefab.
// Extracts classification, physical dimensions, sights zeroing, ballistics, muzzles, fire modes,
// attachment slots, bone pivots, and obstruction rules across arbitrary ancestor hierarchies.
//------------------------------------------------------------------------------------------------

class TBD_WeaponExtractor
{
	protected static const int ANCESTOR_CAP = 16;
	protected static const int COMPONENT_DEPTH_CAP = 4;

	//------------------------------------------------------------------------------------------------
	//! Extract all components across the entire inheritance hierarchy into a mapped bucket list.
	static void CollectComponentChain(BaseContainer prefabRoot, notnull map<string, ref array<BaseContainer>> outComps)
	{
		BaseContainer cur = prefabRoot;
		int hops = 0;
		while (cur && hops < ANCESTOR_CAP)
		{
			CollectComponentsRec(cur, outComps, 0);
			cur = cur.GetAncestor();
			hops++;
		}
	}

	//------------------------------------------------------------------------------------------------
	protected static void CollectComponentsRec(BaseContainer holder, notnull map<string, ref array<BaseContainer>> outComps, int depth)
	{
		if (depth > COMPONENT_DEPTH_CAP)
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

			CollectComponentsRec(comp, outComps, depth + 1);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Returns true if any component in the map ends with the given suffix.
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
	//! Extract classification: weapon type and equip slot.
	static void ExtractClassification(map<string, ref array<BaseContainer>> comps, TBD_WeaponClassificationInfo outClass, string category)
	{
		string weaponType = "";
		string slotType = "";

		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("WeaponComponent"))
				continue;

			foreach (BaseContainer wc : bucket)
			{
				BaseContainer cur = wc;
				while (cur)
				{
					if (weaponType.IsEmpty())
					{
						int wtInt = -1;
						if (cur.Get("WeaponType", wtInt) && wtInt >= 0)
							weaponType = WeaponTypeIntToString(wtInt);
						else
							cur.GetDefaultAsString("WeaponType", weaponType);
					}

					if (slotType.IsEmpty())
					{
						cur.Get("WeaponSlotType", slotType);
						if (slotType.IsEmpty())
							cur.GetDefaultAsString("WeaponSlotType", slotType);
					}

					cur = cur.GetAncestor();
				}
			}
		}

		// Normalize weaponType
		if (weaponType.StartsWith("WT_"))
			weaponType = weaponType.Substring(3, weaponType.Length() - 3);

		if (weaponType.IsEmpty())
		{
			if (category == "rifles") weaponType = "Rifle";
			else if (category == "machine_guns") weaponType = "MachineGun";
			else if (category == "handguns") weaponType = "Handgun";
			else if (category == "launchers") weaponType = "RocketLauncher";
			else if (category == "grenades") weaponType = "FragGrenade";
			else if (category == "explosives") weaponType = "Mine";
			else if (category == "underbarrel") weaponType = "GrenadeLauncher";
			else weaponType = "Weapon";
		}

		// Normalize slotType
		if (slotType.IsEmpty())
		{
			if (category == "handguns") slotType = "secondary";
			else if (category == "launchers") slotType = "launcher";
			else if (category == "grenades") slotType = "grenade";
			else if (category == "explosives") slotType = "item";
			else if (category == "underbarrel") slotType = "attachment";
			else slotType = "primary";
		}

		outClass.m_sWeaponType = weaponType;
		outClass.m_sWeaponSlotType = slotType;
	}

	//------------------------------------------------------------------------------------------------
	protected static string WeaponTypeIntToString(int wt)
	{
		switch (wt)
		{
			case 1: return "Rifle";
			case 2: return "GrenadeLauncher";
			case 3: return "SniperRifle";
			case 4: return "RocketLauncher";
			case 5: return "MachineGun";
			case 6: return "Handgun";
			case 7: return "FragGrenade";
			case 8: return "SmokeGrenade";
			case 9: return "Autocannon";
		}
		return "";
	}

	//------------------------------------------------------------------------------------------------
	//! Recursively inspects a container's properties, sub-objects, and arrays for bipod definitions.
	//! Detects bipod stabilization points, deployment points, and bone references (e.g. w_bipodleg).
	protected static bool ContainerContainsBipod(BaseContainer cont, int depth = 0)
	{
		if (!cont || depth > 3)
			return false;

		int nv = cont.GetNumVars();
		for (int v = 0; v < nv; v++)
		{
			string varName = cont.GetVarName(v);
			if (varName.IsEmpty())
				continue;

			string lower = varName;
			lower.ToLower();
			if (lower.Contains("bipod"))
			{
				bool bVal = false;
				if (cont.Get(varName, bVal) && bVal)
					return true;
				string sVal = "";
				if (cont.Get(varName, sVal) && !sVal.IsEmpty())
					return true;
			}

			// Sub-object
			BaseContainer subObj = cont.GetObject(varName);
			if (subObj)
			{
				string subCls = subObj.GetClassName();
				string subClsLower = subCls;
				subClsLower.ToLower();
				if (subClsLower.Contains("bipod"))
					return true;
				if (ContainerContainsBipod(subObj, depth + 1))
					return true;
			}

			// Sub-object array
			BaseContainerList list = cont.GetObjectArray(varName);
			if (list)
			{
				for (int i = 0, n = list.Count(); i < n; i++)
				{
					BaseContainer elem = list.Get(i);
					if (!elem)
						continue;
					string elemCls = elem.GetClassName();
					string elemClsLower = elemCls;
					elemClsLower.ToLower();
					if (elemClsLower.Contains("bipod"))
						return true;
					int elemNv = elem.GetNumVars();
					for (int ev = 0; ev < elemNv; ev++)
					{
						string eVar = elem.GetVarName(ev);
						string strVal;
						if (elem.Get(eVar, strVal) && !strVal.IsEmpty())
						{
							string strLower = strVal;
							strLower.ToLower();
							if (strLower.Contains("bipod"))
								return true;
						}
					}
					if (ContainerContainsBipod(elem, depth + 1))
						return true;
				}
			}
		}
		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! Returns true if the class name represents a true weapon firing muzzle component (not visual effects).
	static bool IsMuzzleComponentClass(string cls)
	{
		if (cls.Contains("MuzzleEffect"))
			return false;
		return cls.EndsWith("MuzzleComponent") || cls.EndsWith("MuzzleInMagComponent");
	}

	//------------------------------------------------------------------------------------------------
	//! Extract physical attributes: weight, volume, dimensions, inventory layout size, bipod, disposable.
	static void ExtractPhysical(map<string, ref array<BaseContainer>> comps, TBD_WeaponPhysicalInfo outPhys)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("InventoryItemComponent") && !cls.EndsWith("StorageComponent"))
				continue;

			foreach (BaseContainer c : bucket)
			{
				BaseContainer attrs = c.GetObject("Attributes");
				if (!attrs)
					continue;

				// Inventory size slot (e.g. SLOT_1x1, SLOT_2x4)
				if (outPhys.m_sInventorySize.IsEmpty())
				{
					string sz;
					if (attrs.Get("m_Size", sz) && !sz.IsEmpty())
						outPhys.m_sInventorySize = sz;
					else
					{
						attrs.GetDefaultAsString("m_Size", sz);
						if (!sz.IsEmpty())
							outPhys.m_sInventorySize = sz;
					}
				}

				BaseContainer phys = attrs.GetObject("ItemPhysAttributes");
				if (phys)
				{
					BaseContainer curPhys = phys;
					while (curPhys)
					{
						if (outPhys.m_fWeightKg < 0)
						{
							float w;
							if (curPhys.Get("Weight", w) && w >= 0)
								outPhys.m_fWeightKg = w;
						}

						if (outPhys.m_fVolumeCm3 < 0)
						{
							float v;
							if (curPhys.Get("ItemVolume", v) && v >= 0)
								outPhys.m_fVolumeCm3 = v;
						}

						if (outPhys.m_vDimensions == "0 0 0")
						{
							vector dims;
							if (curPhys.Get("ItemDimensions", dims) && dims != "0 0 0")
								outPhys.m_vDimensions = dims;
						}

						curPhys = curPhys.GetAncestor();
					}
				}
			}
		}

		// Fallback for grenades / launchers that specify Mass on RigidBody or GrenadeMoveComponent
		if (outPhys.m_fWeightKg < 0)
		{
			array<BaseContainer> rbBucket = comps.Get("RigidBody");
			if (rbBucket)
			{
				foreach (BaseContainer rb : rbBucket)
				{
					float mass;
					if (rb.Get("Mass", mass) && mass > 0)
					{
						outPhys.m_fWeightKg = mass;
						break;
					}
				}
			}
		}

		// Bipod check
		outPhys.m_bHasBipod = HasCompSuffix(comps, "BipodComponent");
		if (!outPhys.m_bHasBipod)
		{
			foreach (string wCls, array<BaseContainer> wBucket : comps)
			{
				if (!wCls.EndsWith("WeaponComponent"))
					continue;
				foreach (BaseContainer wc : wBucket)
				{
					bool hasBip = false;
					if (wc.Get("m_bHasBipod", hasBip) && hasBip)
					{
						outPhys.m_bHasBipod = true;
						break;
					}
					if (ContainerContainsBipod(wc, 0))
					{
						outPhys.m_bHasBipod = true;
						break;
					}
				}
				if (outPhys.m_bHasBipod)
					break;
			}
		}

		// Disposable launcher check
		outPhys.m_bIsDisposable = HasCompSuffix(comps, "SCR_DisposableWeaponComponent");
		if (!outPhys.m_bIsDisposable)
		{
			foreach (string mCls, array<BaseContainer> mBucket : comps)
			{
				if (!IsMuzzleComponentClass(mCls))
					continue;
				foreach (BaseContainer mc : mBucket)
				{
					bool disp = false;
					if (mc.Get("Disposable", disp) && disp)
					{
						outPhys.m_bIsDisposable = true;
						break;
					}
					int canReload = 1;
					if (mc.Get("m_bCanBeReloaded", canReload) && canReload == 0)
					{
						outPhys.m_bIsDisposable = true;
						break;
					}
				}
				if (outPhys.m_bIsDisposable)
					break;
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract sights, zeroing distances, default zeroing index, and sight alignment pivots.
	static void ExtractSights(map<string, ref array<BaseContainer>> comps, TBD_WeaponSightsInfo outSights)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("SightsComponent"))
				continue;

			if (outSights.m_sSightsType.IsEmpty())
				outSights.m_sSightsType = cls;

			foreach (BaseContainer sc : bucket)
			{
				BaseContainer cur = sc;
				while (cur)
				{
					// Zeroing distances from SightsRanges
					if (outSights.m_aZeroingDistances.IsEmpty())
					{
						BaseContainerList sightRanges = cur.GetObjectArray("SightsRanges");
						if (sightRanges)
						{
							for (int i = 0, n = sightRanges.Count(); i < n; i++)
							{
								BaseContainer sri = sightRanges.Get(i);
								if (!sri)
									continue;

								// Method 1: Range is stored as vector2 / vector (elevation, distance_meters)
								vector rVec;
								if (sri.Get("Range", rVec))
								{
									int distMeters = Math.Round(rVec[1]);
									if (distMeters > 0)
									{
										outSights.m_aZeroingDistances.Insert(distMeters.ToString() + "m");
										continue;
									}
								}

								// Method 2: GetDefaultAsString fallback
								string rStr = "";
								if (sri.GetDefaultAsString("Range", rStr) && !rStr.IsEmpty())
								{
									array<string> parts = {};
									rStr.Split(" ", parts, false);
									if (!parts.IsEmpty())
									{
										string lastPart = parts[parts.Count() - 1];
										int dist = lastPart.ToInt();
										if (dist > 0)
											outSights.m_aZeroingDistances.Insert(dist.ToString() + "m");
										else if (!lastPart.IsEmpty())
											outSights.m_aZeroingDistances.Insert(lastPart + "m");
									}
								}
							}
						}
					}

					// Alternative zeroing property names on sights
					if (outSights.m_aZeroingDistances.IsEmpty())
					{
						array<float> ranges = {};
						cur.Get("m_aZeroingDistances", ranges);
						if (ranges)
						{
							foreach (float r : ranges)
								outSights.m_aZeroingDistances.Insert(r.ToString() + "m");
						}
						else
						{
							float singleZero;
							if (cur.Get("m_fZeroingDistance", singleZero) && singleZero > 0)
								outSights.m_aZeroingDistances.Insert(singleZero.ToString() + "m");
						}
					}

					// Default zero index
					if (outSights.m_iDefaultZeroIndex == 0)
					{
						int defIdx = 0;
						if (cur.Get("SightsRangesDefaultIndex", defIdx))
							outSights.m_iDefaultZeroIndex = defIdx;
						else if (cur.Get("m_iDefaultZeroIndex", defIdx))
							outSights.m_iDefaultZeroIndex = defIdx;
					}

					// Rear sight pivot
					if (outSights.m_sRearPivot.IsEmpty())
					{
						BaseContainer spObj = cur.GetObject("SightsPosition");
						if (spObj)
							spObj.Get("PivotID", outSights.m_sRearPivot);
						if (outSights.m_sRearPivot.IsEmpty())
						{
							BaseContainer sprObj = cur.GetObject("SightsPointRear");
							if (sprObj)
								sprObj.Get("PivotID", outSights.m_sRearPivot);
						}
					}

					// Front sight pivot
					if (outSights.m_sFrontPivot.IsEmpty())
					{
						BaseContainer spfObj = cur.GetObject("SightsPointFront");
						if (spfObj)
							spfObj.Get("PivotID", outSights.m_sFrontPivot);
					}

					cur = cur.GetAncestor();
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract ballistics & dispersion metrics from muzzles.
	static void ExtractBallistics(map<string, ref array<BaseContainer>> comps, TBD_WeaponBallisticsInfo outBallistics)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!IsMuzzleComponentClass(cls))
				continue;

			foreach (BaseContainer mc : bucket)
			{
				BaseContainer cur = mc;
				while (cur)
				{
					if (outBallistics.m_fInitSpeedCoef == 1.0)
					{
						float coef;
						if (cur.Get("BulletInitSpeedCoef", coef) && coef > 0)
							outBallistics.m_fInitSpeedCoef = coef;
					}

					if (outBallistics.m_fDispersionDiameter < 0)
					{
						float dia;
						if (cur.Get("DispersionDiameter", dia) && dia >= 0)
							outBallistics.m_fDispersionDiameter = dia;
					}

					if (outBallistics.m_fDispersionRange < 0)
					{
						float range;
						if (cur.Get("DispersionRange", range) && range >= 0)
							outBallistics.m_fDispersionRange = range;
					}

					cur = cur.GetAncestor();
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract muzzles, magazine wells, default magazines, and fire modes.
	static void ExtractMuzzles(map<string, ref array<BaseContainer>> comps, notnull array<ref TBD_WeaponMuzzleInfo> outMuzzles)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!IsMuzzleComponentClass(cls))
				continue;

			// Deduplicate: filter out containers that are ancestors of another container in the bucket
			array<BaseContainer> distinctMuzzles = {};
			foreach (BaseContainer mc : bucket)
			{
				bool isAncestor = false;
				foreach (BaseContainer other : bucket)
				{
					if (mc == other)
						continue;
					BaseContainer anc = other.GetAncestor();
					while (anc)
					{
						if (anc == mc)
						{
							isAncestor = true;
							break;
						}
						anc = anc.GetAncestor();
					}
					if (isAncestor)
						break;
				}
				if (!isAncestor)
					distinctMuzzles.Insert(mc);
			}

			foreach (BaseContainer muzComp : distinctMuzzles)
			{
				TBD_WeaponMuzzleInfo m = new TBD_WeaponMuzzleInfo();
				m.m_iIndex = outMuzzles.Count();
				m.m_sMuzzleClass = cls;

				// MagazineWell
				BaseContainer cur = muzComp;
				while (cur && m.m_aMagazineWells.IsEmpty())
				{
					BaseContainer wellObj = cur.GetObject("MagazineWell");
					if (wellObj)
					{
						string wCls = wellObj.GetClassName();
						if (!wCls.IsEmpty() && m.m_aMagazineWells.Find(wCls) == -1)
							m.m_aMagazineWells.Insert(wCls);
					}
					BaseContainerList wellList = cur.GetObjectArray("MagazineWells");
					if (!wellList)
						wellList = cur.GetObjectArray("m_aMagazineWells");
					if (wellList)
					{
						for (int wi = 0, win = wellList.Count(); wi < win; wi++)
						{
							BaseContainer wElem = wellList.Get(wi);
							if (wElem)
							{
								string wCls2 = wElem.GetClassName();
								if (!wCls2.IsEmpty() && m.m_aMagazineWells.Find(wCls2) == -1)
									m.m_aMagazineWells.Insert(wCls2);
							}
						}
					}
					cur = cur.GetAncestor();
				}

				// Default MagazineTemplate or AmmoTemplate
				cur = muzComp;
				while (cur && m.m_sDefaultMagazineTemplate.IsEmpty())
				{
					string magTmpl;
					if (cur.Get("MagazineTemplate", magTmpl) && !magTmpl.IsEmpty())
						m.m_sDefaultMagazineTemplate = ResolveCanonical(magTmpl);
					else if (cur.Get("AmmoTemplate", magTmpl) && !magTmpl.IsEmpty())
						m.m_sDefaultMagazineTemplate = ResolveCanonical(magTmpl);

					cur = cur.GetAncestor();
				}

				// Disposable flag
				cur = muzComp;
				while (cur && !m.m_bIsDisposable)
				{
					bool disp = false;
					if (cur.Get("Disposable", disp) && disp)
						m.m_bIsDisposable = true;
					cur = cur.GetAncestor();
				}

				// FireModes
				BaseContainerList modes = null;
				cur = muzComp;
				while (cur && !modes)
				{
					modes = cur.GetObjectArray("FireModes");
					if (!modes)
						modes = cur.GetObjectArray("m_aFireModes");
					cur = cur.GetAncestor();
				}

				if (modes)
				{
					for (int fm = 0, fmn = modes.Count(); fm < fmn; fm++)
					{
						BaseContainer mode = modes.Get(fm);
						if (!mode)
							continue;

						TBD_WeaponFireModeInfo fmInfo = new TBD_WeaponFireModeInfo();

						// Try C++ engine instance instantiation for full config resolution
						BaseFireMode bfm = BaseFireMode.Cast(BaseContainerTools.CreateInstanceFromContainer(mode));
						if (bfm)
						{
							fmInfo.m_sName = bfm.GetUIName();
							fmInfo.m_sFireModeType = FiremodeTypeToString(bfm.GetFiremodeType());
							fmInfo.m_iBurstCount = bfm.GetBurstSize();
							fmInfo.m_sBurstType = BurstTypeToString(bfm.GetBurstType());
							float span = bfm.GetShotSpan();
							if (span > 0)
								fmInfo.m_iRoundsPerMinute = Math.Round(60.0 / span);
						}

						// Fallback if instance instantiation left RPM or Name unpopulated
						if (fmInfo.m_iRoundsPerMinute == 0)
						{
							int rpm = 0;
							if (!mode.Get("RoundsPerMinute", rpm) || rpm == 0)
							{
								if (!mode.Get("m_iRoundsPerMinute", rpm) || rpm == 0)
								{
									float rpmFloat;
									if (mode.Get("m_fRoundsPerMinute", rpmFloat))
										rpm = Math.Round(rpmFloat);
								}
							}
							if (rpm == 0 && mode.GetAncestor())
								mode.GetAncestor().Get("RoundsPerMinute", rpm);
							fmInfo.m_iRoundsPerMinute = rpm;
						}

						if (fmInfo.m_sName.IsEmpty() || fmInfo.m_sName.StartsWith("#") || fmInfo.m_sName.StartsWith("AR-"))
						{
							string modeName = "";
							mode.Get("UIName", modeName);
							if (modeName.IsEmpty() && mode.GetAncestor())
								mode.GetAncestor().Get("UIName", modeName);

							if (modeName.IsEmpty())
							{
								int maxBurst = 1;
								mode.Get("MaxBurst", maxBurst);
								if (maxBurst == 1 && mode.GetAncestor())
									mode.GetAncestor().Get("MaxBurst", maxBurst);

								string confRef = mode.GetResourceName();
								if (confRef.IsEmpty() && mode.GetAncestor())
									confRef = mode.GetAncestor().GetResourceName();

								if (confRef.Contains("Single") || maxBurst == 1)
								{
									modeName = "Semi";
									if (fmInfo.m_iBurstCount <= 0) fmInfo.m_iBurstCount = 1;
									if (fmInfo.m_sFireModeType.IsEmpty()) fmInfo.m_sFireModeType = "Semiauto";
								}
								else if (confRef.Contains("Burst") || maxBurst > 1)
								{
									modeName = "Burst";
									if (fmInfo.m_iBurstCount <= 1) fmInfo.m_iBurstCount = maxBurst;
									if (fmInfo.m_sFireModeType.IsEmpty()) fmInfo.m_sFireModeType = "Burst";
								}
								else if (confRef.Contains("Auto") || maxBurst == -1)
								{
									modeName = "Full Auto";
									fmInfo.m_iBurstCount = 0;
									if (fmInfo.m_sFireModeType.IsEmpty()) fmInfo.m_sFireModeType = "Auto";
								}
								else if (confRef.Contains("Safe") || fmInfo.m_iRoundsPerMinute == 0)
								{
									modeName = "Safe";
									fmInfo.m_iBurstCount = 0;
									if (fmInfo.m_sFireModeType.IsEmpty()) fmInfo.m_sFireModeType = "Safety";
								}
								else
								{
									modeName = "Mode " + (fm + 1).ToString();
								}
							}
							else
							{
								modeName = CleanLocalizationToken(modeName);
							}
							fmInfo.m_sName = modeName;
						}

						if (fmInfo.m_sBurstType.IsEmpty())
							fmInfo.m_sBurstType = "Interruptable";

						m.m_aFireModes.Insert(fmInfo);
					}
				}

				// Deduplicate against already extracted muzzles for this weapon (ancestor overrides)
				bool isDuplicate = false;
				foreach (TBD_WeaponMuzzleInfo existing : outMuzzles)
				{
					if (m.m_sMuzzleClass == existing.m_sMuzzleClass)
					{
						bool wellsMatch = false;
						if (!m.m_aMagazineWells.IsEmpty() && !existing.m_aMagazineWells.IsEmpty())
						{
							if (m.m_aMagazineWells[0] == existing.m_aMagazineWells[0])
								wellsMatch = true;
						}
						else
						{
							wellsMatch = true;
						}

						bool magMatch = false;
						if (!m.m_sDefaultMagazineTemplate.IsEmpty() && !existing.m_sDefaultMagazineTemplate.IsEmpty())
						{
							if (m.m_sDefaultMagazineTemplate == existing.m_sDefaultMagazineTemplate)
								magMatch = true;
						}
						else
						{
							magMatch = true;
						}

						if (wellsMatch && magMatch)
						{
							isDuplicate = true;
							if (existing.m_aMagazineWells.IsEmpty() && !m.m_aMagazineWells.IsEmpty())
							{
								foreach (string mw : m.m_aMagazineWells)
									existing.m_aMagazineWells.Insert(mw);
							}
							if (existing.m_sDefaultMagazineTemplate.IsEmpty() && !m.m_sDefaultMagazineTemplate.IsEmpty())
								existing.m_sDefaultMagazineTemplate = m.m_sDefaultMagazineTemplate;
							if (existing.m_aFireModes.IsEmpty() && !m.m_aFireModes.IsEmpty())
							{
								foreach (TBD_WeaponFireModeInfo fmi : m.m_aFireModes)
									existing.m_aFireModes.Insert(fmi);
							}
							if (!existing.m_bIsDisposable && m.m_bIsDisposable)
								existing.m_bIsDisposable = true;
							break;
						}
					}
				}

				if (!isDuplicate)
				{
					if (!m.m_aMagazineWells.IsEmpty() || !m.m_sDefaultMagazineTemplate.IsEmpty() || !m.m_aFireModes.IsEmpty())
					{
						m.m_iIndex = outMuzzles.Count();
						outMuzzles.Insert(m);
					}
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	protected static string FiremodeTypeToString(EWeaponFiremodeType fmt)
	{
		switch (fmt)
		{
			case EWeaponFiremodeType.Safety: return "Safety";
			case EWeaponFiremodeType.Semiauto: return "Semiauto";
			case EWeaponFiremodeType.Auto: return "Auto";
			case EWeaponFiremodeType.Burst: return "Burst";
			case EWeaponFiremodeType.Manual: return "Manual";
		}
		return "Standard";
	}

	//------------------------------------------------------------------------------------------------
	protected static string BurstTypeToString(EBurstType bt)
	{
		switch (bt)
		{
			case EBurstType.BT_Uninterruptable: return "Uninterruptable";
			case EBurstType.BT_Interruptable: return "Interruptable";
			case EBurstType.BT_InterruptableAndResetting: return "InterruptableAndResetting";
		}
		return "Interruptable";
	}

	//------------------------------------------------------------------------------------------------
	//! Extract attachment slots, mount types, pre-attached prefabs, and obstruction rules.
	static void ExtractAttachmentSlots(map<string, ref array<BaseContainer>> comps, notnull array<ref TBD_WeaponAttachmentSlotInfo> outSlots)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("AttachmentSlotComponent"))
				continue;

			foreach (BaseContainer slotComp : bucket)
			{
				BaseContainer slotObj = slotComp.GetObject("AttachmentSlot");
				BaseContainer typeObj = slotComp.GetObject("AttachmentType");
				string typeClass = "";
				if (typeObj)
					typeClass = typeObj.GetClassName();

				string slotName = "";
				string pivotId = "";
				string defAttach = "";

				BaseContainer curSlot = slotObj;
				while (curSlot)
				{
					if (slotName.IsEmpty())
						slotName = curSlot.GetName();

					if (pivotId.IsEmpty())
						curSlot.Get("PivotID", pivotId);

					if (defAttach.IsEmpty())
					{
						if (!curSlot.Get("Prefab", defAttach) || defAttach.IsEmpty())
							curSlot.Get("m_sAttachment", defAttach);
					}

					curSlot = curSlot.GetAncestor();
				}

				// Resolve attachment type class from ancestor slot component if missing
				if (typeClass.IsEmpty())
				{
					BaseContainer curComp = slotComp.GetAncestor();
					while (curComp && typeClass.IsEmpty())
					{
						BaseContainer ancType = curComp.GetObject("AttachmentType");
						if (ancType)
							typeClass = ancType.GetClassName();
						curComp = curComp.GetAncestor();
					}
				}

				// Clean human-friendly slot name
				if (slotName.IsEmpty() || slotName.StartsWith("InventoryStorageSlot"))
				{
					if (!pivotId.IsEmpty())
						slotName = pivotId;
					else if (!typeClass.IsEmpty())
						slotName = typeClass;
					else
						slotName = "attachment_slot";
				}

				if (slotName.StartsWith("slot_"))
					slotName = slotName.Substring(5, slotName.Length() - 5);

				if (typeClass.IsEmpty() && slotName.IsEmpty())
					continue;

				// Obstruction check: extract blocked attachment classes from SCR_WeaponAttachmentObstructionAttributes
				array<string> obstructedTypes = {};
				ExtractObstructions(slotComp, obstructedTypes);

				// Deduplicate and merge defaults
				bool duplicate = false;
				foreach (TBD_WeaponAttachmentSlotInfo existing : outSlots)
				{
					if (existing.m_sSlotName == slotName || (!typeClass.IsEmpty() && existing.m_sRequiredAttachmentType == typeClass))
					{
						duplicate = true;
						if (existing.m_sDefaultAttachedPrefab.IsEmpty() && !defAttach.IsEmpty())
							existing.m_sDefaultAttachedPrefab = ResolveCanonical(defAttach);
						if (existing.m_sPivotId.IsEmpty() && !pivotId.IsEmpty())
							existing.m_sPivotId = pivotId;

						foreach (string obs : obstructedTypes)
						{
							if (existing.m_aObstructedAttachmentTypes.Find(obs) == -1)
								existing.m_aObstructedAttachmentTypes.Insert(obs);
						}
						break;
					}
				}

				if (!duplicate)
				{
					TBD_WeaponAttachmentSlotInfo s = new TBD_WeaponAttachmentSlotInfo();
					s.m_sSlotName = slotName;
					s.m_sPivotId = pivotId;
					s.m_sRequiredAttachmentType = typeClass;
					if (!defAttach.IsEmpty())
						s.m_sDefaultAttachedPrefab = ResolveCanonical(defAttach);
					s.m_aObstructedAttachmentTypes = obstructedTypes;
					outSlots.Insert(s);
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	protected static void ExtractObstructions(BaseContainer slotComp, notnull array<string> outObstructed)
	{
		BaseContainer cur = slotComp;
		while (cur)
		{
			BaseContainer customAttrs = cur.GetObject("CustomAttributes");
			if (customAttrs)
			{
				BaseContainerList attrList = customAttrs.GetObjectArray("m_aAttributes");
				if (attrList)
				{
					for (int i = 0, n = attrList.Count(); i < n; i++)
					{
						BaseContainer attr = attrList.Get(i);
						if (!attr || !attr.GetClassName().Contains("Obstruction"))
							continue;

						BaseContainerList obsList = attr.GetObjectArray("m_aObstructedAttachmentTypes");
						if (obsList)
						{
							for (int j = 0, jn = obsList.Count(); j < jn; j++)
							{
								BaseContainer obsObj = obsList.Get(j);
								if (obsObj)
								{
									string obsCls = obsObj.GetClassName();
									if (!obsCls.IsEmpty() && outObstructed.Find(obsCls) == -1)
										outObstructed.Insert(obsCls);
								}
							}
						}
					}
				}
			}
			cur = cur.GetAncestor();
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract human-readable display name.
	static string DisplayNameFor(map<string, ref array<BaseContainer>> comps, string filePath)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("InventoryItemComponent"))
				continue;
			foreach (BaseContainer inv : bucket)
			{
				BaseContainer attrs = inv.GetObject("Attributes");
				if (!attrs)
					continue;
				BaseContainer disp = attrs.GetObject("ItemDisplayName");
				if (!disp)
					continue;
				string n;
				if (disp.Get("Name", n) && !n.IsEmpty())
				{
					string cleaned = CleanLocalizationToken(n);
					if (!cleaned.IsEmpty())
						return cleaned;
				}
			}
		}

		array<BaseContainer> weaponComps = comps.Get("WeaponComponent");
		if (weaponComps)
		{
			foreach (BaseContainer wc : weaponComps)
			{
				BaseContainer ui = wc.GetObject("UIInfo");
				if (!ui)
					continue;
				string n2;
				if (ui.Get("Name", n2) && !n2.IsEmpty())
				{
					string cleaned2 = CleanLocalizationToken(n2);
					if (!cleaned2.IsEmpty())
						return cleaned2;
				}
			}
		}

		return HumanizeStem(filePath);
	}

	//------------------------------------------------------------------------------------------------
	//! Extract localized description string.
	static string DescriptionFor(map<string, ref array<BaseContainer>> comps)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("InventoryItemComponent"))
				continue;
			foreach (BaseContainer inv : bucket)
			{
				BaseContainer attrs = inv.GetObject("Attributes");
				if (!attrs)
					continue;
				BaseContainer disp = attrs.GetObject("ItemDisplayName");
				if (!disp)
					continue;
				string d;
				if (disp.Get("Description", d) && !d.IsEmpty())
					return CleanLocalizationToken(d);
			}
		}

		array<BaseContainer> weaponComps = comps.Get("WeaponComponent");
		if (weaponComps)
		{
			foreach (BaseContainer wc : weaponComps)
			{
				BaseContainer ui = wc.GetObject("UIInfo");
				if (!ui)
					continue;
				string d2;
				if (ui.Get("Description", d2) && !d2.IsEmpty())
					return CleanLocalizationToken(d2);
			}
		}

		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! Extract icon texture resource path.
	static string IconFor(map<string, ref array<BaseContainer>> comps)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("InventoryItemComponent"))
				continue;
			foreach (BaseContainer inv : bucket)
			{
				BaseContainer attrs = inv.GetObject("Attributes");
				if (!attrs)
					continue;
				BaseContainer disp = attrs.GetObject("ItemDisplayName");
				if (!disp)
					continue;
				string icon;
				if (disp.Get("Icon", icon) && !icon.IsEmpty())
					return ResolveCanonical(icon);
			}
		}

		array<BaseContainer> weaponComps = comps.Get("WeaponComponent");
		if (weaponComps)
		{
			foreach (BaseContainer wc : weaponComps)
			{
				BaseContainer ui = wc.GetObject("UIInfo");
				if (!ui)
					continue;
				string icon2;
				if (ui.Get("Icon", icon2) && !icon2.IsEmpty())
					return ResolveCanonical(icon2);
			}
		}

		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! Clean localization tokens into human-readable strings.
	static string CleanLocalizationToken(string token)
	{
		if (!token.StartsWith("#") && !token.StartsWith("AR-"))
			return token;

		string s = token;
		if (s.StartsWith("#"))
			s = s.Substring(1, s.Length() - 1);
		if (s.StartsWith("AR-"))
			s = s.Substring(3, s.Length() - 3);

		if (s.StartsWith("Weapon_"))
			s = s.Substring(7, s.Length() - 7);
		else if (s.StartsWith("Item_"))
			s = s.Substring(5, s.Length() - 5);
		else if (s.StartsWith("Magazine_"))
			s = s.Substring(9, s.Length() - 9);

		if (s.EndsWith("_Name"))
			s = s.Substring(0, s.Length() - 5);
		else if (s.EndsWith("_Description"))
			s = s.Substring(0, s.Length() - 12);

		s.Replace("_", " ");
		s.Trim();
		return s;
	}

	//------------------------------------------------------------------------------------------------
	static string HumanizeStem(string filePath)
	{
		string stem = filePath;
		int slash = stem.LastIndexOf("/");
		if (slash >= 0)
			stem = stem.Substring(slash + 1, stem.Length() - slash - 1);
		if (stem.EndsWith(".et"))
			stem = stem.Substring(0, stem.Length() - 3);

		if (stem.StartsWith("Rifle_"))
			stem = stem.Substring(6, stem.Length() - 6);
		else if (stem.StartsWith("Weapon_"))
			stem = stem.Substring(7, stem.Length() - 7);
		else if (stem.StartsWith("Launcher_"))
			stem = stem.Substring(9, stem.Length() - 9);
		else if (stem.StartsWith("Grenade_"))
			stem = stem.Substring(8, stem.Length() - 8);
		else if (stem.StartsWith("Handgun_"))
			stem = stem.Substring(8, stem.Length() - 8);
		else if (stem.StartsWith("UGL_"))
			stem = stem.Substring(4, stem.Length() - 4);
		else if (stem.StartsWith("Mine_"))
			stem = stem.Substring(5, stem.Length() - 5);

		stem.Replace("_", " ");
		stem.Trim();
		return stem;
	}

	//------------------------------------------------------------------------------------------------
	//! Extract weapon family from folder hierarchy.
	static string ExtractFamily(string filePath, string category)
	{
		string marker = "Weapons/" + category + "/";
		if (category == "rifles") marker = "Weapons/Rifles/";
		else if (category == "machine_guns") marker = "Weapons/MachineGuns/";
		else if (category == "handguns") marker = "Weapons/Handguns/";
		else if (category == "launchers") marker = "Weapons/Launchers/";
		else if (category == "grenades") marker = "Weapons/Grenades/";
		else if (category == "explosives") marker = "Weapons/Explosives/";
		else if (category == "underbarrel") marker = "Attachments/Underbarrel/";

		int idx = filePath.IndexOf(marker);
		if (idx >= 0)
		{
			string sub = filePath.Substring(idx + marker.Length(), filePath.Length() - idx - marker.Length());
			int slash = sub.IndexOf("/");
			if (slash > 0)
				return sub.Substring(0, slash);
			if (sub.EndsWith(".et"))
			{
				sub = sub.Substring(0, sub.Length() - 3);
				return sub;
			}
			return sub;
		}

		// Fallback to stem prefix
		string stem = HumanizeStem(filePath);
		int sp = stem.IndexOf(" ");
		if (sp > 0)
			return stem.Substring(0, sp);
		return stem;
	}

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	static string ResolveCanonical(string resName)
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
}
