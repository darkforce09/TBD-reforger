//------------------------------------------------------------------------------------------------
// TBD_BayonetCombatExtractor.c
//
// Reads what a bayonet does as a weapon: melee damage and damage type, attack range and rate, and
// the handling change fitting it makes to the rifle carrying it.
//
// A bayonet is the one attachment family that is itself a weapon, so its damage values come from
// melee and damage components rather than from the attachment attributes the other domains read.
//------------------------------------------------------------------------------------------------

class TBD_BayonetCombatExtractor
{
	//------------------------------------------------------------------------------------------------
	//! Extract combat & handling attributes (extra obstruction length and melee damage).
	//! ZERO synthetic defaults: omitted fields serialize as null.
	static void ExtractCombatHandling(map<string, ref array<BaseContainer>> comps, TBD_BayonetCombatInfo outCombat)
	{
		// 1. Extra obstruction length from SCR_WeaponAttachmentBayonetAttributes or custom attributes
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

								BaseContainer curAttr = attr;
								while (curAttr)
								{
									// Extra obstruction length
									if (!outCombat.m_bHasExtraObstructionLength)
									{
										float extraLen = 0.0;
										if (curAttr.IsVariableSet("m_fExtraObstructionLength") && curAttr.Get("m_fExtraObstructionLength", extraLen) && extraLen > 0)
										{
											outCombat.m_fExtraObstructionLength = extraLen;
											outCombat.m_bHasExtraObstructionLength = true;
										}
										else if (curAttr.IsVariableSet("ExtraObstructionLength") && curAttr.Get("ExtraObstructionLength", extraLen) && extraLen > 0)
										{
											outCombat.m_fExtraObstructionLength = extraLen;
											outCombat.m_bHasExtraObstructionLength = true;
										}
									}

									// Direct melee damage on bayonet attributes if configured
									if (!outCombat.m_bHasMeleeDamage)
									{
										float directDmg = 0.0;
										if (curAttr.IsVariableSet("m_fMeleeDamage") && curAttr.Get("m_fMeleeDamage", directDmg) && directDmg > 0)
										{
											outCombat.m_fMeleeDamage = directDmg;
											outCombat.m_bHasMeleeDamage = true;
										}
										else if (curAttr.IsVariableSet("MeleeDamage") && curAttr.Get("MeleeDamage", directDmg) && directDmg > 0)
										{
											outCombat.m_fMeleeDamage = directDmg;
											outCombat.m_bHasMeleeDamage = true;
										}
										else if (curAttr.IsVariableSet("m_fDamage") && curAttr.Get("m_fDamage", directDmg) && directDmg > 0)
										{
											outCombat.m_fMeleeDamage = directDmg;
											outCombat.m_bHasMeleeDamage = true;
										}
										else if (curAttr.IsVariableSet("Damage") && curAttr.Get("Damage", directDmg) && directDmg > 0)
										{
											outCombat.m_fMeleeDamage = directDmg;
											outCombat.m_bHasMeleeDamage = true;
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

		// Fallback check on components directly for extra obstruction length
		if (!outCombat.m_bHasExtraObstructionLength)
		{
			foreach (string compCls, array<BaseContainer> compBucket : comps)
			{
				foreach (BaseContainer c : compBucket)
				{
					BaseContainer curC = c;
					while (curC)
					{
						float cExtraLen = 0.0;
						if (curC.IsVariableSet("m_fExtraObstructionLength") && curC.Get("m_fExtraObstructionLength", cExtraLen) && cExtraLen > 0)
						{
							outCombat.m_fExtraObstructionLength = cExtraLen;
							outCombat.m_bHasExtraObstructionLength = true;
							break;
						}
						else if (curC.IsVariableSet("ExtraObstructionLength") && curC.Get("ExtraObstructionLength", cExtraLen) && cExtraLen > 0)
						{
							outCombat.m_fExtraObstructionLength = cExtraLen;
							outCombat.m_bHasExtraObstructionLength = true;
							break;
						}
						curC = curC.GetAncestor();
					}
					if (outCombat.m_bHasExtraObstructionLength)
						break;
				}
				if (outCombat.m_bHasExtraObstructionLength)
					break;
			}
		}

		// 2. Melee damage introspection from SCR_MeleeComponent, MeleeWeaponComponent, or SCR_BayonetComponent
		if (!outCombat.m_bHasMeleeDamage)
		{
			foreach (string meleeCls, array<BaseContainer> meleeBucket : comps)
			{
				if (!meleeCls.Contains("Melee") && !meleeCls.Contains("Bayonet"))
					continue;

				foreach (BaseContainer mc : meleeBucket)
				{
					BaseContainer curMc = mc;
					while (curMc)
					{
						float mDmg = 0.0;
						if (curMc.IsVariableSet("m_fMeleeDamage") && curMc.Get("m_fMeleeDamage", mDmg) && mDmg > 0)
						{
							outCombat.m_fMeleeDamage = mDmg;
							outCombat.m_bHasMeleeDamage = true;
							break;
						}
						if (curMc.IsVariableSet("MeleeDamage") && curMc.Get("MeleeDamage", mDmg) && mDmg > 0)
						{
							outCombat.m_fMeleeDamage = mDmg;
							outCombat.m_bHasMeleeDamage = true;
							break;
						}
						if (curMc.IsVariableSet("m_fDamage") && curMc.Get("m_fDamage", mDmg) && mDmg > 0)
						{
							outCombat.m_fMeleeDamage = mDmg;
							outCombat.m_bHasMeleeDamage = true;
							break;
						}
						if (curMc.IsVariableSet("Damage") && curMc.Get("Damage", mDmg) && mDmg > 0)
						{
							outCombat.m_fMeleeDamage = mDmg;
							outCombat.m_bHasMeleeDamage = true;
							break;
						}
						if (curMc.IsVariableSet("m_fBaseDamage") && curMc.Get("m_fBaseDamage", mDmg) && mDmg > 0)
						{
							outCombat.m_fMeleeDamage = mDmg;
							outCombat.m_bHasMeleeDamage = true;
							break;
						}

						// Check nested sub-objects (e.g. WeaponProperties, HitData, DamageEffect)
						BaseContainer propObj = curMc.GetObject("m_WeaponProperties");
						if (!propObj)
							propObj = curMc.GetObject("WeaponProperties");
						if (!propObj)
							propObj = curMc.GetObject("m_DamageEffect");
						if (!propObj)
							propObj = curMc.GetObject("DamageEffect");

						if (propObj)
						{
							float propDmg = 0.0;
							if (propObj.IsVariableSet("m_fDamage") && propObj.Get("m_fDamage", propDmg) && propDmg > 0)
							{
								outCombat.m_fMeleeDamage = propDmg;
								outCombat.m_bHasMeleeDamage = true;
								break;
							}
							if (propObj.IsVariableSet("Damage") && propObj.Get("Damage", propDmg) && propDmg > 0)
							{
								outCombat.m_fMeleeDamage = propDmg;
								outCombat.m_bHasMeleeDamage = true;
								break;
							}
							if (propObj.IsVariableSet("m_fMeleeDamage") && propObj.Get("m_fMeleeDamage", propDmg) && propDmg > 0)
							{
								outCombat.m_fMeleeDamage = propDmg;
								outCombat.m_bHasMeleeDamage = true;
								break;
							}
						}

						curMc = curMc.GetAncestor();
					}

					if (outCombat.m_bHasMeleeDamage)
						break;
				}

				if (outCombat.m_bHasMeleeDamage)
					break;
			}
		}
	}
}
