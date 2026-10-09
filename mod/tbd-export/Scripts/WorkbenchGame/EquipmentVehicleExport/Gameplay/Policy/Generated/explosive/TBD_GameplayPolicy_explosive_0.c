// Generated from the authoritative gameplay field-selection policy.
class TBD_GameplayPolicy_explosive_0
{
	static void Apply(TBD_GameplaySelectionPolicy policy)
	{
		policy.AddClass("BaseExplosionDamage", "explosive");
		policy.AddRule("BaseExplosionDamage", "DamageEffect\tOBJECT_ARRAY", "retain_relationship", "explosive", false, "Native configuration used by explosive");
		policy.AddRule("BaseExplosionDamage", "DamageDistance\tSCALAR|DamageDistanceCurve\tVECTOR2_ARRAY|DamageFalloffCurve\tVECTOR2_ARRAY|DamageType\tINTEGER|DamageValue\tSCALAR|ExplosionDamagePower\tSCALAR|ExplosionEffectSpeed\tSCALAR|ExplosionRangePower\tSCALAR|PerformTraces\tBOOLEAN", "retain_value", "explosive", false, "Native configuration used by explosive");
		policy.AddClass("ExplosionDamageContainer", "explosive");
		policy.AddRule("ExplosionDamageContainer", "ExplosionEffects\tOBJECT_ARRAY", "retain_relationship", "explosive", false, "Native configuration used by explosive");
		policy.AddRule("ExplosionDamageContainer", "ChargeWeight\tSCALAR|Enabled\tBOOLEAN|ExplosionScale\tSCALAR|InteractionLayer\tFLAGS|LayerPreset\tSTRING|MaxExplosionMembersLimit\tINTEGER|TntEquivalent\tSCALAR|TriggerOnce\tBOOLEAN", "retain_value", "explosive", false, "Native configuration used by explosive");
		policy.AddClass("ExplosionFragmentationEffect", "explosive");
		policy.AddRule("ExplosionFragmentationEffect", "DamageEffect\tOBJECT_ARRAY", "retain_relationship", "explosive", false, "Native configuration used by explosive");
		policy.AddRule("ExplosionFragmentationEffect", "CaseWeight\tSCALAR|DamageDistance\tSCALAR|DamageDistanceCurve\tVECTOR2_ARRAY|DamageFalloffCurve\tVECTOR2_ARRAY|DamageFragmentCount\tINTEGER|DamageType\tINTEGER|DamageValue\tSCALAR|ExplosionDamagePower\tSCALAR|ExplosionEffectSpeed\tSCALAR|ExplosionRangePower\tSCALAR|FragDamagePower\tSCALAR|FragMassScale\tSCALAR|FragRangePower\tSCALAR|FragRangeScale\tSCALAR|FragSpeedScale\tSCALAR|GurneyConstant\tSCALAR|GurneyShape\tINTEGER|PerformTraces\tBOOLEAN", "retain_value", "explosive", false, "Native configuration used by explosive");
		policy.AddClass("ExplosionImpulseEffect", "explosive");
		policy.AddRule("ExplosionImpulseEffect", "DamageEffect\tOBJECT_ARRAY", "retain_relationship", "explosive", false, "Native configuration used by explosive");
		policy.AddRule("ExplosionImpulseEffect", "DamageDistance\tSCALAR|DamageDistanceCurve\tVECTOR2_ARRAY|DamageFalloffCurve\tVECTOR2_ARRAY|DamageType\tINTEGER|DamageValue\tSCALAR|ExplosionDamagePower\tSCALAR|ExplosionEffectSpeed\tSCALAR|ExplosionImpulseMultiplier\tSCALAR|ExplosionRangePower\tSCALAR|PerformTraces\tBOOLEAN", "retain_value", "explosive", false, "Native configuration used by explosive");
		policy.AddClass("ProjectileDamage", "explosive");
		policy.AddRule("ProjectileDamage", "DamageEffects\tOBJECT_ARRAY", "retain_relationship", "explosive", false, "Native configuration used by explosive");
		policy.AddRule("ProjectileDamage", "DamageType\tINTEGER|DamageValue\tSCALAR|Enabled\tBOOLEAN|TriggerOnce\tBOOLEAN", "retain_value", "explosive", false, "Native configuration used by explosive");
		policy.AddClass("SCR_AntiPersonnelMineCollisionHandlerComponent", "explosive");
		policy.AddRule("SCR_AntiPersonnelMineCollisionHandlerComponent", "m_aSpecialCollisions\tOBJECT_ARRAY", "retain_relationship", "explosive", false, "Native configuration used by explosive");
		policy.AddRule("SCR_AntiPersonnelMineCollisionHandlerComponent", "Enabled\tBOOLEAN|m_fContactHeightOverride\tSCALAR|m_iContactType\tINTEGER", "retain_value", "explosive", false, "Native configuration used by explosive");
		policy.AddRule("SCR_AntiPersonnelMineCollisionHandlerComponent", "components\tOBJECT_ARRAY", "traverse_required_container", "explosive", false, "Native configuration used by explosive");
		policy.AddClass("SCR_DetonatorGadgetComponent", "explosive");
		policy.AddRule("SCR_DetonatorGadgetComponent", "m_eAnimVariable\tINTEGER", "exclude", "excluded", false, "Presentation, authoring, lifecycle, replication, animation, audio, or cosmetic effect setting");
		policy.AddRule("SCR_DetonatorGadgetComponent", "Enabled\tBOOLEAN|m_bCanBeHeld\tBOOLEAN|m_eUseMask\tINTEGER|m_fClothesOffsetStrength\tSCALAR|m_fMaxDetonationRange\tSCALAR|m_fWeaponNoFireTime\tSCALAR|m_vEquipmentSlotOffset\tVECTOR3", "retain_value", "explosive", false, "Native configuration used by explosive");
		policy.AddRule("SCR_DetonatorGadgetComponent", "components\tOBJECT_ARRAY", "traverse_required_container", "explosive", false, "Native configuration used by explosive");
		policy.AddClass("SCR_ExplosionAmmoEffect", "explosive");
		policy.AddRule("SCR_ExplosionAmmoEffect", "Enabled\tBOOLEAN|TriggerOnce\tBOOLEAN", "retain_value", "explosive", false, "Native configuration used by explosive");
		policy.AddClass("SCR_ExplosiveChargeComponent", "explosive");
		policy.AddRule("SCR_ExplosiveChargeComponent", "Enabled\tBOOLEAN", "retain_value", "explosive", false, "Native configuration used by explosive");
		policy.AddRule("SCR_ExplosiveChargeComponent", "components\tOBJECT_ARRAY", "traverse_required_container", "explosive", false, "Native configuration used by explosive");
		policy.AddClass("SCR_MortarShellGadgetComponent", "explosive");
		policy.AddRule("SCR_MortarShellGadgetComponent", "m_bCanBeHeld\tBOOLEAN|m_eAnimVariable\tINTEGER|m_eUseMask\tINTEGER|m_fClothesOffsetStrength\tSCALAR|m_vEquipmentSlotOffset\tVECTOR3", "exclude", "excluded", false, "Non-gameplay setting of a mixed-purpose SCR_MortarShellGadgetComponent container");
		policy.AddRule("SCR_MortarShellGadgetComponent", "Enabled\tBOOLEAN|m_aChargeRingConfig\tVECTOR3_ARRAY|m_bIsUsingTimeFuze\tBOOLEAN|m_fDetonationAltitude\tSCALAR|m_fMaxFuzeTime\tSCALAR|m_fMinFuzeTime\tSCALAR|m_fVerticalImpactTimeOffset\tSCALAR|m_fWeaponNoFireTime\tSCALAR", "retain_value", "explosive", false, "Native configuration used by explosive");
		policy.AddRule("SCR_MortarShellGadgetComponent", "components\tOBJECT_ARRAY", "traverse_required_container", "explosive", false, "Native configuration used by explosive");
		policy.AddClass("SCR_SpecialCollisionHandlerComponent", "explosive");
		policy.AddRule("SCR_SpecialCollisionHandlerComponent", "m_aSpecialCollisions\tOBJECT_ARRAY", "retain_relationship", "explosive", false, "Native configuration used by explosive");
		policy.AddRule("SCR_SpecialCollisionHandlerComponent", "Enabled\tBOOLEAN|m_fContactHeightOverride\tSCALAR|m_iContactType\tINTEGER", "retain_value", "explosive", false, "Native configuration used by explosive");
		policy.AddRule("SCR_SpecialCollisionHandlerComponent", "components\tOBJECT_ARRAY", "traverse_required_container", "explosive", false, "Native configuration used by explosive");
	}
}
