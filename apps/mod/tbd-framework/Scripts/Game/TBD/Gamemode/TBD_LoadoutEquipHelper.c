/**
 * TBD_LoadoutEquipHelper.c - shared loadout application (T-068.12, reworked A2).
 *
 * One in-flight equip pass, shared by the dev harness (TestNPC) and the SpawnManager
 * player path so both run the SAME proven APIs with only the log tag differing.
 *
 * A2 determinism rework:
 *  - poll-until-worn VERIFY (500 ms ticks, 6 attempts) replaces the old fixed +1000 ms
 *    one-shot that deleted slow-settling equips;
 *  - DETERMINISTIC SWAP: incumbents (kit garments / current weapon) are captured
 *    BEFORE the equip; when the new item verifies worn, the displaced incumbent is
 *    deleted WITH its contents (deliberate: Arsenal cargo is the contents source of
 *    truth) and logged `swapped`; same-prefab equips are skipped up front
 *    (`swap-skipped`) so the verify can't mistake the old item for the new one;
 *    `IsRootedOn`-only verifies log `swap-deferred` and never guess-delete;
 *  - cargo runs strictly AFTER the wear verify (FinishRest) so container resolution
 *    sees the NEW garments.
 */

//------------------------------------------------------------------------------------------------
//! One issued equip awaiting its deferred worn-verify pass.
class TBD_PendingEquip
{
	string label;
	string resName;
	IEntity item;
	bool isWeapon;
	typename areaType;                       // primary LoadoutAreaType for clothing
	ref array<typename> candidateAreas = {}; // all areas this item may land in
	ref map<typename, IEntity> incumbents;   // pre-equip worn garment per candidate area
	IEntity oldWeapon;                       // pre-equip current weapon (weapon rows)
}

//------------------------------------------------------------------------------------------------
//! One loadout application: equip gear -> poll worn-verify (+swap) -> cargo.
//! The owner must hold a strong ref until `IsDone()` (CallLater does not keep one).
class TBD_LoadoutApplication : Managed
{
	protected const int VERIFY_TICK_MS = 500;
	protected const int VERIFY_MAX_ATTEMPTS = 6;

	protected IEntity m_Character;
	protected ref TBD_SlotLoadoutStruct m_Loadout;
	protected string m_sTag;    // "[TBD][Loadout][Player]" / "[TBD][Loadout][TestNPC]"
	protected string m_sLabel;  // slot id / harness label for log context
	protected ref array<ref TBD_PendingEquip> m_aPending = {};
	protected ref array<ref TBD_PendingEquip> m_aVerified = {};
	protected bool m_bDone;

	//------------------------------------------------------------------------------------------------
	void TBD_LoadoutApplication(IEntity character, TBD_SlotLoadoutStruct loadout, string tag, string label)
	{
		m_Character = character;
		m_Loadout = loadout;
		m_sTag = tag;
		m_sLabel = label;
	}

	//------------------------------------------------------------------------------------------------
	bool IsDone()
	{
		return m_bDone;
	}

	//------------------------------------------------------------------------------------------------
	IEntity GetCharacter()
	{
		return m_Character;
	}

	//------------------------------------------------------------------------------------------------
	//! A2 — abort an in-flight application whose body was reaped (vanilla
	//! double-spawn): loose not-yet-rooted spawned items are deleted; equipped ones
	//! die with the body. Idempotent.
	void Cancel(string reason)
	{
		if (m_bDone)
			return;
		foreach (TBD_PendingEquip p : m_aPending)
		{
			if (p.item && !IsRootedOn(p.item, m_Character))
				SCR_EntityHelper.DeleteEntityAndChildren(p.item);
		}
		m_aPending.Clear();
		Print(string.Format("%1 loadout application cancelled (%2) [%3]", m_sTag, reason, m_sLabel));
		m_bDone = true;
	}

