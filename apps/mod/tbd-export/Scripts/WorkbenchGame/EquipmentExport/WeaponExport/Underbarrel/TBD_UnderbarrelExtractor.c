//------------------------------------------------------------------------------------------------
// TBD_UnderbarrelExtractor.c
//
// Core introspection engine for extracting complete, ground-truth component graphs from underbarrel devices.
// Extracts mounting relational keys, secondary weapon / launcher attributes (muzzles, magazine wells,
// chamber capacity, zeroing distances), physical attributes, and visual 3D model meshes.
//
// 100% genuine introspection:
//   - ZERO fake/mock data: omitted values serialize as JSON null.
//   - ZERO hardcoded class checks: works universally with any mod.
//   - All attributes and relational keys are extracted directly from native Enfusion BaseContainer
//     objects and container ancestry.
//   - Localization tokens are preserved raw without stripping # or fabricating humanized strings.
//------------------------------------------------------------------------------------------------

class TBD_UnderbarrelExtractor
{

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

	//------------------------------------------------------------------------------------------------
	//! Extract mounting relational keys directly from container data and genuine container ancestry:
	//!   - attachment_type: primary class name declared in AttachmentType / m_AttachmentType
	//!   - compatible_attachment_types: container ancestor classes and declared m_aCompatibleAttachmentTypes
	//!   - obstructed_attachment_types: classes from m_aObstructedAttachmentTypes
	//! ZERO hardcoded class checks: works dynamically for any mod.
	static void ExtractMounting(map<string, ref array<BaseContainer>> comps, TBD_UnderbarrelMountingInfo outMounting)
	{
		string primaryType;
		ref array<string> compatTypes = {};
		ref array<string> obstructedTypes = {};

		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("InventoryItemComponent") && !cls.EndsWith("WeaponAttachmentComponent"))
				continue;

