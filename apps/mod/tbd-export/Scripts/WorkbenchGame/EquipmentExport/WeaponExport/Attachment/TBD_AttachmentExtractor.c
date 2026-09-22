//------------------------------------------------------------------------------------------------
// TBD_AttachmentExtractor.c
//
// Reads what an attachment is as an object: its mass, volume, dimensions and inventory footprint,
// and the mesh it renders as.
//
// Mounting, per-family properties, naming and this domain's custom attribute reader each have
// their own extractor beside this one. The scanner calls them in turn and assembles a single
// TBD_AttachmentInfo.
//
// Every value is read from the prefab's own BaseContainer graph. Nothing is synthesized: a field
// the prefab does not declare is omitted and serializes as JSON null.
//------------------------------------------------------------------------------------------------

class TBD_AttachmentExtractor
{
	//------------------------------------------------------------------------------------------------
	//! Extract physical attributes: mass, volume, dimensions, inventory layout size.
	//! ZERO synthetic defaults: if omitted in prefab, fields remain null.
	static void ExtractPhysical(map<string, ref array<BaseContainer>> comps, TBD_AttachmentPhysicalInfo outPhys, string category, string filePath)
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
					if (attrs)
					{
						BaseContainer phys = attrs.GetObject("ItemPhysicalAttributes");
						if (!phys)
							phys = attrs.GetObject("ItemPhysAttributes");

						if (phys)
						{
							float weight;
							if (outPhys.m_fWeightKg < 0 && phys.Get("Weight", weight) && weight >= 0)
								outPhys.m_fWeightKg = weight;

							float vol;
							if (outPhys.m_fVolumeCm3 < 0)
							{
								if (phys.Get("Volume", vol) && vol >= 0)
									outPhys.m_fVolumeCm3 = vol;
								else if (phys.Get("ItemVolume", vol) && vol >= 0)
									outPhys.m_fVolumeCm3 = vol;
							}

							vector dims;
							if (!outPhys.m_bHasDimensions)
							{
								if (phys.Get("Dimension", dims) && (dims[0] > 0 || dims[1] > 0 || dims[2] > 0))
								{
									outPhys.m_vDimensions = dims;
									outPhys.m_bHasDimensions = true;
								}
								else if (phys.Get("ItemDimensions", dims) && (dims[0] > 0 || dims[1] > 0 || dims[2] > 0))
								{
									outPhys.m_vDimensions = dims;
									outPhys.m_bHasDimensions = true;
								}
							}
						}

						if (outPhys.m_sInventorySize.IsEmpty())
						{
							BaseContainer sizeObj = attrs.GetObject("m_Size");
							if (sizeObj)
							{
								string sizeCls = sizeObj.GetClassName();
								if (!sizeCls.IsEmpty())
									outPhys.m_sInventorySize = sizeCls;
							}
							else
							{
								string sz;
								if (attrs.Get("m_Size", sz) && !sz.IsEmpty())
									outPhys.m_sInventorySize = sz;
							}
						}
					}
					cur = cur.GetAncestor();
				}
			}
		}

		// Check Physics / RigidBody component for mass if still unset
		if (outPhys.m_fWeightKg < 0)
		{
			foreach (string pCls, array<BaseContainer> pBucket : comps)
			{
				if (!pCls.EndsWith("Physics") && !pCls.EndsWith("PhysicsComponent") && !pCls.EndsWith("RigidBody"))
					continue;

				foreach (BaseContainer pc : pBucket)
				{
					float mass;
					if (pc.Get("Mass", mass) && mass > 0)
					{
						outPhys.m_fWeightKg = mass;
						break;
					}
				}
				if (outPhys.m_fWeightKg > 0)
					break;
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract 3D visual mesh model path.
	static void ExtractVisuals(map<string, ref array<BaseContainer>> comps, TBD_AttachmentVisualsInfo outVisuals)
	{
		string meshPath = "";

		array<BaseContainer> meshComps = comps.Get("MeshObject");
		if (!meshComps)
			meshComps = comps.Get("SCR_MeshObject");

		if (meshComps)
		{
			foreach (BaseContainer mo : meshComps)
			{
				BaseContainer curMo = mo;
				while (curMo)
				{
					string obj;
					if (curMo.Get("Object", obj) && !obj.IsEmpty())
					{
						meshPath = TBD_EquipmentResourceNames.NormalizePathSeparators(obj);
						break;
					}
					curMo = curMo.GetAncestor();
				}
				if (!meshPath.IsEmpty())
					break;
			}
		}

		outVisuals.m_sModelMesh = meshPath;
	}
}