	//------------------------------------------------------------------------------------------------
	//! Issue every gear equip, then start the poll-verify (EquipCloth/EquipWeapon
	//! settle asynchronously — the T-068.5.1 finding; A2 polls instead of guessing).
	void Run()
	{
		if (!m_Character || !m_Loadout)
		{
			m_bDone = true;
			return;
		}

		TBD_SlotGearStruct gear = m_Loadout.gear;
		if (gear)
		{
			IssueEquip("primary",  gear.primary,  true,  LoadoutAreaType); // areaType unused for weapon
			IssueEquip("uniform",  gear.uniform,  false, LoadoutJacketArea);
			IssueEquip("vest",     gear.vest,     false, LoadoutVestArea);
			IssueEquip("helmet",   gear.helmet,   false, LoadoutHeadCoverArea);
			// A3 — the wear map arrives complete now.
			IssueEquip("pants",    gear.pants,    false, LoadoutPantsArea);
			IssueEquip("boots",    gear.boots,    false, LoadoutBootsArea);
			IssueEquip("handwear", gear.handwear, false, LoadoutHandwearSlotArea);
			IssueEquip("backpack", gear.backpack, false, LoadoutBackpackArea);
		}

		GetGame().GetCallqueue().CallLater(VerifyTick, VERIFY_TICK_MS, false, 1);
	}

	//------------------------------------------------------------------------------------------------
	//! Candidate landing areas per clothing label. EquipCloth routes by the ITEM's own
	//! AreaType (locked landmine, t068_10_4), so a "vest" pick may land in the armored
	//! area (plate carrier) — capture + verify must cover every candidate.
	protected static void AreasForLabel(string label, typename primaryArea, notnull array<typename> outAreas)
	{
		outAreas.Insert(primaryArea);
		if (label == "vest")
			outAreas.Insert(LoadoutArmoredVestSlotArea);
	}

	//------------------------------------------------------------------------------------------------
	//! Resolved prefab ResourceName of a spawned entity ("" when unresolvable).
	protected static string PrefabOf(IEntity ent)
	{
		if (!ent)
			return string.Empty;
		EntityPrefabData pd = ent.GetPrefabData();
		if (!pd)
			return string.Empty;
		return pd.GetPrefabName();
	}

	//------------------------------------------------------------------------------------------------
	//! Spawn the gear item and hand it to the real equip API; capture displaced
	//! incumbents first (deterministic swap) and skip same-prefab re-equips.
	protected void IssueEquip(string label, string resName, bool isWeapon, typename areaType)
	{
		if (resName.IsEmpty())
			return; // absent gear slot — kit garment (if any) is deliberately retained

		SCR_InventoryStorageManagerComponent mgr = SCR_InventoryStorageManagerComponent.Cast(
			m_Character.FindComponent(SCR_InventoryStorageManagerComponent));
		if (!mgr)
		{
			Print(string.Format("%1 %2 FAILED: no inventory manager (%3)", m_sTag, label, resName), LogLevel.ERROR);
			return;
		}

		TBD_PendingEquip pending = new TBD_PendingEquip();
		pending.label = label;
		pending.resName = resName;
		pending.isWeapon = isWeapon;
		pending.areaType = areaType;
		pending.incumbents = new map<typename, IEntity>();

		// --- capture incumbents / same-prefab short-circuit -------------------------------
		if (isWeapon)
		{
			BaseWeaponManagerComponent weaponMgr = BaseWeaponManagerComponent.Cast(
				m_Character.FindComponent(BaseWeaponManagerComponent));
			if (weaponMgr)
			{
				BaseWeaponComponent cur = weaponMgr.GetCurrentWeapon();
				if (cur)
					pending.oldWeapon = cur.GetOwner();
			}
			if (pending.oldWeapon && PrefabOf(pending.oldWeapon) == resName)
			{
				Print(string.Format("%1 %2 swap-skipped (already worn) %3 [%4]", m_sTag, label, resName, m_sLabel));
				return;
			}
		}
		else
		{
			SCR_CharacterInventoryStorageComponent charStorage = SCR_CharacterInventoryStorageComponent.Cast(
				m_Character.FindComponent(SCR_CharacterInventoryStorageComponent));
			if (charStorage)
			{
				AreasForLabel(label, areaType, pending.candidateAreas);
				foreach (typename area : pending.candidateAreas)
				{
					IEntity worn = charStorage.GetClothFromArea(area);
					if (!worn)
						continue;
					pending.incumbents.Insert(area, worn);
					if (PrefabOf(worn) == resName)
					{
						Print(string.Format("%1 %2 swap-skipped (already worn) %3 [%4]", m_sTag, label, resName, m_sLabel));
						return;
					}
				}
			}
		}

		IEntity item = SpawnAtCharacter(resName);
		if (!item)
		{
			Print(string.Format("%1 %2 FAILED to load/spawn %3", m_sTag, label, resName), LogLevel.ERROR);
			return;
		}
		pending.item = item;

		if (isWeapon)
			mgr.EquipWeapon(item);
		else
			mgr.EquipCloth(item);

		m_aPending.Insert(pending);
	}

