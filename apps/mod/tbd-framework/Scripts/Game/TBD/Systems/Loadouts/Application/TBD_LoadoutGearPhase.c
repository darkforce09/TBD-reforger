/**
 * @file TBD_LoadoutGearPhase.c
 * @brief The worn-gear phase of a loadout application: equip, poll the worn state, swap.
 *
 * Role: issues every weapon-slot and clothing equip a slot loadout authors, polls until each item
 * is worn, deletes the incumbent each verified item displaced, and hands over to the weapon phase.
 * Position: owned by `TBD_LoadoutApplication`, which calls `Begin` and `Cancel`; reports every
 * failure through the application's `Fail` and `Degrade` ledger.
 * State: the pending and verified equips of one application, on the server.  Invariants: weapon
 * rows land in the engine weapon slot the loadout names or fail by name; an incumbent is deleted
 * only once the engine has unseated it; an absent gear field keeps the kit's garment.
 */

//! One issued equip awaiting its worn verify.
class TBD_PendingEquip
{
	string label; //!< loadout row label, for example `vest` or `launcher`
	string resName; //!< the item ResourceName
	IEntity item; //!< the spawned item; null once deleted
	bool isWeapon; //!< true for a weapon-slot row, false for clothing
	typename areaType; //!< primary `LoadoutAreaType` of a clothing row; unused on weapon rows
	ref array<typename> candidateAreas = {}; //!< every area a clothing item may land in
	ref map<typename, IEntity> incumbents; //!< garment worn in each candidate area before the equip
	IEntity oldWeapon; //!< occupant of the target weapon slot before the equip; weapon rows only
	int weaponSlotIndex = -1; //!< target engine weapon slot, read again at verify; -1 on clothing rows
	BaseInventoryStorageComponent weaponStorage; //!< the body's equipped-weapon storage; weapon rows only
}

//! Equips the worn gear of one loadout application and verifies each item is worn.
class TBD_LoadoutGearPhase : Managed
{
	protected const int VERIFY_TICK_MS = 500; //!< ms between worn-verify ticks
	protected const int VERIFY_MAX_ATTEMPTS = 6; //!< worn-verify ticks before stragglers fail

	protected TBD_LoadoutApplication m_Application; //!< the owning application; weak, it owns this phase
	protected IEntity m_Character; //!< the body being dressed; null once the engine deletes it
	protected string m_sTag; //!< log tag, for example `[TBD][Loadout][Slot]`
	protected string m_sLabel; //!< slot id or harness label named on every log line
	protected ref array<ref TBD_PendingEquip> m_aPending = {}; //!< issued equips not yet verified worn
	protected ref array<ref TBD_PendingEquip> m_aVerified = {}; //!< equips verified worn

	//! Bind the phase to its application and read the body, tag and label it logs against.
	void TBD_LoadoutGearPhase(TBD_LoadoutApplication application)
	{
		m_Application = application;
		m_Character = application.GetCharacter();
		m_sTag = application.GetTag();
		m_sLabel = application.GetLabel();
	}

	//! Issue every gear equip `gear` authors, then schedule the first worn-verify tick. Weapons take
	//! the engine weapon slot ids 0 primary, 1 launcher (the second untyped long slot), 2 handgun
	//! and 3 throwable, the same ids as the Mission Creator's Arsenal weapon slots and
	//! `loadout-export.schema.json#/weapons`. Optic, magazine and attachments are not worn; the
	//! weapon phase mounts them.
	//! @param gear the authored gear; null issues nothing and still schedules the verify
	//! @authority server
	void Begin(TBD_SlotGearStruct gear)
	{
		if (gear)
		{
			IssueEquip("primary",   gear.primary,   true, LoadoutAreaType, 0);
			IssueEquip("launcher",  gear.launcher,  true, LoadoutAreaType, 1);
			IssueEquip("handgun",   gear.handgun,   true, LoadoutAreaType, 2);
			IssueEquip("throwable", gear.throwable, true, LoadoutAreaType, 3);
			IssueEquip("uniform",  gear.uniform,  false, LoadoutJacketArea);
			IssueEquip("vest",     gear.vest,     false, LoadoutVestArea);
			IssueEquip("helmet",   gear.helmet,   false, LoadoutHeadCoverArea);
			IssueEquip("pants",    gear.pants,    false, LoadoutPantsArea);
			IssueEquip("boots",    gear.boots,    false, LoadoutBootsArea);
			IssueEquip("handwear", gear.handwear, false, LoadoutHandwearSlotArea);
			IssueEquip("backpack", gear.backpack, false, LoadoutBackpackArea);
		}

		GetGame().GetCallqueue().CallLater(VerifyTick, VERIFY_TICK_MS, false, 1);
	}

