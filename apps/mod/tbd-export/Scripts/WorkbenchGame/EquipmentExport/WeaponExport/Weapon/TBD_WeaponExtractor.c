//------------------------------------------------------------------------------------------------
// TBD_WeaponExtractor.c
//
// Reads what a weapon is as a physical object: its weight, volume, inventory footprint and
// dimensions, whether it mounts a bipod or is disposable, the zeroing its sights offer, and the
// ballistic table it fires against.
//
// Classification, muzzles, attachment slots and naming each have their own extractor beside this
// one. The scanner calls all five and assembles a single TBD_WeaponInfo from the results.
//------------------------------------------------------------------------------------------------

class TBD_WeaponExtractor
{
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
		outPhys.m_bHasBipod = TBD_EquipmentComponentGraph.HasCompSuffix(comps, "BipodComponent");
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
					if (TBD_WeaponClassificationExtractor.ContainerContainsBipod(wc, 0))
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
		outPhys.m_bIsDisposable = TBD_EquipmentComponentGraph.HasCompSuffix(comps, "SCR_DisposableWeaponComponent");
		if (!outPhys.m_bIsDisposable)
		{
			foreach (string mCls, array<BaseContainer> mBucket : comps)
			{
				if (!TBD_WeaponMuzzleExtractor.IsMuzzleComponentClass(mCls))
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
			if (!TBD_WeaponMuzzleExtractor.IsMuzzleComponentClass(cls))
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
}
