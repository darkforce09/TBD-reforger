//------------------------------------------------------------------------------------------------
// TBD_WeaponMuzzleExtractor.c
//
// Reads the muzzles a weapon declares: the magazine wells each one feeds from, its default
// magazine, its barrel and chamber properties, and the fire modes it supports with their rate of
// fire and burst length.
//
// A weapon reaches its muzzles through MuzzleComponent and its subclasses, and a variant prefab
// usually declares only what it changes, so each value is searched up the container ancestry until
// one is found. Engine firemode and burst enums are mapped to the catalog's strings here.
//------------------------------------------------------------------------------------------------

class TBD_WeaponMuzzleExtractor
{
	//------------------------------------------------------------------------------------------------
	//! Returns true if the class name represents a true weapon firing muzzle component (not visual effects).
	static bool IsMuzzleComponentClass(string cls)
	{
		if (cls.Contains("MuzzleEffect"))
			return false;
		return cls.EndsWith("MuzzleComponent") || cls.EndsWith("MuzzleInMagComponent");
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
						m.m_sDefaultMagazineTemplate = TBD_EquipmentResourceNames.ResolveCanonicalResourceName(magTmpl);
					else if (cur.Get("AmmoTemplate", magTmpl) && !magTmpl.IsEmpty())
						m.m_sDefaultMagazineTemplate = TBD_EquipmentResourceNames.ResolveCanonicalResourceName(magTmpl);

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
								modeName = TBD_WeaponNaming.CleanLocalizationToken(modeName);
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
}
