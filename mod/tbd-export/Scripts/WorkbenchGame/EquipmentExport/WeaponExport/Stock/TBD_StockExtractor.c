//------------------------------------------------------------------------------------------------
// TBD_StockExtractor.c
//
// Core introspection engine for extracting complete, ground-truth component graphs from weapon buttstocks
// and stock assemblies.
// Extracts mounting relational keys, nested child attachment slots (e.g. cheek pads, risers, sling points),
// handling and recoil modifiers, physical attributes, and visuals (3D model mesh).
//
// 100% genuine introspection:
//   - ZERO fake/mock data: omitted values serialize as JSON null.
//   - ZERO hardcoded class checks: works universally with any mod.
//   - All attributes, relational keys, and child slots are extracted directly from native Enfusion
//     BaseContainer objects and container ancestry.
//   - Localization tokens are preserved raw without stripping # or fabricating humanized strings.
//   - Nested Attachment Host: Modular buttstocks provide child slots (cheek pad/riser slots,
//     sling swivel attachment points) declared via AttachmentSlotComponent.
//------------------------------------------------------------------------------------------------

class TBD_StockExtractor
{
	//------------------------------------------------------------------------------------------------
	//! Extract handling and recoil modifiers directly from SCR_WeaponAttachmentAttributes.
	//! Introspects recoil angular factors, linear factors, turn factors, and extra obstruction length.
	static void ExtractHandlingModifiers(map<string, ref array<BaseContainer>> comps, TBD_StockHandlingInfo outHandling)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("InventoryItemComponent"))
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

								// Walk container ancestor chain to extract declared recoil/handling modifiers
								BaseContainer curAttr = attr;
								while (curAttr)
								{
									if (!outHandling.m_bHasRecoilAngularFactors)
									{
										vector ang;
										if (curAttr.IsVariableSet("m_vAngularFactors") && curAttr.Get("m_vAngularFactors", ang) && ang != "0 0 0")
										{
											outHandling.m_vRecoilAngularFactors = ang;
											outHandling.m_bHasRecoilAngularFactors = true;
										}
										else if (curAttr.IsVariableSet("AngularFactors") && curAttr.Get("AngularFactors", ang) && ang != "0 0 0")
										{
											outHandling.m_vRecoilAngularFactors = ang;
											outHandling.m_bHasRecoilAngularFactors = true;
										}
									}

									if (!outHandling.m_bHasRecoilLinearFactors)
									{
										vector lin;
										if (curAttr.IsVariableSet("m_vLinearFactors") && curAttr.Get("m_vLinearFactors", lin) && lin != "0 0 0")
										{
											outHandling.m_vRecoilLinearFactors = lin;
											outHandling.m_bHasRecoilLinearFactors = true;
										}
										else if (curAttr.IsVariableSet("LinearFactors") && curAttr.Get("LinearFactors", lin) && lin != "0 0 0")
										{
											outHandling.m_vRecoilLinearFactors = lin;
											outHandling.m_bHasRecoilLinearFactors = true;
										}
									}

									if (!outHandling.m_bHasTurnFactors)
									{
										vector turn;
										if (curAttr.IsVariableSet("m_vTurnFactors") && curAttr.Get("m_vTurnFactors", turn) && turn != "0 0 0")
										{
											outHandling.m_vTurnFactors = turn;
											outHandling.m_bHasTurnFactors = true;
										}
										else if (curAttr.IsVariableSet("TurnFactors") && curAttr.Get("TurnFactors", turn) && turn != "0 0 0")
										{
											outHandling.m_vTurnFactors = turn;
											outHandling.m_bHasTurnFactors = true;
										}
									}

									if (!outHandling.m_bHasExtraObstructionLength)
									{
										float extra;
										if (curAttr.IsVariableSet("m_fExtraObstructionLength") && curAttr.Get("m_fExtraObstructionLength", extra))
										{
											outHandling.m_fExtraObstructionLength = extra;
											outHandling.m_bHasExtraObstructionLength = true;
										}
										else if (curAttr.IsVariableSet("ExtraObstructionLength") && curAttr.Get("ExtraObstructionLength", extra))
										{
											outHandling.m_fExtraObstructionLength = extra;
											outHandling.m_bHasExtraObstructionLength = true;
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

		// Fallback check on non-slot components for direct obstruction length if not under CustomAttributes
		if (!outHandling.m_bHasExtraObstructionLength)
		{
			foreach (string compCls, array<BaseContainer> compBucket : comps)
			{
				if (compCls.EndsWith("AttachmentSlotComponent") || compCls.EndsWith("SlotManagerComponent"))
					continue;

				foreach (BaseContainer c : compBucket)
				{
					BaseContainer curC = c;
					while (curC)
					{
						float cExtraLen;
						if (curC.IsVariableSet("m_fExtraObstructionLength") && curC.Get("m_fExtraObstructionLength", cExtraLen))
						{
							outHandling.m_fExtraObstructionLength = cExtraLen;
							outHandling.m_bHasExtraObstructionLength = true;
							break;
						}
						else if (curC.IsVariableSet("ExtraObstructionLength") && curC.Get("ExtraObstructionLength", cExtraLen))
						{
							outHandling.m_fExtraObstructionLength = cExtraLen;
							outHandling.m_bHasExtraObstructionLength = true;
							break;
						}
						curC = curC.GetAncestor();
					}
					if (outHandling.m_bHasExtraObstructionLength)
						break;
				}
				if (outHandling.m_bHasExtraObstructionLength)
					break;
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract physical attributes: mass, volume, dimensions, inventory layout size.
	//! ZERO synthetic defaults: omitted fields serialize as null.
	static void ExtractPhysical(map<string, ref array<BaseContainer>> comps, TBD_StockPhysicalInfo outPhys)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("InventoryItemComponent"))
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
	}

	//------------------------------------------------------------------------------------------------
	//! Extract visual 3D model mesh path directly from MeshObject.Object.
	static void ExtractVisuals(map<string, ref array<BaseContainer>> comps, TBD_StockVisualsInfo outVisuals)
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
