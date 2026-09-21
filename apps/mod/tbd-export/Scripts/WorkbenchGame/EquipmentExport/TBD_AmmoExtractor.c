//------------------------------------------------------------------------------------------------
// TBD_AmmoExtractor.c
//
// Core introspection engine for extracting complete, deep component graphs from magazines,
// ammunition configs, and projectile prefabs across arbitrary ancestor hierarchies.
// Extracts magazine wells, round capacities, tracer ratios, projectile ballistics, warhead damage,
// arming safety distances, mass, volume, and 3D projectile/cartridge assets.
//------------------------------------------------------------------------------------------------

class TBD_AmmoExtractor
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
								outCaliber.m_sCaliberName = CleanLocalizationToken(cal);
						}

						if (outCaliber.m_sAmmoType.IsEmpty())
						{
							string atype;
							if (ui.Get("m_sAmmoType", atype) && !atype.IsEmpty())
								outCaliber.m_sAmmoType = CleanLocalizationToken(atype);
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
											string canon = ResolveCanonical(resArr[ri]);
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
						string canonTmpl = ResolveCanonical(ammoTmpl);
						if (!canonTmpl.IsEmpty() && outAmmoResources.Find(canonTmpl) == -1)
							outAmmoResources.Insert(canonTmpl);
					}

					cur = cur.GetAncestor();
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Parse AmmoMapping and calculate exact tracer ratios, intervals, and clusters.
	static void ExtractTracers(map<string, ref array<BaseContainer>> comps, array<string> ammoResources, int roundCapacity, TBD_TracerRatioInfo outTracers, string displayName)
	{
		// 1. Read AmmoMapping from MagazineComponent
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("MagazineComponent") && !cls.EndsWith("BaseMagazineComponent"))
				continue;

			foreach (BaseContainer magComp : bucket)
			{
				BaseContainer cur = magComp;
				while (cur && outTracers.m_aAmmoMapping.IsEmpty())
				{
					cur.Get("AmmoMapping", outTracers.m_aAmmoMapping);
					cur = cur.GetAncestor();
				}
			}
		}

		// 2. If AmmoMapping is empty: fallback based on name and capacity
		if (outTracers.m_aAmmoMapping.IsEmpty())
		{
			string dLower = displayName;
			dLower.ToLower();
			if (dLower.Contains("tracer") && !dLower.Contains("mixed") && !dLower.Contains("4ap"))
			{
				outTracers.m_bHasTracers = true;
				outTracers.m_iTracerCount = roundCapacity;
				outTracers.m_iStandardCount = 0;
				outTracers.m_sRatioString = "1:0";
				outTracers.m_iInterval = 1;
				outTracers.m_iTerminalTracerCluster = 0;
			}
			else
			{
				outTracers.m_bHasTracers = false;
				outTracers.m_iTracerCount = 0;
				outTracers.m_iStandardCount = roundCapacity;
				outTracers.m_sRatioString = "none";
				outTracers.m_iInterval = 0;
				outTracers.m_iTerminalTracerCluster = 0;
			}
			return;
		}

		// 3. Identify which resource indices are tracers
		array<int> tracerIndices = {};
		for (int ri = 0; ri < ammoResources.Count(); ri++)
		{
			string res = ammoResources[ri];
			string resLower = res;
			resLower.ToLower();

			bool isTracer = (resLower.Contains("tracer") || resLower.Contains("_t_") || resLower.EndsWith("_t.et") || resLower.Contains("-t."));
			if (!isTracer)
			{
				Resource r = Resource.Load(res);
				if (r && r.IsValid())
				{
					BaseResourceObject ro = r.GetResource();
					if (ro && ro.ToBaseContainer() && ro.ToBaseContainer().GetClassName() == "TracerProjectile")
						isTracer = true;
				}
			}

			if (isTracer)
				tracerIndices.Insert(ri);
		}

		// If no resource identified as tracer by string, but display name says tracer and we have > 1 resources:
		if (tracerIndices.IsEmpty() && ammoResources.Count() > 1)
		{
			string dLower2 = displayName;
			dLower2.ToLower();
			if (dLower2.Contains("tracer"))
				tracerIndices.Insert(1); // default secondary index in belt configs
		}

		// 4. Count rounds
		int totalRounds = outTracers.m_aAmmoMapping.Count();
		int tracerCount = 0;
		for (int m = 0; m < totalRounds; m++)
		{
			int idx = outTracers.m_aAmmoMapping[m];
			if (tracerIndices.Find(idx) != -1)
				tracerCount++;
		}

		outTracers.m_iTracerCount = tracerCount;
		outTracers.m_iStandardCount = totalRounds - tracerCount;
		outTracers.m_bHasTracers = (tracerCount > 0);

		if (!outTracers.m_bHasTracers)
		{
			outTracers.m_sRatioString = "none";
			outTracers.m_iInterval = 0;
			outTracers.m_iTerminalTracerCluster = 0;
			return;
		}

		if (outTracers.m_iStandardCount == 0)
		{
			outTracers.m_sRatioString = "1:0";
			outTracers.m_iInterval = 1;
			outTracers.m_iTerminalTracerCluster = 0;
			return;
		}

		// 5. Detect cadence / interval between tracers
		array<int> tracerPositions = {};
		for (int tp = 0; tp < totalRounds; tp++)
		{
			if (tracerIndices.Find(outTracers.m_aAmmoMapping[tp]) != -1)
				tracerPositions.Insert(tp);
		}

		if (tracerPositions.Count() >= 2)
		{
			int diff = tracerPositions[1] - tracerPositions[0];
			if (diff > 0)
				outTracers.m_iInterval = diff;
		}

		// 6. Detect terminal tracer cluster at the end of the belt
		int termCluster = 0;
		for (int back = totalRounds - 1; back >= 0; back--)
		{
			if (tracerIndices.Find(outTracers.m_aAmmoMapping[back]) != -1)
				termCluster++;
			else
				break;
		}
		outTracers.m_iTerminalTracerCluster = termCluster;

		// 7. Format ratio string
		if (outTracers.m_iInterval > 1)
		{
			int stdPerTracer = outTracers.m_iInterval - 1;
			outTracers.m_sRatioString = stdPerTracer.ToString() + ":1";
		}
		else if (outTracers.m_iTracerCount > 0 && outTracers.m_iStandardCount > 0)
		{
			int approxRatio = Math.Round((1.0 * outTracers.m_iStandardCount) / outTracers.m_iTracerCount);
			if (approxRatio > 0)
				outTracers.m_sRatioString = approxRatio.ToString() + ":1";
			else
				outTracers.m_sRatioString = "1:1";
		}
		else
		{
			outTracers.m_sRatioString = "1:1";
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
	//! Extract projectile ballistics & kinetic parameters from Move components.
	static void ExtractBallistics(map<string, ref array<BaseContainer>> comps, TBD_ProjectileBallisticsInfo outBallistics)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("MoveComponent") && !cls.EndsWith("ProjectileComponent"))
				continue;

			foreach (BaseContainer mc : bucket)
			{
				BaseContainer cur = mc;
				while (cur)
				{
					if (outBallistics.m_fInitSpeedMps < 0)
					{
						float speed;
						if (cur.Get("InitSpeed", speed) && speed >= 0)
							outBallistics.m_fInitSpeedMps = speed;
						else if (cur.Get("m_fInitSpeed", speed) && speed >= 0)
							outBallistics.m_fInitSpeedMps = speed;
					}

					if (outBallistics.m_fInitSpeedVariation == 0)
					{
						float var;
						if (cur.Get("InitSpeedVariation", var) && var > 0)
							outBallistics.m_fInitSpeedVariation = var;
					}

					if (outBallistics.m_fAirDrag < 0)
					{
						float drag;
						if (cur.Get("AirDrag", drag) && drag >= 0)
							outBallistics.m_fAirDrag = drag;
					}

					if (outBallistics.m_fMassKg < 0)
					{
						float mass;
						if (cur.Get("Mass", mass) && mass >= 0)
							outBallistics.m_fMassKg = mass;
					}

					if (outBallistics.m_fMaxPenetration < 0)
					{
						float pen;
						if (cur.Get("MaxPenetration", pen) && pen >= 0)
							outBallistics.m_fMaxPenetration = pen;
					}

					if (outBallistics.m_sBallisticTableConfig.IsEmpty())
					{
						string bt;
						if (cur.Get("BallisticTableConfig", bt) && !bt.IsEmpty())
							outBallistics.m_sBallisticTableConfig = ResolveCanonical(bt);
					}

					cur = cur.GetAncestor();
				}
			}
		}

		// Fallback for Mass from RigidBody if MoveComponent did not specify
		if (outBallistics.m_fMassKg < 0)
		{
			array<BaseContainer> rbBucket = comps.Get("RigidBody");
			if (rbBucket)
			{
				foreach (BaseContainer rb : rbBucket)
				{
					float m;
					if (rb.Get("Mass", m) && m > 0)
					{
						outBallistics.m_fMassKg = m;
						break;
					}
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract warhead, explosive parameters, safety distance, and damage effects.
	static void ExtractWarhead(map<string, ref array<BaseContainer>> comps, TBD_ProjectileWarheadInfo outWarhead)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("CollisionTriggerComponent") && !cls.EndsWith("TriggerComponent") && !cls.EndsWith("ExplosiveChargeComponent"))
				continue;

			foreach (BaseContainer tc : bucket)
			{
				BaseContainer cur = tc;
				while (cur)
				{
					if (outWarhead.m_fSafetyDistanceMeters == 0)
					{
						float sd;
						if (cur.Get("SafetyDistance", sd) && sd > 0)
							outWarhead.m_fSafetyDistanceMeters = sd;
					}

					// Inspect PROJECTILE_EFFECTS
					BaseContainerList effList = cur.GetObjectArray("PROJECTILE_EFFECTS");
					if (!effList)
						effList = cur.GetObjectArray("ProjectileEffects");
					if (effList)
					{
						for (int e = 0, en = effList.Count(); e < en; e++)
						{
							BaseContainer eff = effList.Get(e);
							if (!eff)
								continue;

							string effCls = eff.GetClassName();
							if (effCls.Contains("Explosion"))
							{
								outWarhead.m_bIsExplosive = true;

								if (outWarhead.m_sEffectPrefab.IsEmpty())
								{
									string ep;
									if (eff.Get("EffectPrefab", ep) && !ep.IsEmpty())
										outWarhead.m_sEffectPrefab = ResolveCanonical(ep);
								}

								if (outWarhead.m_sSoundEvent.IsEmpty())
									eff.Get("SoundEvent", outWarhead.m_sSoundEvent);

								if (outWarhead.m_sParticleEffect.IsEmpty())
									eff.Get("ParticleEffect", outWarhead.m_sParticleEffect);
							}

							if (effCls.Contains("Damage"))
							{
								if (outWarhead.m_fDamageValue < 0)
								{
									float dmg;
									if (eff.Get("DamageValue", dmg) && dmg >= 0)
										outWarhead.m_fDamageValue = dmg;
								}
							}
						}
					}

					cur = cur.GetAncestor();
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract tracer rendering properties.
	static void ExtractTracerInfo(BaseContainer root, map<string, ref array<BaseContainer>> comps, TBD_ProjectileTracerInfo outTracer)
	{
		string rootCls = root.GetClassName();
		if (rootCls == "TracerProjectile" || rootCls.Contains("Tracer"))
			outTracer.m_bIsTracer = true;

		if (!outTracer.m_bIsTracer)
		{
			outTracer.m_bIsTracer = HasCompSuffix(comps, "TracerComponent") || HasCompSuffix(comps, "TracerMoveComponent");
		}

		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.Contains("Tracer"))
				continue;

			foreach (BaseContainer tc : bucket)
			{
				BaseContainer cur = tc;
				while (cur)
				{
					if (outTracer.m_sTracerColor.IsEmpty())
						cur.Get("TracerColor", outTracer.m_sTracerColor);

					if (outTracer.m_fTracerStartDistance == 0)
						cur.Get("TracerStartDistance", outTracer.m_fTracerStartDistance);

					if (outTracer.m_fTracerBurnTime == 0)
						cur.Get("TracerBurnTime", outTracer.m_fTracerBurnTime);

					cur = cur.GetAncestor();
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract 3D models for projectile and fired cartridge casing.
	static void ExtractVisuals(BaseContainer root, TBD_ProjectileVisualsInfo outVisuals)
	{
		BaseContainer cur = root;
		while (cur)
		{
			if (outVisuals.m_sProjectileModel.IsEmpty())
			{
				string pm;
				if (cur.Get("ProjectileModel", pm) && !pm.IsEmpty())
					outVisuals.m_sProjectileModel = ResolveCanonical(pm);
			}

			if (outVisuals.m_sCartridgeModel.IsEmpty())
			{
				string cm;
				if (cur.Get("CartridgeModel", cm) && !cm.IsEmpty())
					outVisuals.m_sCartridgeModel = ResolveCanonical(cm);
			}

			cur = cur.GetAncestor();
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

	//------------------------------------------------------------------------------------------------
	//! Categorize projectile into dedicated sub-catalog.
	static string CategorizeProjectile(string filePath, string caliber, bool isExplosive)
	{
		string p = filePath;
		p.ToLower();

		if (p.Contains("mortar") || caliber.Contains("81mm") || caliber.Contains("82mm"))
			return "projectiles_mortar";

		if (p.Contains("rocket") || p.Contains("missile") || caliber.Contains("PG-7") || caliber == "66mm")
			return "projectiles_rockets";

		if (p.Contains("grenade") || caliber == "40x46mm" || caliber == "40mm VOG-25")
			return "projectiles_grenades";

		if (caliber.Contains("25mm") || caliber.Contains("30mm") || caliber.Contains("50 BMG") || caliber.Contains("12.7") || caliber.Contains("14.5"))
			return "projectiles_heavy";

		return "projectiles_bullets";
	}

	//------------------------------------------------------------------------------------------------
	//! Extract human-readable display name.
	static string DisplayNameFor(map<string, ref array<BaseContainer>> comps, string filePath)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("InventoryItemComponent") && !cls.EndsWith("InventoryMagazineComponent"))
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

		array<BaseContainer> magComps = comps.Get("MagazineComponent");
		if (magComps)
		{
			foreach (BaseContainer mc : magComps)
			{
				BaseContainer ui = mc.GetObject("UIInfo");
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
	//! Extract localized description.
	static string DescriptionFor(map<string, ref array<BaseContainer>> comps)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("InventoryItemComponent") && !cls.EndsWith("InventoryMagazineComponent"))
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

		array<BaseContainer> magComps = comps.Get("MagazineComponent");
		if (magComps)
		{
			foreach (BaseContainer mc : magComps)
			{
				BaseContainer ui = mc.GetObject("UIInfo");
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
			if (!cls.EndsWith("InventoryItemComponent") && !cls.EndsWith("InventoryMagazineComponent"))
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

		array<BaseContainer> magComps = comps.Get("MagazineComponent");
		if (magComps)
		{
			foreach (BaseContainer mc : magComps)
			{
				BaseContainer ui = mc.GetObject("UIInfo");
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
	//! Extract magazine family (STANAG, AK74, AK47, M60, PKM, etc.).
	static string ExtractFamily(string filePath, string category)
	{
		string p = filePath;
		p.ToLower();

		if (p.Contains("stanag")) return "STANAG";
		if (p.Contains("545") || p.Contains("ak74")) return "AK-74";
		if (p.Contains("ak47") || p.Contains("akm") || p.Contains("762x39")) return "AK-47";
		if (p.Contains("m14") || p.Contains("m21")) return "M14";
		if (p.Contains("svd")) return "SVD";
		if (p.Contains("vz58") || p.Contains("sa58")) return "VZ-58";
		if (p.Contains("mosin")) return "Mosin";
		if (p.Contains("m249")) return "M249";
		if (p.Contains("m60")) return "M60";
		if (p.Contains("pkm")) return "PKM";
		if (p.Contains("uk59")) return "UK-59";
		if (p.Contains("m9") || p.Contains("beretta")) return "M9";
		if (p.Contains("makarov") || p.Contains("pm")) return "Makarov";
		if (p.Contains("1911") || p.Contains("colt")) return "1911";
		if (p.Contains("m203") || p.Contains("40x46")) return "M203";
		if (p.Contains("gp25") || p.Contains("vog")) return "GP-25";
		if (p.Contains("rpg7") || p.Contains("pg7")) return "RPG-7";
		if (p.Contains("m72")) return "M72 LAW";
		if (p.Contains("m242")) return "M242";
		if (p.Contains("2a42")) return "2A42";
		if (p.Contains("m2hb")) return "M2HB";
		if (p.Contains("nsv")) return "NSV";

		string stem = HumanizeStem(filePath);
		int sp = stem.IndexOf(" ");
		if (sp > 0)
			return stem.Substring(0, sp);
		return stem;
	}

	//------------------------------------------------------------------------------------------------
	static string CleanLocalizationToken(string token)
	{
		if (!token.StartsWith("#") && !token.StartsWith("AR-"))
			return token;

		string s = token;
		if (s.StartsWith("#"))
			s = s.Substring(1, s.Length() - 1);
		if (s.StartsWith("AR-"))
			s = s.Substring(3, s.Length() - 3);

		if (s.StartsWith("Magazine_"))
			s = s.Substring(9, s.Length() - 9);
		else if (s.StartsWith("Item_"))
			s = s.Substring(5, s.Length() - 5);
		else if (s.StartsWith("AmmoType_"))
			s = s.Substring(9, s.Length() - 9);
		else if (s.StartsWith("AmmunitionID_"))
			s = s.Substring(13, s.Length() - 13);

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

		if (stem.StartsWith("Magazine_"))
			stem = stem.Substring(9, stem.Length() - 9);
		else if (stem.StartsWith("Box_"))
			stem = stem.Substring(4, stem.Length() - 4);
		else if (stem.StartsWith("Ammo_"))
			stem = stem.Substring(5, stem.Length() - 5);

		stem.Replace("_", " ");
		stem.Trim();
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