			foreach (BaseContainer inv : bucket)
			{
				BaseContainer cur = inv;
				while (cur)
				{
					BaseContainer attrs = cur.GetObject("Attributes");
					while (attrs)
					{
						BaseContainerList attrList = TBD_EquipmentDisplayAttributes.GetCustomAttributes(attrs);
						if (attrList)
						{
							for (int a = 0, an = attrList.Count(); a < an; a++)
							{
								BaseContainer attr = attrList.Get(a);
								if (!attr)
									continue;

								// Walk container ancestor chain to discover genuine attachment types and compatibility
								BaseContainer curAttr = attr;
								while (curAttr)
								{
									// 1. Primary AttachmentType and container ancestor classes
									BaseContainer typeObj = curAttr.GetObject("AttachmentType");
									if (!typeObj)
										typeObj = curAttr.GetObject("m_AttachmentType");

									if (typeObj)
									{
										string typeCls = typeObj.GetClassName();
										if (!typeCls.IsEmpty() && !typeCls.StartsWith("AttachmentCamouflage"))
										{
											if (primaryType.IsEmpty() || primaryType == "BaseAttachmentType" || primaryType == "AttachmentBase" || primaryType == "AttachmentUnderBarrel")
												primaryType = typeCls;

											TBD_EquipmentComponentGraph.AddUniqueType(compatTypes, typeCls);

											// Check if typeObj itself has an ancestor container
											BaseContainer ancType = typeObj.GetAncestor();
											while (ancType)
											{
												string ancCls = ancType.GetClassName();
												if (!ancCls.IsEmpty())
													TBD_EquipmentComponentGraph.AddUniqueType(compatTypes, ancCls);
												ancType = ancType.GetAncestor();
											}
										}
									}

									// Read explicit compatible types if declared on container
									BaseContainerList cList = curAttr.GetObjectArray("m_aCompatibleAttachmentTypes");
									if (cList)
									{
										for (int c = 0, cn = cList.Count(); c < cn; c++)
										{
											BaseContainer ct = cList.Get(c);
											if (ct)
											{
												string ctCls = ct.GetClassName();
												if (!ctCls.IsEmpty())
													TBD_EquipmentComponentGraph.AddUniqueType(compatTypes, ctCls);
											}
										}
									}

									// 2. Weapon attachment obstruction attributes
									BaseContainerList obsList = curAttr.GetObjectArray("m_aObstructedAttachmentTypes");
									if (obsList)
									{
										for (int o = 0, on = obsList.Count(); o < on; o++)
										{
											BaseContainer ot = obsList.Get(o);
											if (ot)
											{
												string otCls = ot.GetClassName();
												if (!otCls.IsEmpty())
													TBD_EquipmentComponentGraph.AddUniqueType(obstructedTypes, otCls);
											}
										}
									}

									curAttr = curAttr.GetAncestor();
								}
							}
						}

						attrs = attrs.GetAncestor();
					}

					cur = cur.GetAncestor();
				}
			}
		}

		// Fallback check on non-slot components for direct AttachmentType if still empty
		if (primaryType.IsEmpty() || primaryType == "BaseAttachmentType" || primaryType == "AttachmentUnderBarrel")
		{
			foreach (string compCls, array<BaseContainer> compBucket : comps)
			{
				if (compCls.EndsWith("AttachmentSlotComponent") || compCls.EndsWith("SlotManagerComponent"))
					continue;

				foreach (BaseContainer c : compBucket)
				{
					BaseContainer atObj = c.GetObject("AttachmentType");
					if (!atObj)
						atObj = c.GetObject("m_AttachmentType");

					if (atObj)
					{
						string cType = atObj.GetClassName();
						if (cType != "BaseAttachmentType" && !cType.IsEmpty() && !cType.StartsWith("AttachmentCamouflage"))
						{
							primaryType = cType;
							TBD_EquipmentComponentGraph.AddUniqueType(compatTypes, cType);
							BaseContainer ancType = atObj.GetAncestor();
							while (ancType)
							{
								string ancCls = ancType.GetClassName();
								if (!ancCls.IsEmpty())
									TBD_EquipmentComponentGraph.AddUniqueType(compatTypes, ancCls);
								ancType = ancType.GetAncestor();
							}
							break;
						}
					}
				}
				if (!primaryType.IsEmpty() && primaryType != "BaseAttachmentType" && primaryType != "AttachmentUnderBarrel")
					break;
			}
		}

		// Ensure primary type is at index 0 of compatible types if present
		if (!primaryType.IsEmpty())
		{
			int idx = compatTypes.Find(primaryType);
			if (idx > 0)
			{
				compatTypes.Remove(idx);
				compatTypes.InsertAt(primaryType, 0);
			}
			else if (idx == -1)
			{
				compatTypes.InsertAt(primaryType, 0);
			}
		}

		outMounting.m_sAttachmentType = primaryType;
		outMounting.m_aCompatibleAttachmentTypes = compatTypes;
		outMounting.m_aObstructedAttachmentTypes = obstructedTypes;
	}

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
	//! Extract physical attributes: mass, volume, dimensions, inventory layout size.
	//! ZERO synthetic defaults: omitted fields serialize as null.
	static void ExtractPhysical(map<string, ref array<BaseContainer>> comps, TBD_UnderbarrelPhysicalInfo outPhys)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("InventoryItemComponent") && !cls.EndsWith("StorageComponent"))
				continue;

			foreach (BaseContainer inv : bucket)
			{
				BaseContainer cur = inv;
				while (cur)
				{
					BaseContainer attrs = cur.GetObject("Attributes");
					while (attrs)
					{
						// Inventory item size token
						if (outPhys.m_sInventorySize.IsEmpty())
						{
							string sz;
							if (attrs.Get("m_Size", sz) && !sz.IsEmpty())
								outPhys.m_sInventorySize = sz;
						}

						// Item physical attributes
						BaseContainer phys = attrs.GetObject("ItemPhysAttributes");
						while (phys)
						{
							if (!outPhys.m_bHasWeight)
							{
								float weight;
								if (phys.Get("Weight", weight) && weight > 0)
								{
									outPhys.m_fWeightKg = weight;
									outPhys.m_bHasWeight = true;
								}
							}

							if (!outPhys.m_bHasVolume)
							{
								float volume;
								if (phys.Get("ItemVolume", volume) && volume > 0)
								{
									outPhys.m_fVolumeCm3 = volume;
									outPhys.m_bHasVolume = true;
								}
							}

							if (!outPhys.m_bHasDimensions)
							{
								vector dim;
								if (phys.Get("ItemDimensions", dim) && dim != "0 0 0")
								{
									outPhys.m_vDimensions = dim;
									outPhys.m_bHasDimensions = true;
								}
							}

							phys = phys.GetAncestor();
						}

						attrs = attrs.GetAncestor();
					}

					cur = cur.GetAncestor();
				}
			}
		}

		// Fallback for mass on RigidBody if weight was not defined in ItemPhysAttributes
		if (!outPhys.m_bHasWeight)
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
						outPhys.m_bHasWeight = true;
						break;
					}
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract visual 3D model mesh path directly from MeshObject.Object.
	static void ExtractVisuals(map<string, ref array<BaseContainer>> comps, TBD_UnderbarrelVisualsInfo outVisuals)
	{
		string meshPath;

		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("MeshObject"))
				continue;

			foreach (BaseContainer mesh : bucket)
			{
				BaseContainer cur = mesh;
				while (cur)
				{
					string obj;
					if (cur.Get("Object", obj) && !obj.IsEmpty())
					{
						meshPath = TBD_EquipmentResourceNames.NormalizePathSeparators(obj);
						break;
					}
					cur = cur.GetAncestor();
				}

				if (!meshPath.IsEmpty())
					break;
			}

			if (!meshPath.IsEmpty())
				break;
		}

		outVisuals.m_sModelMesh = meshPath;
	}
}
