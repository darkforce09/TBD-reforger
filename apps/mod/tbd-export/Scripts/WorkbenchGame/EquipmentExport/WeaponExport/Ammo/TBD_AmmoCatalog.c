//------------------------------------------------------------------------------------------------
// TBD_AmmoCatalog.c
//
// The classified rows one ammunition scan produced: magazines bucketed by caliber, projectiles
// bucketed by kind, and a rollup of each half.
//
// This is the hand-off between discovery and serialization. TBD_AmmoScanner finds prefabs and
// inserts them here; TBD_AmmoCatalogWriter reads the buckets and writes the catalog files. Holding
// them in one carrier rather than on the scanner is what lets the writer stay stateless and take
// everything it needs as a single argument.
//
// A row is inserted into its category bucket before the rollup, and the writer emits rows in
// insertion order, so the order rows arrive in is the order they appear in the exported JSON.
//------------------------------------------------------------------------------------------------

class TBD_AmmoCatalog
{
	// Magazine category buckets
	ref array<ref TBD_MagazineInfo> m_aRifleMags = {};
	ref array<ref TBD_MagazineInfo> m_aMgMags = {};
	ref array<ref TBD_MagazineInfo> m_aHandgunMags = {};
	ref array<ref TBD_MagazineInfo> m_aHeavyMags = {};
	ref array<ref TBD_MagazineInfo> m_aGrenadeMags = {};
	ref array<ref TBD_MagazineInfo> m_aRocketMags = {};
	ref array<ref TBD_MagazineInfo> m_aAllMags = {};

	// Projectile category buckets
	ref array<ref TBD_ProjectileInfo> m_aBulletProjectiles = {};
	ref array<ref TBD_ProjectileInfo> m_aHeavyProjectiles = {};
	ref array<ref TBD_ProjectileInfo> m_aGrenadeProjectiles = {};
	ref array<ref TBD_ProjectileInfo> m_aRocketProjectiles = {};
	ref array<ref TBD_ProjectileInfo> m_aMortarProjectiles = {};
	ref array<ref TBD_ProjectileInfo> m_aAllProjectiles = {};

	//------------------------------------------------------------------------------------------------
	//! Drop every row, leaving the catalog ready for another scan.
	void Clear()
	{
		m_aRifleMags.Clear();
		m_aMgMags.Clear();
		m_aHandgunMags.Clear();
		m_aHeavyMags.Clear();
		m_aGrenadeMags.Clear();
		m_aRocketMags.Clear();
		m_aAllMags.Clear();

		m_aBulletProjectiles.Clear();
		m_aHeavyProjectiles.Clear();
		m_aGrenadeProjectiles.Clear();
		m_aRocketProjectiles.Clear();
		m_aMortarProjectiles.Clear();
		m_aAllProjectiles.Clear();
	}

	//------------------------------------------------------------------------------------------------
	//! File a magazine under its caliber category and in the rollup. An unrecognized category
	//! falls back to rifle, which is the bucket the caliber classifier itself defaults to.
	void InsertMagazine(string category, TBD_MagazineInfo mag)
	{
		if (category == "magazines_rifle") m_aRifleMags.Insert(mag);
		else if (category == "magazines_mg") m_aMgMags.Insert(mag);
		else if (category == "magazines_handgun") m_aHandgunMags.Insert(mag);
		else if (category == "magazines_heavy") m_aHeavyMags.Insert(mag);
		else if (category == "magazines_grenades") m_aGrenadeMags.Insert(mag);
		else if (category == "magazines_rockets") m_aRocketMags.Insert(mag);
		else m_aRifleMags.Insert(mag);

		m_aAllMags.Insert(mag);
	}

	//------------------------------------------------------------------------------------------------
	//! File a projectile under its category and in the rollup. An unrecognized category falls back
	//! to bullets, which is the bucket the move-component classifier itself defaults to.
	void InsertProjectile(string category, TBD_ProjectileInfo proj)
	{
		if (category == "projectiles_bullets") m_aBulletProjectiles.Insert(proj);
		else if (category == "projectiles_heavy") m_aHeavyProjectiles.Insert(proj);
		else if (category == "projectiles_grenades") m_aGrenadeProjectiles.Insert(proj);
		else if (category == "projectiles_rockets") m_aRocketProjectiles.Insert(proj);
		else if (category == "projectiles_mortar") m_aMortarProjectiles.Insert(proj);
		else m_aBulletProjectiles.Insert(proj);

		m_aAllProjectiles.Insert(proj);
	}

	//------------------------------------------------------------------------------------------------
	int MagazineCount()
	{
		return m_aAllMags.Count();
	}

	//------------------------------------------------------------------------------------------------
	int ProjectileCount()
	{
		return m_aAllProjectiles.Count();
	}
}
