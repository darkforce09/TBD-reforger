// Generated from the authoritative gameplay field-selection policy.
class TBD_GameplayPolicy_magazine_0
{
	static void Apply(TBD_GameplaySelectionPolicy policy)
	{
		policy.AddClass("BaseMagazineComponent", "magazine");
		policy.AddClass("InventoryMagazineComponent", "magazine");
		policy.AddRule("InventoryMagazineComponent", "WbPlacementFromAttributes\tBOOLEAN", "exclude", "excluded", false, "Presentation, authoring, lifecycle, replication, animation, audio, or cosmetic effect setting");
		policy.AddRule("InventoryMagazineComponent", "Attributes\tOBJECT", "retain_relationship", "magazine", false, "Native configuration used by magazine");
		policy.AddRule("InventoryMagazineComponent", "Enabled\tBOOLEAN|WeightPerAmmo\tSCALAR", "retain_value", "magazine", false, "Native configuration used by magazine");
		policy.AddRule("InventoryMagazineComponent", "components\tOBJECT_ARRAY", "traverse_required_container", "magazine", false, "Native configuration used by magazine");
		policy.AddClass("MagazineComponent", "magazine");
		policy.AddRule("MagazineComponent", "CustomAnimationAttributes\tOBJECT", "exclude", "excluded", false, "Presentation, authoring, lifecycle, replication, animation, audio, or cosmetic effect setting");
		policy.AddRule("MagazineComponent", "ItemModel\tRESOURCE_NAME|MagazineWell\tOBJECT|UIInfo\tOBJECT", "retain_relationship", "magazine", false, "Native configuration used by magazine");
		policy.AddRule("MagazineComponent", "AmmoConfig\tRESOURCE_NAME", "retain_relationship", "magazine", true, "Native configuration used by magazine");
		policy.AddRule("MagazineComponent", "AmmoMapping\tINTEGER_ARRAY|Enabled\tBOOLEAN|MaxAmmo\tINTEGER", "retain_value", "magazine", false, "Native configuration used by magazine");
		policy.AddRule("MagazineComponent", "components\tOBJECT_ARRAY", "traverse_required_container", "magazine", false, "Native configuration used by magazine");
		policy.AddClass("MagazineConfig", "magazine");
		policy.AddRule("MagazineConfig", "AmmoResourceArray\tRESOURCE_NAME_ARRAY", "retain_relationship", "magazine", true, "Native configuration used by magazine");
		policy.AddClass("SCR_AmmoTypeInfoConfig", "magazine");
		policy.AddRule("SCR_AmmoTypeInfoConfig", "m_aAmmoTypes\tOBJECT_ARRAY", "retain_relationship", "magazine", false, "Native configuration used by magazine");
		policy.AddRule("SCR_AmmoTypeInfoConfig", "m_sGlowImagesetPath\tRESOURCE_NAME|m_sImagesetPath\tRESOURCE_NAME", "retain_relationship", "magazine", true, "Native configuration used by magazine");
		policy.AddClass("SCR_AmmoTypeInfoConfigEntry", "magazine");
		policy.AddRule("SCR_AmmoTypeInfoConfigEntry", "m_eAmmoType\tINTEGER|m_sDescription\tSTRING|m_sQuadName\tSTRING", "retain_value", "magazine", false, "Native configuration used by magazine");
	}
}
