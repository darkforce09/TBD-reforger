//------------------------------------------------------------------------------------------------
// TBD_AmmoProjectileExtractor.c
//
// Reads what a projectile does in flight and on impact: muzzle velocity, mass, drag and the
// ballistic table it follows; its warhead - explosive yield, penetration, fragmentation, arming
// distance, and the effect prefab it spawns; and the meshes it renders as round and cartridge.
//
// Categorization keys off the move component a projectile carries, which is how the engine
// distinguishes a bullet from a shell, a rocket, a grenade or a flare.
//------------------------------------------------------------------------------------------------

class TBD_AmmoProjectileExtractor
{
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
							outBallistics.m_sBallisticTableConfig = TBD_EquipmentResourceNames.ResolveCanonicalResourceName(bt);
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
										outWarhead.m_sEffectPrefab = TBD_EquipmentResourceNames.ResolveCanonicalResourceName(ep);
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
					outVisuals.m_sProjectileModel = TBD_EquipmentResourceNames.ResolveCanonicalResourceName(pm);
			}

			if (outVisuals.m_sCartridgeModel.IsEmpty())
			{
				string cm;
				if (cur.Get("CartridgeModel", cm) && !cm.IsEmpty())
					outVisuals.m_sCartridgeModel = TBD_EquipmentResourceNames.ResolveCanonicalResourceName(cm);
			}

			cur = cur.GetAncestor();
		}
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
}
