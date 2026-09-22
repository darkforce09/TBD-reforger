//------------------------------------------------------------------------------------------------
// TBD_AmmoMagazineExtractor.c
//
// Reads what a magazine is: the wells it fits, how many rounds it holds, the caliber and ammo type
// it chambers, its empty and loaded mass, and the caliber category the catalog files it under.
//
// Capacity and caliber are frequently declared on an ancestor rather than on the variant prefab,
// so each is searched up the container ancestry. The ammunition config is the indirection that
// names the projectile a magazine chambers; it is read here and exported as a foreign key rather
// than inlined, so the platform resolves magazines against projectiles itself.
//------------------------------------------------------------------------------------------------

class TBD_AmmoMagazineExtractor
{
	//------------------------------------------------------------------------------------------------
	//! Extract magazine wells (pure relational foreign keys to weapon muzzles).
	static void ExtractMagazineWells(map<string, ref array<BaseContainer>> comps, notnull array<string> outWells)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("MagazineComponent") && !cls.EndsWith("BaseMagazineComponent"))
				continue;

			foreach (BaseContainer magComp : bucket)
			{
				BaseContainer cur = magComp;
				while (cur)
				{
					// Direct single MagazineWell object
					BaseContainer wellObj = cur.GetObject("MagazineWell");
					if (wellObj)
					{
						string wCls = wellObj.GetClassName();
						if (!wCls.IsEmpty() && outWells.Find(wCls) == -1)
							outWells.Insert(wCls);
					}

					// Array of MagazineWells
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
								if (!wCls2.IsEmpty() && outWells.Find(wCls2) == -1)
									outWells.Insert(wCls2);
							}
						}
					}

					cur = cur.GetAncestor();
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract capacity, weight per round, and ammo style.
	static void ExtractCapacity(map<string, ref array<BaseContainer>> comps, TBD_MagazineCapacityInfo outCap, string filePath)
	{
		// 1. MaxAmmo on MagazineComponent
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("MagazineComponent") && !cls.EndsWith("BaseMagazineComponent"))
				continue;

			foreach (BaseContainer magComp : bucket)
			{
				BaseContainer cur = magComp;
				while (cur && outCap.m_iRoundCapacity <= 0)
				{
					int maxAmmo = 0;
					if (cur.Get("MaxAmmo", maxAmmo) && maxAmmo > 0)
						outCap.m_iRoundCapacity = maxAmmo;
					else if (cur.Get("m_iMaxAmmo", maxAmmo) && maxAmmo > 0)
						outCap.m_iRoundCapacity = maxAmmo;

					cur = cur.GetAncestor();
				}
			}
		}

		// 2. WeightPerAmmo on InventoryMagazineComponent
		foreach (string invCls, array<BaseContainer> invBucket : comps)
		{
			if (!invCls.EndsWith("InventoryMagazineComponent"))
				continue;

			foreach (BaseContainer invComp : invBucket)
			{
				BaseContainer curInv = invComp;
				while (curInv && outCap.m_fWeightPerRoundKg < 0)
				{
					float wpa = -1.0;
					if (curInv.Get("WeightPerAmmo", wpa) && wpa >= 0)
						outCap.m_fWeightPerRoundKg = wpa;

					curInv = curInv.GetAncestor();
				}
			}
		}

		// 3. Fallback capacity parsing from filename
		if (outCap.m_iRoundCapacity <= 0)
		{
			string lower = filePath;
			lower.ToLower();

			if (lower.Contains("250rnd")) outCap.m_iRoundCapacity = 250;
			else if (lower.Contains("200rnd")) outCap.m_iRoundCapacity = 200;
			else if (lower.Contains("150rnd")) outCap.m_iRoundCapacity = 150;
			else if (lower.Contains("100rnd")) outCap.m_iRoundCapacity = 100;
			else if (lower.Contains("75rnd")) outCap.m_iRoundCapacity = 75;
			else if (lower.Contains("60rnd")) outCap.m_iRoundCapacity = 60;
			else if (lower.Contains("50rnd")) outCap.m_iRoundCapacity = 50;
			else if (lower.Contains("45rnd")) outCap.m_iRoundCapacity = 45;
			else if (lower.Contains("40rnd")) outCap.m_iRoundCapacity = 40;
			else if (lower.Contains("30rnd")) outCap.m_iRoundCapacity = 30;
			else if (lower.Contains("20rnd")) outCap.m_iRoundCapacity = 20;
			else if (lower.Contains("15rnd")) outCap.m_iRoundCapacity = 15;
			else if (lower.Contains("10rnd")) outCap.m_iRoundCapacity = 10;
			else if (lower.Contains("8rnd")) outCap.m_iRoundCapacity = 8;
			else if (lower.Contains("7rnd")) outCap.m_iRoundCapacity = 7;
			else if (lower.Contains("5rnd")) outCap.m_iRoundCapacity = 5;
			else if (lower.Contains("40mm") || lower.Contains("grenade") || lower.Contains("rocket") || lower.Contains("missile")) outCap.m_iRoundCapacity = 1;
			else outCap.m_iRoundCapacity = 30; // default standard
		}

		// 4. Derive ammo style
		string pLower = filePath;
		pLower.ToLower();
		if (pLower.Contains("drum"))
		{
			outCap.m_sAmmoStyle = "drum";
		}
		else if (pLower.Contains("belt") || (pLower.Contains("box") && outCap.m_iRoundCapacity >= 50))
		{
			outCap.m_sAmmoStyle = "belt";
		}
		else if (pLower.Contains("clip") || pLower.Contains("stripper"))
		{
			outCap.m_sAmmoStyle = "clip";
		}
		else if (outCap.m_iRoundCapacity == 1)
		{
			outCap.m_sAmmoStyle = "single_round";
		}
		else
		{
			outCap.m_sAmmoStyle = "box";
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract caliber name, ammo type string, and flags.
	static void ExtractCaliber(map<string, ref array<BaseContainer>> comps, TBD_MagazineCaliberInfo outCaliber, string filePath)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("MagazineComponent") && !cls.EndsWith("BaseMagazineComponent"))
				continue;

			foreach (BaseContainer magComp : bucket)
			{
				BaseContainer cur = magComp;
				while (cur)
				{
					BaseContainer ui = cur.GetObject("UIInfo");
					if (ui)
					{
						if (outCaliber.m_sCaliberName.IsEmpty())
						{
							string cal;
							if (ui.Get("m_sAmmoCaliber", cal) && !cal.IsEmpty())
								outCaliber.m_sCaliberName = TBD_AmmoNaming.CleanLocalizationToken(cal);
						}

						if (outCaliber.m_sAmmoType.IsEmpty())
						{
							string atype;
							if (ui.Get("m_sAmmoType", atype) && !atype.IsEmpty())
								outCaliber.m_sAmmoType = TBD_AmmoNaming.CleanLocalizationToken(atype);
						}

						if (outCaliber.m_iAmmoTypeFlags == 0)
						{
							int flags;
							if (ui.Get("m_eAmmoTypeFlags", flags) && flags > 0)
								outCaliber.m_iAmmoTypeFlags = flags;
						}
					}
					cur = cur.GetAncestor();
				}
			}
		}

		// Fallback caliber parsing from path / stem
		if (outCaliber.m_sCaliberName.IsEmpty())
		{
			string p = filePath;
			p.ToLower();
			if (p.Contains("556x45")) outCaliber.m_sCaliberName = "5.56x45mm NATO";
			else if (p.Contains("545x39")) outCaliber.m_sCaliberName = "5.45x39mm";
			else if (p.Contains("762x39")) outCaliber.m_sCaliberName = "7.62x39mm";
			else if (p.Contains("762x51")) outCaliber.m_sCaliberName = "7.62x51mm NATO";
			else if (p.Contains("762x54")) outCaliber.m_sCaliberName = "7.62x54mmR";
			else if (p.Contains("9x18")) outCaliber.m_sCaliberName = "9x18mm Makarov";
			else if (p.Contains("9x19")) outCaliber.m_sCaliberName = "9x19mm Parabellum";
			else if (p.Contains("45acp")) outCaliber.m_sCaliberName = ".45 ACP";
			else if (p.Contains("40x46") || p.Contains("40mm") || p.Contains("m203")) outCaliber.m_sCaliberName = "40x46mm";
			else if (p.Contains("vog25") || p.Contains("gp25")) outCaliber.m_sCaliberName = "40mm VOG-25";
			else if (p.Contains("12_7x108") || p.Contains("12.7x108")) outCaliber.m_sCaliberName = "12.7x108mm";
			else if (p.Contains("50bmg") || p.Contains("12_7x99")) outCaliber.m_sCaliberName = ".50 BMG (12.7x99mm)";
			else if (p.Contains("14_5x114") || p.Contains("14.5x114")) outCaliber.m_sCaliberName = "14.5x114mm";
			else if (p.Contains("25x137")) outCaliber.m_sCaliberName = "25x137mm";
			else if (p.Contains("30x165") || p.Contains("30mm")) outCaliber.m_sCaliberName = "30x165mm";
			else if (p.Contains("81mm")) outCaliber.m_sCaliberName = "81mm";
			else if (p.Contains("82mm")) outCaliber.m_sCaliberName = "82mm";
			else if (p.Contains("pg7") || p.Contains("rpg7")) outCaliber.m_sCaliberName = "85mm PG-7";
			else if (p.Contains("m72")) outCaliber.m_sCaliberName = "66mm";
			else outCaliber.m_sCaliberName = "Standard";
		}

		// Fallback ammo type
		if (outCaliber.m_sAmmoType.IsEmpty())
		{
			string p2 = filePath;
			p2.ToLower();
			if (p2.Contains("tracer") && p2.Contains("ap")) outCaliber.m_sAmmoType = "APTracer";
			else if (p2.Contains("tracer")) outCaliber.m_sAmmoType = "Tracer";
			else if (p2.Contains("ap")) outCaliber.m_sAmmoType = "AP";
			else if (p2.Contains("hedp")) outCaliber.m_sAmmoType = "HEDP";
			else if (p2.Contains("heat")) outCaliber.m_sAmmoType = "HEAT";
			else if (p2.Contains("smoke")) outCaliber.m_sAmmoType = "Smoke";
			else if (p2.Contains("illum")) outCaliber.m_sAmmoType = "Illum";
			else outCaliber.m_sAmmoType = "Ball";
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract AmmoConfig (.conf) and referenced projectile prefabs (AmmoResourceArray / AmmoTemplate).
	static void ExtractAmmoConfig(map<string, ref array<BaseContainer>> comps, notnull array<string> outAmmoResources, out string outConfPath)
	{
		outConfPath = string.Empty;

		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("MagazineComponent") && !cls.EndsWith("BaseMagazineComponent"))
				continue;

			foreach (BaseContainer magComp : bucket)
			{
				BaseContainer cur = magComp;
				while (cur)
				{
					string confPath;
					if (cur.Get("AmmoConfig", confPath) && !confPath.IsEmpty())
					{
						outConfPath = confPath;
						Resource r = Resource.Load(confPath);
						if (r && r.IsValid())
						{
							BaseResourceObject bro = r.GetResource();
							if (bro)
							{
								BaseContainer confCont = bro.ToBaseContainer();
								if (confCont)
								{
									array<ResourceName> resArr = {};
									confCont.Get("AmmoResourceArray", resArr);
									if (resArr)
									{
										for (int ri = 0; ri < resArr.Count(); ri++)
										{
											string canon = TBD_EquipmentResourceNames.ResolveCanonicalResourceName(resArr[ri]);
											if (!canon.IsEmpty() && outAmmoResources.Find(canon) == -1)
												outAmmoResources.Insert(canon);
										}
									}
								}
							}
						}
					}

					// Check fallback AmmoTemplate
					string ammoTmpl;
					if (cur.Get("AmmoTemplate", ammoTmpl) && !ammoTmpl.IsEmpty())
					{
						string canonTmpl = TBD_EquipmentResourceNames.ResolveCanonicalResourceName(ammoTmpl);
						if (!canonTmpl.IsEmpty() && outAmmoResources.Find(canonTmpl) == -1)
							outAmmoResources.Insert(canonTmpl);
					}

					cur = cur.GetAncestor();
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract physical metrics: empty weight, full weight, volume, dimensions, and slot size.
	static void ExtractPhysical(map<string, ref array<BaseContainer>> comps, int capacity, float weightPerRound, TBD_MagazinePhysicalInfo outPhys)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("InventoryItemComponent") && !cls.EndsWith("InventoryMagazineComponent") && !cls.EndsWith("StorageComponent"))
				continue;

			foreach (BaseContainer comp : bucket)
			{
				BaseContainer attrs = comp.GetObject("Attributes");
				if (!attrs)
					continue;

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
						if (outPhys.m_fWeightEmptyKg < 0)
						{
							float w;
							if (curPhys.Get("Weight", w) && w >= 0)
								outPhys.m_fWeightEmptyKg = w;
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

		// Compute full weight: empty + (capacity * weight_per_round)
		if (outPhys.m_fWeightEmptyKg >= 0)
		{
			if (capacity > 0 && weightPerRound > 0)
				outPhys.m_fWeightFullKg = outPhys.m_fWeightEmptyKg + (capacity * weightPerRound);
			else
				outPhys.m_fWeightFullKg = outPhys.m_fWeightEmptyKg;
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Categorize magazine into dedicated sub-catalog.
	static string CategorizeMagazine(array<string> wells, int capacity, string filePath)
	{
		foreach (string w : wells)
		{
			if (w.Contains("Stanag") || w.Contains("AK545") || w.Contains("AK762") || w.Contains("M14") || w.Contains("SVD") || w.Contains("VZ58") || w.Contains("Mosin"))
				return "magazines_rifle";
			if (w.Contains("M249") || w.Contains("M60") || w.Contains("PKM") || w.Contains("UK59") || w.Contains("RPK"))
				return "magazines_mg";
			if (w.Contains("M9") || w.Contains("Makarov") || w.Contains("1911") || w.Contains("PM"))
				return "magazines_handgun";
			if (w.Contains("M2HB") || w.Contains("NSV") || w.Contains("KPVT") || w.Contains("2A42") || w.Contains("M242"))
				return "magazines_heavy";
			if (w.Contains("UGL") || w.Contains("GP") || w.Contains("M203"))
				return "magazines_grenades";
			if (w.Contains("RPG7") || w.Contains("RPG75") || w.Contains("LAW") || w.Contains("Rocket"))
				return "magazines_rockets";
		}

		// Fallback to path
		string p = filePath;
		p.ToLower();
		if (p.Contains("drum") || (p.Contains("box") && capacity >= 100)) return "magazines_mg";
		if (p.Contains("handgun") || p.Contains("pistol")) return "magazines_handgun";
		if (p.Contains("grenade") || p.Contains("40mm")) return "magazines_grenades";
		if (p.Contains("rocket") || p.Contains("missile") || p.Contains("rpg")) return "magazines_rockets";
		if (p.Contains("heavy") || p.Contains("30mm") || p.Contains("25mm") || p.Contains("12_7") || p.Contains("50bmg")) return "magazines_heavy";

		return "magazines_rifle";
	}
}
