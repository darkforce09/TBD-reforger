/**
 * @file TBD_LoadoutCargoPhase.c
 * @brief The cargo phase of a loadout application: insert each cargo row into its container.
 *
 * Role: resolves each cargo row's container to the garment the body wears now and inserts the
 * row's units there, falling back to any storage on the body.  Position: owned by
 * `TBD_LoadoutApplication`, which calls `InsertCargo` once the gear and weapon phases settle;
 * reports through the application's `Fail` and `Degrade` ledger.
 * State: none beyond the back-reference; runs synchronously on the server.  Invariants: every
 * insert is preceded by the matching `Can*` check; a refused unit is deleted and ends its row;
 * a prefab that does not load is blocking, a full body is not.
 */

//! Inserts the cargo rows of one loadout application.
class TBD_LoadoutCargoPhase : Managed
{
	protected TBD_LoadoutApplication m_Application; //!< the owning application; weak, it owns this phase
	protected IEntity m_Character; //!< the body being filled; null once the engine deletes it
	protected TBD_SlotLoadoutStruct m_Loadout; //!< the authored loadout; owned by the application
	protected string m_sTag; //!< log tag, for example `[TBD][Loadout][Slot]`
	protected string m_sLabel; //!< slot id or harness label named on every log line

	//! Bind the phase to its application and read the body, loadout, tag and label.
	void TBD_LoadoutCargoPhase(TBD_LoadoutApplication application)
	{
		m_Application = application;
		m_Character = application.GetCharacter();
		m_Loadout = application.GetLoadout();
		m_sTag = application.GetTag();
		m_sLabel = application.GetLabel();
	}

	//! The garment worn for a cargo container key: `vest` (the vest area, else the armored-vest
	//! area), `jacket`, `pants` or `backpack`.
	//! @return the worn garment, or null for an unknown key or an empty area
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

	//! Insert every cargo row. Each unit tries the authored container (`CanInsertItemInStorage`,
	//! then `TryInsertItemInStorage`), then any storage on the body (`CanInsertItem`, then
	//! `TryInsertItem`, degraded). A unit neither accepts is deleted and the rest of its row is
	//! abandoned. A row whose container the kit does not wear is degraded up front and still
	//! inserted through the fallback.
	//! @authority server
	void InsertCargo()
	{
		if (!m_Loadout.cargo || m_Loadout.cargo.IsEmpty())
			return;

		SCR_InventoryStorageManagerComponent mgr = SCR_InventoryStorageManagerComponent.Cast(
			m_Character.FindComponent(SCR_InventoryStorageManagerComponent));
		SCR_CharacterInventoryStorageComponent charStorage = SCR_CharacterInventoryStorageComponent.Cast(
			m_Character.FindComponent(SCR_CharacterInventoryStorageComponent));
		if (!mgr || !charStorage)
		{
			foreach (TBD_SlotCargoStruct broken : m_Loadout.cargo)
			{
				if (broken)
					m_Application.Fail("cargo:" + broken.container, broken.item, "character missing inventory components");
			}
			return;
		}

		foreach (TBD_SlotCargoStruct row : m_Loadout.cargo)
		{
			if (!row)
				continue;
			if (row.item.IsEmpty())
			{
				m_Application.Fail("cargo:" + row.container, "<empty>", "cargo row carries no item ResourceName");
				continue;
			}
			if (row.qty < 1)
			{
				m_Application.Fail("cargo:" + row.container, row.item, string.Format("cargo row qty=%1 is below the schema minimum of 1", row.qty));
				continue;
			}

			IEntity garment = GarmentForContainer(charStorage, row.container);
			BaseInventoryStorageComponent storage;
			if (garment)
				storage = BaseInventoryStorageComponent.Cast(garment.FindComponent(BaseInventoryStorageComponent));
			// A container the kit does not wear is a mission and kit authoring mismatch, not a mod
			// fault: the item still goes in through the fallback, and the row counts as degraded.
			if (!storage)
				m_Application.Degrade("cargo:" + row.container, row.item,
					string.Format("this slot's kit wears no %1 -- mission/kit authoring mismatch, NOT a mod fault; the item is still inserted via the any-storage fallback", row.container));

			int inserted = 0;
			string stopReason;
			// A prefab that does not load is blocking (the document names a missing asset); a body
			// with no room left is not.
			bool stopBlocking = true;
			for (int u = 0; u < row.qty; u++)
			{
				IEntity item = m_Application.SpawnAtCharacter(row.item);
				if (!item)
				{
					stopReason = string.Format("prefab failed to load/spawn at unit %1/%2 (bad or missing asset)", u + 1, row.qty);
					break; // a prefab that does not load fails for every later unit too
				}

				bool ok = false;
				// Authored container: `Try*` runs only after `Can*` accepts.
				if (storage && mgr.CanInsertItemInStorage(item, storage))
					ok = mgr.TryInsertItemInStorage(item, storage);

				if (!ok)
				{
					// Any-storage fallback, also gated by `Can*`.
					if (mgr.CanInsertItem(item))
					{
						ok = mgr.TryInsertItem(item);
						if (ok && storage)
							m_Application.Degrade("cargo:" + row.container, row.item, string.Format(
								"unit %1/%2 did not fit the authored container (CanInsertItemInStorage=0) -- inserted elsewhere",
								u + 1, row.qty));
					}
				}

				if (ok)
				{
					inserted++;
				}
				else
				{
					SCR_EntityHelper.DeleteEntityAndChildren(item);
					// Capacity, not a broken document: the inserted units stay and the body carries
					// less than authored. Not blocking.
					stopBlocking = false;
					if (storage)
						stopReason = string.Format(
							"authored container refused unit %1/%2 (CanInsertItemInStorage=0) and no other storage would accept it -- deleted; remaining qty of this row abandoned",
							u + 1, row.qty);
					else
						stopReason = string.Format(
							"no storage would accept unit %1/%2 (CanInsertItem=0, character full) -- deleted; remaining qty of this row abandoned",
							u + 1, row.qty);
					break; // a full character won't accept later units either
				}
			}

			m_Application.CountCargoInserted(inserted);
			Print(string.Format("%1 slot=%2 cargo %3 x%4/%5 -> %6", m_sTag, m_sLabel, row.item, inserted, row.qty, row.container));
			if (!stopReason.IsEmpty())
				m_Application.Fail("cargo:" + row.container, row.item, stopReason, stopBlocking);
		}
	}
}
