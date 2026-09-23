/**
 * TBD_WearableExtractor.c
 *
 * Introspects container graphs across prefab ancestry to extract:
 * 1. Wear area type and blocked loadout slots (BaseLoadoutClothComponent).
 * 2. Storage capacity, max weight, and cargo grid (SCR_UniversalInventoryStorageComponent).
 * 3. Physical weight, volume, dimensions, and inventory size (InventoryItemComponent).
 * 4. Ballistic protection, passed damage scale, and protected hit zones (SCR_ArmorDamageManagerComponent).
 * 5. Modular child slots (e.g. helmet NVG/cover slots, vest pouch slots).
 * 6. Visual worn, item, and deflated models.
 */

class TBD_WearableExtractor
{
	//------------------------------------------------------------------------------------------------
	//! Extract loadout area class and blocked slots from cloth component.
	static void ExtractWearableArea(map<string, ref array<BaseContainer>> comps, out string outAreaType, notnull array<string> outBlockedSlots)
	{
		outAreaType = string.Empty;

		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("LoadoutClothComponent") && !cls.EndsWith("ClothComponent"))
				continue;

			foreach (BaseContainer cloth : bucket)
			{
				BaseContainer cur = cloth;
				while (cur)
				{
					if (outAreaType.IsEmpty())
					{
						BaseContainer areaObj = cur.GetObject("AreaType");
						if (areaObj)
						{
							string aCls = areaObj.GetClassName();
							if (!aCls.IsEmpty())
								outAreaType = aCls;
						}
					}

					// Blocked slots array
					BaseContainerList blocked = cur.GetObjectArray("BlockedSlots");
					if (blocked)
					{
						for (int b = 0, nb = blocked.Count(); b < nb; b++)
						{
							BaseContainer blkObj = blocked.Get(b);
							if (!blkObj)
								continue;
							string blkCls = blkObj.GetClassName();
							if (!blkCls.IsEmpty() && outBlockedSlots.Find(blkCls) == -1)
								outBlockedSlots.Insert(blkCls);
						}
					}

					cur = cur.GetAncestor();
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract storage capacity and derive cargo grid dimensions.
	static void ExtractStorage(map<string, ref array<BaseContainer>> comps, TBD_WearableStorageInfo outStorage)
	{
		// First pass: Universal inventory storage (inventory panel source)
		ExtractStoragePass(comps, outStorage, true);

		// Fallback pass: other storage components if universal storage had no values
		if (!outStorage.m_bHasStorage)
			ExtractStoragePass(comps, outStorage, false);

		if (outStorage.m_fMaxVolumeCm3 > 0)
		{
			outStorage.m_bHasStorage = true;
			float vol = outStorage.m_fMaxVolumeCm3;
			int cells = Math.Ceil(vol / 50.0);
			if (cells < 1)
				cells = 1;
			outStorage.m_iCargoGridW = 4;
			int h = Math.Ceil(cells / 4.0);
			if (h < 3)
				h = 3;
			outStorage.m_iCargoGridH = h;
		}
		else if (outStorage.m_fMaxWeightKg > 0)
		{
			outStorage.m_bHasStorage = true;
		}
	}

	//------------------------------------------------------------------------------------------------
	protected static void ExtractStoragePass(map<string, ref array<BaseContainer>> comps, TBD_WearableStorageInfo outStorage, bool universalOnly)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (universalOnly && !cls.EndsWith("UniversalInventoryStorageComponent"))
				continue;
			if (!universalOnly && !cls.EndsWith("StorageComponent"))
				continue;

			foreach (BaseContainer storage : bucket)
			{
				BaseContainer cur = storage;
				while (cur)
				{
					if (outStorage.m_fMaxWeightKg < 0)
					{
						float mw;
						if (cur.Get("m_fMaxWeight", mw) && mw >= 0)
							outStorage.m_fMaxWeightKg = mw;
					}

					if (outStorage.m_fMaxVolumeCm3 < 0)
					{
						float mv;
						if (cur.Get("MaxCumulativeVolume", mv) && mv >= 0)
							outStorage.m_fMaxVolumeCm3 = mv;
					}

					cur = cur.GetAncestor();
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract physical attributes: weight, volume, dimensions, and inventory UI size.
	static void ExtractPhysical(map<string, ref array<BaseContainer>> comps, TBD_WearablePhysicalInfo outPhys)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("InventoryItemComponent") && !cls.EndsWith("StorageComponent"))
				continue;

			foreach (BaseContainer comp : bucket)
			{
				BaseContainer curComp = comp;
				while (curComp)
				{
					BaseContainer attrs = curComp.GetObject("Attributes");
					if (attrs)
					{
						BaseContainer curAttrs = attrs;
						while (curAttrs)
						{
							if (outPhys.m_sInventorySize.IsEmpty())
							{
								string sz;
								if (curAttrs.Get("m_Size", sz) && !sz.IsEmpty())
									outPhys.m_sInventorySize = sz;
							}

							BaseContainer phys = curAttrs.GetObject("ItemPhysAttributes");
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

							curAttrs = curAttrs.GetAncestor();
						}
					}

					curComp = curComp.GetAncestor();
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract ballistic armor damage management attributes.
	static void ExtractArmor(map<string, ref array<BaseContainer>> comps, TBD_WearableArmorInfo outArmor)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("ArmorDamageManagerComponent"))
				continue;

			outArmor.m_bHasArmor = true;

			foreach (BaseContainer dmgMgr : bucket)
			{
				BaseContainer cur = dmgMgr;
				while (cur)
				{
					if (outArmor.m_fPassedDamageScale < 0)
					{
						float scale;
						if (cur.Get("m_fPassedDamageScale", scale) && scale >= 0)
							outArmor.m_fPassedDamageScale = scale;
					}

					// Check hit zones container / array
					BaseContainerList hitZones = cur.GetObjectArray("Additional hit zones");
					if (!hitZones)
						hitZones = cur.GetObjectArray("HitZones");

					if (hitZones)
					{
						for (int h = 0, nh = hitZones.Count(); h < nh; h++)
						{
							BaseContainer hz = hitZones.Get(h);
							if (!hz)
								continue;

							string hzName = hz.GetName();
							if (!hzName.IsEmpty() && outArmor.m_aProtectedHitZones.Find(hzName) == -1)
								outArmor.m_aProtectedHitZones.Insert(hzName);

							if (outArmor.m_sProtectionLevel.IsEmpty())
							{
								string lvl;
								if (hz.Get("m_eProtectionLevel", lvl) && !lvl.IsEmpty())
									outArmor.m_sProtectionLevel = lvl;
							}
						}
					}

					cur = cur.GetAncestor();
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract child attachment slots (e.g. helmet rails, vest pouch slots).
	static void ExtractSlots(map<string, ref array<BaseContainer>> comps, notnull array<ref TBD_WearableSlotInfo> outSlots)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("LoadoutClothComponent") && !cls.EndsWith("ClothComponent"))
				continue;

			foreach (BaseContainer cloth : bucket)
			{
				BaseContainer cur = cloth;
				while (cur)
				{
					BaseContainerList slots = cur.GetObjectArray("Slots");
					if (slots)
					{
						for (int i = 0, n = slots.Count(); i < n; i++)
						{
							BaseContainer slot = slots.Get(i);
							if (!slot)
								continue;

							string slotName = slot.GetName();
							if (slotName.IsEmpty())
								slotName = slot.GetClassName();

							// Avoid duplicate slot registrations from ancestry
							bool exists = false;
							for (int s = 0; s < outSlots.Count(); s++)
							{
								if (outSlots[s].m_sSlotName == slotName)
								{
									exists = true;
									break;
								}
							}
							if (exists)
								continue;

							TBD_WearableSlotInfo info = new TBD_WearableSlotInfo();
							info.m_sSlotName = slotName;

							BaseContainer areaObj = slot.GetObject("AreaType");
							if (areaObj)
								info.m_sAreaType = areaObj.GetClassName();

							string prefab;
							if (slot.Get("Prefab", prefab) && !prefab.IsEmpty())
								info.m_sDefaultPrefab = TBD_EquipmentResourceNames.ResolveCanonicalResourceName(prefab);

							outSlots.Insert(info);
						}
					}

					cur = cur.GetAncestor();
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract 3D models (worn on character, ground item, deflated).
	static void ExtractVisual(map<string, ref array<BaseContainer>> comps, TBD_WearableVisualInfo outVisual)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("LoadoutClothComponent") && !cls.EndsWith("ClothComponent"))
				continue;

			foreach (BaseContainer cloth : bucket)
			{
				BaseContainer cur = cloth;
				while (cur)
				{
					if (outVisual.m_sWornModel.IsEmpty())
					{
						string worn;
						if (cur.Get("WornModel", worn) && !worn.IsEmpty())
							outVisual.m_sWornModel = TBD_EquipmentResourceNames.NormalizePathSeparators(worn);
					}

					if (outVisual.m_sItemModel.IsEmpty())
					{
						string item;
						if (cur.Get("ItemModel", item) && !item.IsEmpty())
							outVisual.m_sItemModel = TBD_EquipmentResourceNames.NormalizePathSeparators(item);
					}

					if (outVisual.m_sDeflatedModel.IsEmpty())
					{
						string defl;
						if (cur.Get("DeflatedModel", defl) && !defl.IsEmpty())
							outVisual.m_sDeflatedModel = TBD_EquipmentResourceNames.NormalizePathSeparators(defl);
					}

					cur = cur.GetAncestor();
				}
			}
		}
	}
}