	//! Delete every issued item that is not yet rooted on the body and drop all pending equips.
	//! Equipped items stay; they are deleted with the body.
	//! @authority server
	void Cancel()
	{
		foreach (TBD_PendingEquip p : m_aPending)
		{
			if (p.item && !TBD_LoadoutInventoryUtil.IsRootedOn(p.item, m_Character))
				SCR_EntityHelper.DeleteEntityAndChildren(p.item);
		}
		m_aPending.Clear();
	}

	//! Spawn one gear item and equip it. Clothing goes through `EquipCloth` after the garment worn
	//! in each candidate area is captured as its incumbent; a same-prefab garment is skipped. A
	//! weapon goes into the engine weapon slot the row names: `CanInsertItemInStorage` then
	//! `TryInsertItemInStorage`, or `CanReplaceItem` then `TryReplaceItem` when a kit weapon holds
	//! the slot. `EquipWeapon` is not used: it targets the slot in hand, so the rifle and the
	//! launcher (slots 0 and 1 are both untyped long slots) would contend for one slot, and its
	//! `EquipAny` path cannot address slot 0.
	//! @param label the row label
	//! @param resName the item; empty keeps the kit's item and issues nothing
	//! @param isWeapon true for a weapon-slot row
	//! @param areaType the clothing row's primary area
	//! @param weaponSlotIndex the engine weapon slot of a weapon row
	//! A slot the body lacks, or one that takes the item neither way, is a named blocking failure.
	protected void IssueEquip(string label, string resName, bool isWeapon, typename areaType, int weaponSlotIndex = -1)
	{
		if (resName.IsEmpty())
			return; // absent gear field: the kit's item stays

		SCR_InventoryStorageManagerComponent mgr = SCR_InventoryStorageManagerComponent.Cast(
			m_Character.FindComponent(SCR_InventoryStorageManagerComponent));
		if (!mgr)
		{
			if (isWeapon)
				LogWeaponEquipResult(weaponSlotIndex, resName, "failed");
			m_Application.Fail(label, resName, "character has no SCR_InventoryStorageManagerComponent");
			return;
		}

		TBD_PendingEquip pending = new TBD_PendingEquip();
		pending.label = label;
		pending.resName = resName;
		pending.isWeapon = isWeapon;
		pending.areaType = areaType;
		pending.weaponSlotIndex = weaponSlotIndex;
		pending.incumbents = new map<typename, IEntity>();

		if (isWeapon)
		{
			SCR_CharacterInventoryStorageComponent weaponOwnerStorage = SCR_CharacterInventoryStorageComponent.Cast(
				m_Character.FindComponent(SCR_CharacterInventoryStorageComponent));
			if (!weaponOwnerStorage)
			{
				LogWeaponEquipResult(weaponSlotIndex, resName, "failed");
				m_Application.Fail(label, resName, "character has no SCR_CharacterInventoryStorageComponent");
				return;
			}

			pending.weaponStorage = weaponOwnerStorage.GetWeaponStorage();
			if (!pending.weaponStorage)
			{
				LogWeaponEquipResult(weaponSlotIndex, resName, "failed");
				m_Application.Fail(label, resName, "character has no equipped-weapon storage");
				return;
			}

			int slotCount = pending.weaponStorage.GetSlotsCount();
			if (weaponSlotIndex < 0 || weaponSlotIndex >= slotCount)
			{
				LogWeaponEquipResult(weaponSlotIndex, resName, "failed");
				m_Application.Fail(label, resName, string.Format(
					"engine weapon slot %1 does not exist on this character (storage has %2 slot(s))",
					weaponSlotIndex, slotCount));
				return;
			}

			// The incumbent is this slot's occupant, not the weapon in hand, so four weapon rows in
			// one pass never claim the same entity.
			InventoryStorageSlot targetSlot = pending.weaponStorage.GetSlot(weaponSlotIndex);
			if (targetSlot)
				pending.oldWeapon = targetSlot.GetAttachedEntity();

			if (pending.oldWeapon && TBD_LoadoutInventoryUtil.PrefabOf(pending.oldWeapon) == resName)
			{
				Print(string.Format("%1 slot=%2 %3 swap-skipped (already in weapon slot %4) %5",
					m_sTag, m_sLabel, label, weaponSlotIndex, resName));
				LogWeaponEquipResult(weaponSlotIndex, resName, "ok");
				m_Application.CountGearApplied();
				return;
			}
		}
		else
		{
			SCR_CharacterInventoryStorageComponent charStorage = SCR_CharacterInventoryStorageComponent.Cast(
				m_Character.FindComponent(SCR_CharacterInventoryStorageComponent));
			if (charStorage)
			{
				TBD_LoadoutInventoryUtil.AreasForLabel(label, areaType, pending.candidateAreas);
				foreach (typename area : pending.candidateAreas)
				{
					IEntity worn = charStorage.GetClothFromArea(area);
					if (!worn)
						continue;
					pending.incumbents.Insert(area, worn);
					if (TBD_LoadoutInventoryUtil.PrefabOf(worn) == resName)
					{
						Print(string.Format("%1 slot=%2 %3 swap-skipped (already worn) %4", m_sTag, m_sLabel, label, resName));
						m_Application.CountGearApplied();
						return;
					}
				}
			}
		}

		IEntity item = m_Application.SpawnAtCharacter(resName);
		if (!item)
		{
			if (isWeapon)
				LogWeaponEquipResult(weaponSlotIndex, resName, "failed");
			m_Application.Fail(label, resName, "prefab failed to load/spawn (bad or missing asset)");
			return;
		}
		pending.item = item;

		if (isWeapon)
		{
			// The named slot, else replace its occupant; never another slot.
			if (mgr.CanInsertItemInStorage(item, pending.weaponStorage, weaponSlotIndex))
			{
				mgr.TryInsertItemInStorage(item, pending.weaponStorage, weaponSlotIndex);
				LogWeaponEquipResult(weaponSlotIndex, resName, "ok");
			}
			else if (mgr.CanReplaceItem(item, pending.weaponStorage, weaponSlotIndex))
			{
				mgr.TryReplaceItem(item, pending.weaponStorage, weaponSlotIndex);
				LogWeaponEquipResult(weaponSlotIndex, resName, "replaced");
			}
			else
			{
				LogWeaponEquipResult(weaponSlotIndex, resName, "failed");
				m_Application.Fail(label, resName, string.Format(
					"engine weapon slot %1 accepts this item neither by insert nor by replace", weaponSlotIndex));
				SCR_EntityHelper.DeleteEntityAndChildren(item);
				return;
			}
		}
		else
		{
			mgr.EquipCloth(item);
		}

		m_aPending.Insert(pending);
	}

