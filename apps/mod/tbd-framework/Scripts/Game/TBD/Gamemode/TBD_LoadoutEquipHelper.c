/**
 * TBD_LoadoutEquipHelper.c - T-068.12 shared loadout application.
 *
 * One in-flight equip pass, extracted from the T-068.5.1 test component so the
 * dev harness (TestNPC) and the SpawnManager player path (T-068.12) run the SAME
 * proven APIs with only the log tag differing:
 *   wear/weapons: SCR_InventoryStorageManagerComponent.EquipWeapon / EquipCloth,
 *   worn-verify one settle tick later via
 *   SCR_CharacterInventoryStorageComponent.GetClothFromArea across candidate
 *   LoadoutAreaTypes (a plate carrier reports LoadoutArmoredVestSlotArea, not
 *   LoadoutVestArea) with IsRootedOn as the safety fallback; not-worn items are
 *   deleted, never silently kept.
 *   cargo (new): per {container,item,qty} row, resolve the WORN garment for the
 *   container key (vest/pants/jacket/backpack -> LoadoutAreaType ->
 *   GetClothFromArea -> its BaseInventoryStorageComponent), then
 *   TryInsertItemInStorage per unit; fallback TryInsertItem anywhere on the
 *   character (WARN); total failure deletes the spawned unit (ERROR). Cargo runs
 *   AFTER the wear verify so the target garments are actually worn.
 */

//------------------------------------------------------------------------------------------------
//! One issued equip awaiting its deferred worn-verify pass.
class TBD_PendingEquip
{
	string label;
	string resName;
	IEntity item;
	bool isWeapon;
	typename areaType; // LoadoutAreaType subclass for clothing; ignored for weapon
}

//------------------------------------------------------------------------------------------------
//! One loadout application: equip gear -> settle tick -> worn-verify -> insert cargo.
//! The owner must hold a strong ref until `IsDone()` (CallLater does not keep one).
class TBD_LoadoutApplication : Managed
{
	protected IEntity m_Character;
	protected ref TBD_SlotLoadoutStruct m_Loadout;
	protected string m_sTag;    // "[TBD][Loadout][Player]" / "[TBD][Loadout][TestNPC]"
	protected string m_sLabel;  // slot id / harness label for log context
	protected ref array<ref TBD_PendingEquip> m_aPending = {};
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
	//! Issue every gear equip, then schedule the verify+cargo pass one settle tick later
	//! (EquipCloth/EquipWeapon settle asynchronously — the T-068.5.1 finding).
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
			IssueEquip("primary", gear.primary, true,  LoadoutAreaType); // areaType unused for weapon
			IssueEquip("uniform", gear.uniform, false, LoadoutJacketArea);
			IssueEquip("vest",    gear.vest,    false, LoadoutVestArea);
			IssueEquip("helmet",  gear.helmet,  false, LoadoutHeadCoverArea);
		}

		GetGame().GetCallqueue().CallLater(Finish, 1000, false);
	}

	//------------------------------------------------------------------------------------------------
	//! Spawn the gear item and hand it to the real equip API (worn-verify happens in Finish).
	protected void IssueEquip(string label, string resName, bool isWeapon, typename areaType)
	{
		if (resName.IsEmpty())
		{
			Print(string.Format("%1 %2: skipped (empty slot) [%3]", m_sTag, label, m_sLabel));
			return; // documented skip, not a FAIL
		}

		SCR_InventoryStorageManagerComponent mgr = SCR_InventoryStorageManagerComponent.Cast(
			m_Character.FindComponent(SCR_InventoryStorageManagerComponent));
		if (!mgr)
		{
			Print(string.Format("%1 %2 FAILED: no inventory manager (%3)", m_sTag, label, resName), LogLevel.ERROR);
			return;
		}

		IEntity item = SpawnAtCharacter(resName);
		if (!item)
		{
			Print(string.Format("%1 %2 FAILED to load/spawn %3", m_sTag, label, resName), LogLevel.ERROR);
			return;
		}

		if (isWeapon)
			mgr.EquipWeapon(item);
		else
			mgr.EquipCloth(item);

		TBD_PendingEquip pending = new TBD_PendingEquip();
		pending.label = label;
		pending.resName = resName;
		pending.item = item;
		pending.isWeapon = isWeapon;
		pending.areaType = areaType;
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
	//! Deferred: worn-verify every issued equip (delete the not-worn), then insert cargo.
	protected void Finish()
	{
		VerifyEquips();
		InsertCargo();
		Print(string.Format("%1 loadout pass complete [%2]", m_sTag, m_sLabel));
		m_bDone = true;
	}

	//------------------------------------------------------------------------------------------------
	protected void VerifyEquips()
	{
		if (m_aPending.IsEmpty())
			return;

		SCR_CharacterInventoryStorageComponent charStorage;
		if (m_Character)
			charStorage = SCR_CharacterInventoryStorageComponent.Cast(
				m_Character.FindComponent(SCR_CharacterInventoryStorageComponent));
		if (!charStorage)
		{
			Print(m_sTag + " FAILED: no SCR_CharacterInventoryStorageComponent (cannot verify worn state)", LogLevel.ERROR);
			return;
		}

		foreach (TBD_PendingEquip p : m_aPending)
		{
			bool worn = false;
			string detail;

			if (p.isWeapon)
			{
				// GetCurrentWeapon on the char storage is protected — use the public
				// BaseWeaponManagerComponent instead (T-068.5.1).
				IEntity wornEnt;
				BaseWeaponManagerComponent weaponMgr = BaseWeaponManagerComponent.Cast(
					m_Character.FindComponent(BaseWeaponManagerComponent));
				if (weaponMgr)
				{
					BaseWeaponComponent weapon = weaponMgr.GetCurrentWeapon();
					if (weapon)
						wornEnt = weapon.GetOwner();
				}
				worn = (wornEnt && wornEnt == p.item) || IsRootedOn(p.item, m_Character);
				detail = "weapon";
				if (wornEnt)
					detail = "weapon=" + wornEnt.GetID().ToString();
			}
			else
			{
				// Area typenames vary per item (plate carrier = LoadoutArmoredVestSlotArea) —
				// search the expected area first, then the other body areas (Amendment 3).
				bool foundArea = false;
				string foundName;
				array<typename> candidates = {
					p.areaType,
					LoadoutJacketArea, LoadoutVestArea, LoadoutArmoredVestSlotArea,
					LoadoutHeadCoverArea, LoadoutCoverArea, LoadoutBackpackArea, LoadoutPantsArea
				};
				foreach (typename area : candidates)
				{
					if (charStorage.GetClothFromArea(area) == p.item)
					{
						foundArea = true;
						foundName = area.ToString();
						break;
					}
				}

				if (foundArea)
				{
					worn = true;
					detail = foundName + " ent=" + p.item.GetID().ToString();
				}
				else if (IsRootedOn(p.item, m_Character))
				{
					worn = true;
					detail = "rooted on character (no matching loadout area)";
				}
				else
				{
					detail = "not in any loadout area";
				}
			}

			if (worn)
			{
				Print(string.Format("%1 %2 equip OK %3 [%4]", m_sTag, p.label, p.resName, detail));
			}
			else
			{
				Print(string.Format("%1 %2 FAILED (not worn) %3 [%4]", m_sTag, p.label, p.resName, detail), LogLevel.ERROR);
				if (p.item)
					SCR_EntityHelper.DeleteEntityAndChildren(p.item);
			}
		}
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