	//------------------------------------------------------------------------------------------------
	protected IEntity SpawnAtCharacter(string resName)
	{
		Resource resource = Resource.Load(resName);
		if (!resource || !resource.IsValid())
			return null;

		EntitySpawnParams params = new EntitySpawnParams();
		params.TransformMode = ETransformMode.WORLD;
		Math3D.MatrixIdentity4(params.Transform);
		params.Transform[3] = m_Character.GetOrigin();
		return GetGame().SpawnEntityPrefab(resource, GetGame().GetWorld(), params);
	}

	//------------------------------------------------------------------------------------------------
	//! True if entity's parent chain roots at the given character (attached/worn, not loose).
	protected static bool IsRootedOn(IEntity entity, IEntity root)
	{
		IEntity cur = entity;
		while (cur)
		{
			if (cur == root)
				return true;
			cur = cur.GetParent();
		}
		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! True when `ent` is one of the items THIS application spawned (pending or
	//! verified) — guards the swap-delete against mis-authored same-area collisions.
	protected bool IsOwnIssuedItem(IEntity ent)
	{
		foreach (TBD_PendingEquip p : m_aPending)
		{
			if (p.item == ent)
				return true;
		}
		foreach (TBD_PendingEquip v : m_aVerified)
		{
			if (v.item == ent)
				return true;
		}
		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! Poll pass: verify pending equips; verified items trigger their swap-delete and
	//! move out. All settled (or attempts exhausted) → FinishRest.
	protected void VerifyTick(int attempt)
	{
		if (m_bDone)
			return;
		if (!m_Character)
		{
			// The body was reaped between ticks (vanilla double-spawn) — a clean
			// cancel, not an equip failure.
			Cancel("body superseded");
			return;
		}

		SCR_CharacterInventoryStorageComponent charStorage =
			SCR_CharacterInventoryStorageComponent.Cast(
				m_Character.FindComponent(SCR_CharacterInventoryStorageComponent));

		if (!charStorage && !m_aPending.IsEmpty())
		{
			Print(m_sTag + " FAILED: no SCR_CharacterInventoryStorageComponent (cannot verify worn state)", LogLevel.ERROR);
			m_aPending.Clear();
		}

		for (int i = m_aPending.Count() - 1; i >= 0; i--)
		{
			TBD_PendingEquip p = m_aPending[i];
			string detail;
			typename foundArea;
			bool worn = VerifyOne(charStorage, p, detail, foundArea);
			if (!worn)
				continue;

			Print(string.Format("%1 %2 equip OK %3 [%4]", m_sTag, p.label, p.resName, detail));
			SwapDelete(charStorage, p, foundArea);
			m_aVerified.Insert(p);
			m_aPending.Remove(i);
		}

		if (!m_aPending.IsEmpty() && attempt < VERIFY_MAX_ATTEMPTS)
		{
			GetGame().GetCallqueue().CallLater(VerifyTick, VERIFY_TICK_MS, false, attempt + 1);
			return;
		}

		// Attempts exhausted: stragglers are honestly failed + removed (never worn).
		foreach (TBD_PendingEquip p : m_aPending)
		{
			Print(string.Format("%1 %2 FAILED (not worn after %3 ticks) %4", m_sTag, p.label, VERIFY_MAX_ATTEMPTS, p.resName), LogLevel.ERROR);
			if (p.item)
				SCR_EntityHelper.DeleteEntityAndChildren(p.item);
		}
		m_aPending.Clear();

		FinishRest();
	}

	//------------------------------------------------------------------------------------------------
	//! Worn check for one pending equip (same signals as T-068.5.1: current weapon /
	//! GetClothFromArea across candidates / IsRootedOn fallback).
	protected bool VerifyOne(SCR_CharacterInventoryStorageComponent charStorage, TBD_PendingEquip p, out string detail, out typename foundArea)
	{
		if (p.isWeapon)
		{
			IEntity wornEnt;
			BaseWeaponManagerComponent weaponMgr = BaseWeaponManagerComponent.Cast(
				m_Character.FindComponent(BaseWeaponManagerComponent));
			if (weaponMgr)
			{
				BaseWeaponComponent weapon = weaponMgr.GetCurrentWeapon();
				if (weapon)
					wornEnt = weapon.GetOwner();
			}
			bool worn = (wornEnt && wornEnt == p.item) || IsRootedOn(p.item, m_Character);
			detail = "weapon";
			return worn;
		}

		if (!charStorage)
			return false;

		foreach (typename area : p.candidateAreas)
		{
			if (charStorage.GetClothFromArea(area) == p.item)
			{
				detail = area.ToString() + " ent=" + p.item.GetID().ToString();
				foundArea = area;
				return true;
			}
		}

		if (IsRootedOn(p.item, m_Character))
		{
			detail = "rooted on character (no area resolution)";
			return true;
		}

		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! Delete the displaced incumbent of a freshly verified equip (deterministic swap).
	protected void SwapDelete(SCR_CharacterInventoryStorageComponent charStorage, TBD_PendingEquip p, typename foundArea)
	{
		if (p.isWeapon)
		{
			IEntity old = p.oldWeapon;
			if (!old || old == p.item || IsOwnIssuedItem(old))
				return;
			Print(string.Format("%1 swapped area=weapon out=%2 in=%3 [%4]", m_sTag, PrefabOf(old), p.resName, m_sLabel));
			SCR_EntityHelper.DeleteEntityAndChildren(old);
			return;
		}

		if (!foundArea)
		{
			// IsRootedOn-only verify — landing area unknown; never guess-delete.
			Print(string.Format("%1 swap-deferred (no area resolution) %2 [%3]", m_sTag, p.resName, m_sLabel));
			return;
		}

		IEntity incumbent;
		if (!p.incumbents.Find(foundArea, incumbent) || !incumbent)
			return; // area was empty pre-equip — nothing displaced
		if (incumbent == p.item || IsOwnIssuedItem(incumbent))
			return;
		// Belt-and-braces: only delete once the engine really unseated it.
		if (charStorage && charStorage.GetClothFromArea(foundArea) == incumbent)
			return;

		Print(string.Format("%1 swapped area=%2 out=%3 in=%4 [%5]", m_sTag, foundArea.ToString(), PrefabOf(incumbent), p.resName, m_sLabel));
		SCR_EntityHelper.DeleteEntityAndChildren(incumbent);
	}

	//------------------------------------------------------------------------------------------------
	//! Post-verify tail: cargo insert (against the NEW garments) + completion line.
	protected void FinishRest()
	{
		InsertCargo();
		Print(string.Format("%1 loadout pass complete [%2]", m_sTag, m_sLabel));
		m_bDone = true;
	}

	//------------------------------------------------------------------------------------------------
	//! Container key -> the WORN garment entity (vest also accepts the armored-vest area).
	protected IEntity GarmentForContainer(SCR_CharacterInventoryStorageComponent charStorage, string container)
	{
		if (container == "vest")
		{
			IEntity worn = charStorage.GetClothFromArea(LoadoutVestArea);
			if (!worn)
				worn = charStorage.GetClothFromArea(LoadoutArmoredVestSlotArea);
			return worn;
		}
		if (container == "jacket")
			return charStorage.GetClothFromArea(LoadoutJacketArea);
		if (container == "pants")
			return charStorage.GetClothFromArea(LoadoutPantsArea);
		if (container == "backpack")
			return charStorage.GetClothFromArea(LoadoutBackpackArea);
		return null;
	}

	//------------------------------------------------------------------------------------------------
	//! Insert every cargo row into its resolved container storage. Failure ladder per unit:
	//! targeted TryInsertItemInStorage -> TryInsertItem anywhere (WARN) -> delete (ERROR).
	//! No silent drops: every row logs an outcome.
	protected void InsertCargo()
	{
		if (!m_Loadout.cargo || m_Loadout.cargo.IsEmpty())
			return;

		SCR_InventoryStorageManagerComponent mgr = SCR_InventoryStorageManagerComponent.Cast(
			m_Character.FindComponent(SCR_InventoryStorageManagerComponent));
		SCR_CharacterInventoryStorageComponent charStorage = SCR_CharacterInventoryStorageComponent.Cast(
			m_Character.FindComponent(SCR_CharacterInventoryStorageComponent));
		if (!mgr || !charStorage)
		{
			Print(m_sTag + " cargo FAILED: character missing inventory components", LogLevel.ERROR);
			return;
		}

		foreach (TBD_SlotCargoStruct row : m_Loadout.cargo)
		{
			if (!row || row.item.IsEmpty() || row.qty < 1)
				continue;

			IEntity garment = GarmentForContainer(charStorage, row.container);
			BaseInventoryStorageComponent storage;
			if (garment)
				storage = BaseInventoryStorageComponent.Cast(garment.FindComponent(BaseInventoryStorageComponent));
			if (!storage)
				Print(string.Format("%1 cargo %2: no worn '%3' storage — falling back to any-storage insert",
					m_sTag, row.item, row.container), LogLevel.WARNING);

			int inserted = 0;
			for (int u = 0; u < row.qty; u++)
			{
				IEntity item = SpawnAtCharacter(row.item);
				if (!item)
				{
					Print(string.Format("%1 cargo FAILED to load/spawn %2 (unit %3/%4)",
						m_sTag, row.item, u + 1, row.qty), LogLevel.ERROR);
					break; // resource problems won't fix themselves for later units
				}

				bool ok = false;
				if (storage)
					ok = mgr.TryInsertItemInStorage(item, storage);
				if (!ok)
				{
					ok = mgr.TryInsertItem(item);
					if (ok && storage)
						Print(string.Format("%1 cargo %2 fell back to any-storage (container '%3' full?)",
							m_sTag, row.item, row.container), LogLevel.WARNING);
				}

				if (ok)
				{
					inserted++;
				}
				else
				{
					Print(string.Format("%1 cargo FAILED to insert %2 into '%3' (unit %4/%5) — deleting",
						m_sTag, row.item, row.container, u + 1, row.qty), LogLevel.ERROR);
					SCR_EntityHelper.DeleteEntityAndChildren(item);
					break; // a full character won't accept later units either
				}
			}

			Print(string.Format("%1 cargo %2 x%3/%4 -> %5 [%6]",
				m_sTag, row.item, inserted, row.qty, row.container, m_sLabel));
		}
	}
}