	//! Whether `ent` is an item this phase spawned (pending or verified); guards the swap delete
	//! against two rows landing in the same area.
	//! @return true for an own item
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

	//! One worn-verify tick: each verified item deletes its displaced incumbent and leaves the
	//! pending list. When nothing is pending or the attempts run out, stragglers fail and are
	//! deleted, and the application starts the weapon phase. A deleted body cancels the pass.
	//! @param attempt 1-based tick number
	//! @authority server
	protected void VerifyTick(int attempt)
	{
		if (!m_Application || m_Application.IsDone())
			return;
		if (!m_Character)
		{
			// The body was deleted between ticks (engine double-spawn): a cancel, not a failure.
			m_Application.Cancel("body superseded");
			return;
		}

		SCR_CharacterInventoryStorageComponent charStorage =
			SCR_CharacterInventoryStorageComponent.Cast(
				m_Character.FindComponent(SCR_CharacterInventoryStorageComponent));

		if (!charStorage && !m_aPending.IsEmpty())
		{
			foreach (TBD_PendingEquip broken : m_aPending)
			{
				m_Application.Fail(broken.label, broken.resName, "character has no SCR_CharacterInventoryStorageComponent (cannot verify worn state)");
			}
			m_aPending.Clear();
		}

		for (int i = m_aPending.Count() - 1; i >= 0; i--)
		{
			TBD_PendingEquip p = m_aPending[i];
			string detail;
			typename foundArea;
			bool worn = VerifyOne(charStorage, p, attempt, detail, foundArea);
			if (!worn)
				continue;

			Print(string.Format("%1 slot=%2 %3 equip OK %4 [%5]", m_sTag, m_sLabel, p.label, p.resName, detail));
			m_Application.CountGearApplied();
			SwapDelete(charStorage, p, foundArea);
			m_aVerified.Insert(p);
			m_aPending.Remove(i);
		}

		if (!m_aPending.IsEmpty() && attempt < VERIFY_MAX_ATTEMPTS)
		{
			GetGame().GetCallqueue().CallLater(VerifyTick, VERIFY_TICK_MS, false, attempt + 1);
			return;
		}

		// Attempts exhausted: stragglers fail and are deleted.
		foreach (TBD_PendingEquip straggler : m_aPending)
		{
			m_Application.Fail(straggler.label, straggler.resName, string.Format("not worn after %1 verify ticks -- deleted", VERIFY_MAX_ATTEMPTS));
			if (straggler.item)
				SCR_EntityHelper.DeleteEntityAndChildren(straggler.item);
		}
		m_aPending.Clear();

		m_Application.OnGearSettled();
	}

	//! Whether one pending equip is worn. A weapon is worn when it is attached to its named slot;
	//! on the last tick, a weapon rooted on the body elsewhere counts as degraded. Clothing is worn
	//! when a candidate area holds it, or when it is rooted on the body (area unknown).
	//! @param detail set to where the item was found
	//! @param foundArea set to the area a clothing item was found in
	//! @return true when worn
	protected bool VerifyOne(SCR_CharacterInventoryStorageComponent charStorage, TBD_PendingEquip p, int attempt, out string detail, out typename foundArea)
	{
		if (p.isWeapon)
		{
			// Only the named slot proves the weapon landed where the loadout put it.
			if (p.weaponStorage && p.weaponSlotIndex >= 0)
			{
				InventoryStorageSlot slot = p.weaponStorage.GetSlot(p.weaponSlotIndex);
				if (slot && slot.GetAttachedEntity() == p.item)
				{
					detail = string.Format("engine weapon slot %1", p.weaponSlotIndex);
					return true;
				}
			}

			// On the body but not in the named slot: accepted only on the final tick, because an
			// earlier tick may catch the item still settling.
			if (attempt >= VERIFY_MAX_ATTEMPTS && TBD_LoadoutInventoryUtil.IsRootedOn(p.item, m_Character))
			{
				detail = string.Format("rooted on character but NOT in engine weapon slot %1", p.weaponSlotIndex);
				m_Application.Degrade(p.label, p.resName, detail);
				return true;
			}

			return false;
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

		if (TBD_LoadoutInventoryUtil.IsRootedOn(p.item, m_Character))
		{
			detail = "rooted on character (no area resolution)";
			return true;
		}

		return false;
	}

	//! Delete the incumbent a verified equip displaced, once the engine has unseated it. An
	//! incumbent that is this phase's own item, or still seated, stays; a clothing item verified
	//! only by parent chain (area unknown) deletes nothing.
	protected void SwapDelete(SCR_CharacterInventoryStorageComponent charStorage, TBD_PendingEquip p, typename foundArea)
	{
		if (p.isWeapon)
		{
			IEntity old = p.oldWeapon;
			if (!old || old == p.item || IsOwnIssuedItem(old))
				return;

			// Delete only once the engine has unseated this slot's occupant; one still seated means
			// the equip landed elsewhere and the occupant is still carried.
			if (p.weaponStorage && p.weaponSlotIndex >= 0)
			{
				InventoryStorageSlot targetSlot = p.weaponStorage.GetSlot(p.weaponSlotIndex);
				if (targetSlot && targetSlot.GetAttachedEntity() == old)
				{
					Print(string.Format("%1 slot=%2 swap-deferred (weapon slot %3 still holds %4)",
						m_sTag, m_sLabel, p.weaponSlotIndex, TBD_LoadoutInventoryUtil.PrefabOf(old)));
					return;
				}
			}

			Print(string.Format("%1 slot=%2 swapped area=weapon%3 out=%4 in=%5",
				m_sTag, m_sLabel, p.weaponSlotIndex, TBD_LoadoutInventoryUtil.PrefabOf(old), p.resName));
			SCR_EntityHelper.DeleteEntityAndChildren(old);
			return;
		}

		if (!foundArea)
		{
			// Verified by parent chain only: the landing area is unknown, so nothing is deleted.
			Print(string.Format("%1 slot=%2 swap-deferred (no area resolution) %3", m_sTag, m_sLabel, p.resName));
			return;
		}

		IEntity incumbent;
		if (!p.incumbents.Find(foundArea, incumbent) || !incumbent)
			return; // the area was empty before the equip
		if (incumbent == p.item || IsOwnIssuedItem(incumbent))
			return;
		// Delete only once the engine has unseated it.
		if (charStorage && charStorage.GetClothFromArea(foundArea) == incumbent)
			return;

		Print(string.Format("%1 slot=%2 swapped area=%3 out=%4 in=%5", m_sTag, m_sLabel, foundArea.ToString(), TBD_LoadoutInventoryUtil.PrefabOf(incumbent), p.resName));
		SCR_EntityHelper.DeleteEntityAndChildren(incumbent);
	}

	//! Write the per-weapon-slot equip line `[TBD][Equip] slot=<n> weapon=<res> result=<ok|replaced|failed>`;
	//! the format is fixed because the gates grep it.
	protected static void LogWeaponEquipResult(int slotIndex, string resName, string result)
	{
		Print(string.Format("[TBD][Equip] slot=%1 weapon=%2 result=%3", slotIndex, resName, result));
	}
}
