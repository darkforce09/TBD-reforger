/**
 * @file TBD_MissionVariantFilter.c
 * @brief Reduces a freshly parsed mission document to its active variant set.
 *
 * Role: conditional inclusion of variant-gated rows: zones, ORBAT groups, vehicles, entities and
 * slots, plus the dependants of an excluded vehicle.  Position: called by
 * `TBD_MissionLoader.ParseMissionJson` after the typed parse and before `TBD_MissionValidator.Run`,
 * so validation and spawn see the effective document; the profile cache keeps the full body.
 * State: one `TBD_MissionVariantFilterContext` per run, discarded when it ends.
 * Invariants: inert (no change, no log) when the document declares no top-level `variants` key;
 * the active set is the server override when authored, else the rows with `default: true`; a row
 * with no `variantId` or an active one is kept, a declared inactive one drops silently, a dangling
 * one drops with a WARNING; an excluded vehicle takes its entity twin (uid, else alias|x|z
 * fingerprint) and its crew seats with it; a gate pass whose row count disagrees with the typed
 * parse leaves that collection unfiltered with an ERROR. `objectives[]` and `editorTriggers[]` are
 * gated by their own readers through `TBD_MissionLoader.GetActiveVariantIds`.
 */

//! Mutable state of one filter run, shared by its per-kind passes.
class TBD_MissionVariantFilterContext : Managed
{
	TBD_MissionDocumentStruct m_Document;                              //!< The document being filtered; owned by `TBD_MissionLoader`.
	ref map<string, bool> m_Declared = new map<string, bool>();        //!< Every well-formed `variants[].id`.
	ref map<string, bool> m_Active;                                    //!< The active variant set.
	ref TBD_VariantGateSkeletonStruct m_Skeleton;                      //!< Slot and vehicle gates, index-aligned; null when the pass failed.
	ref array<ref array<string>> m_CrewByVehicle;                      //!< Crew slot ids per vehicle row, index-aligned.
	ref map<string, bool> m_DroppedVehicleUids = new map<string, bool>();         //!< Uids of excluded vehicles.
	ref map<string, bool> m_DroppedVehicleFingerprints = new map<string, bool>(); //!< alias|x|z fingerprints of excluded vehicles.
	ref map<string, bool> m_DroppedCrewSlotIds = new map<string, bool>();         //!< Crew slot ids of excluded vehicles.
	int m_iZonesBefore;     //!< Zone rows before filtering; default 0.
	int m_iZonesAfter;      //!< Zone rows kept; default 0.
	int m_iGroupsBefore;    //!< ORBAT groups before filtering; default 0.
	int m_iGroupsAfter;     //!< ORBAT groups kept; default 0.
	int m_iVehiclesBefore;  //!< Vehicle rows before filtering; default 0.
	int m_iVehiclesAfter;   //!< Vehicle rows kept; default 0.
	int m_iEntitiesBefore;  //!< Entity rows before filtering; default 0.
	int m_iEntitiesAfter;   //!< Entity rows kept; default 0.
	int m_iSlotsBefore;     //!< Slot rows before filtering; default 0.
	int m_iSlotsAfter;      //!< Slot rows kept; default 0.

	//! Start a run over `document`.
	//! @param document the parsed document; never null
	void TBD_MissionVariantFilterContext(TBD_MissionDocumentStruct document)
	{
		m_Document = document;
	}
}

//! Static variant filter over a parsed mission document.
class TBD_MissionVariantFilter
{
	//! Filter `document` in place to its active variant set.
	//! @param document the freshly parsed document; null does nothing
	//! @param data the mission text `document` was parsed from
	//! @return the active variant ids, or null when the document declares no `variants` key
	static ref array<string> Apply(TBD_MissionDocumentStruct document, string data)
	{
		if (!document)
			return null;

		bool variantsKeyPresent;
		array<bool> defaultFlags = TBD_MissionVariantSources.ExtractVariantDefaultFlags(data, variantsKeyPresent);
		if (!variantsKeyPresent)
			return null;

		TBD_MissionVariantFilterContext ctx = new TBD_MissionVariantFilterContext(document);
		ctx.m_Active = ComputeActiveVariantSet(ctx, defaultFlags);

		array<string> activeIds = new array<string>();
		foreach (string activeId, bool activeUnused : ctx.m_Active)
			activeIds.Insert(activeId);

		FilterZones(ctx);
		FilterOrbatGroups(ctx);

		ctx.m_Skeleton = TBD_MissionVariantSources.ParseVariantGateSkeleton();
		ctx.m_CrewByVehicle = TBD_MissionVariantSources.ExtractVehicleCrewSlotIds(data);

		FilterVehicles(ctx);
		FilterEntities(ctx);
		FilterSlots(ctx);
		LogSummary(ctx);
		return activeIds;
	}

	//! Keep the `zones[]` rows the active set includes.
	//! @param ctx the run
	protected static void FilterZones(TBD_MissionVariantFilterContext ctx)
	{
		if (ctx.m_Document.zones)
		{
			ctx.m_iZonesBefore = ctx.m_Document.zones.Count();
			array<ref TBD_MissionZoneStruct> keptZones = new array<ref TBD_MissionZoneStruct>();
			foreach (TBD_MissionZoneStruct zone : ctx.m_Document.zones)
			{
				string zoneName = "";
				string zoneGate = "";
				if (zone)
				{
					zoneName = zone.id;
					zoneGate = zone.variantId;
				}
				if (!zone || TBD_MissionVariants.IsRowIncluded(zoneGate, ctx.m_Declared, ctx.m_Active, "zone", zoneName))
					keptZones.Insert(zone);
			}
			ctx.m_Document.zones = keptZones;
			ctx.m_iZonesAfter = keptZones.Count();
		}
	}

	//! Keep the ORBAT groups of every faction the active set includes.
	//! @param ctx the run
	protected static void FilterOrbatGroups(TBD_MissionVariantFilterContext ctx)
	{
		if (ctx.m_Document.orbat)
		{
			foreach (string factionKey, TBD_MissionOrbatFactionStruct factionOrbat : ctx.m_Document.orbat)
			{
				if (!factionOrbat || !factionOrbat.groups)
					continue;

				ctx.m_iGroupsBefore = ctx.m_iGroupsBefore + factionOrbat.groups.Count();
				array<ref TBD_MissionOrbatGroupStruct> keptGroups = new array<ref TBD_MissionOrbatGroupStruct>();
				foreach (TBD_MissionOrbatGroupStruct group : factionOrbat.groups)
				{
					string groupName = factionKey;
					string groupGate = "";
					if (group)
					{
						groupName = factionKey + ":" + group.callsign;
						groupGate = group.variantId;
					}
					if (!group || TBD_MissionVariants.IsRowIncluded(groupGate, ctx.m_Declared, ctx.m_Active, "orbat group", groupName))
						keptGroups.Insert(group);
				}
				factionOrbat.groups = keptGroups;
				ctx.m_iGroupsAfter = ctx.m_iGroupsAfter + keptGroups.Count();
			}
		}
	}

	//! Keep the `vehicles[]` rows whose skeleton gate the active set includes, and remember each
	//! excluded vehicle's join keys and crew. A skeleton that disagrees with the typed row count
	//! leaves the vehicles unfiltered with an ERROR.
	//! @param ctx the run
	protected static void FilterVehicles(TBD_MissionVariantFilterContext ctx)
	{
		if (ctx.m_Document.vehicles)
		{
			ctx.m_iVehiclesBefore = ctx.m_Document.vehicles.Count();
			ctx.m_iVehiclesAfter = ctx.m_iVehiclesBefore;
			if (ctx.m_iVehiclesBefore > 0)
			{
				int skelCount = -1;
				if (ctx.m_Skeleton && ctx.m_Skeleton.vehicles)
					skelCount = ctx.m_Skeleton.vehicles.Count();

				if (ctx.m_Skeleton && ctx.m_Skeleton.vehicles && skelCount == ctx.m_iVehiclesBefore)
				{
					array<ref TBD_MissionVehicleStruct> keptVehicles = new array<ref TBD_MissionVehicleStruct>();
					foreach (int vehIdx, TBD_MissionVehicleStruct veh : ctx.m_Document.vehicles)
					{
						string vehGate = "";
						TBD_VariantRowRefStruct skelRow = ctx.m_Skeleton.vehicles[vehIdx];
						if (skelRow)
							vehGate = skelRow.variantId;

						string vehName = "";
						if (veh)
							vehName = veh.Label();

						bool keep = !veh || TBD_MissionVariants.IsRowIncluded(vehGate, ctx.m_Declared, ctx.m_Active, "vehicle", vehName);
						if (keep)
						{
							keptVehicles.Insert(veh);
							continue;
						}

						if (veh)
						{
							array<string> crewIds;
							if (vehIdx < ctx.m_CrewByVehicle.Count())
								crewIds = ctx.m_CrewByVehicle[vehIdx];
							RememberDroppedVehicle(ctx, veh, crewIds);
						}
					}
					ctx.m_Document.vehicles = keptVehicles;
					ctx.m_iVehiclesAfter = keptVehicles.Count();
				}
				else
				{
					Print(string.Format(
						"[TBD][Variants] vehicle skeleton parse disagreed with the typed parse (typed=%1 skeleton=%2) -- vehicles NOT variant-filtered this load",
						ctx.m_iVehiclesBefore, skelCount), LogLevel.ERROR);
				}
			}
		}
	}

	//! Keep the `entities[]` rows the active set includes, dropping the twin of every excluded
	//! vehicle with a WARNING.
	//! @param ctx the run
	protected static void FilterEntities(TBD_MissionVariantFilterContext ctx)
	{
		if (ctx.m_Document.entities)
		{
			ctx.m_iEntitiesBefore = ctx.m_Document.entities.Count();
			array<ref TBD_MissionEntityStruct> keptEntities = new array<ref TBD_MissionEntityStruct>();
			foreach (TBD_MissionEntityStruct ent : ctx.m_Document.entities)
			{
				string entName = "";
				string entGate = "";
				string entUid = "";
				string entFp = "";
				if (ent)
				{
					entName = ent.alias;
					entGate = ent.variantId;
					entUid = ent.uid;
					entFp = string.Format("%1|%2|%3", ent.alias, ent.x, ent.z);
				}

				bool twinOfDropped = false;
				if (!entUid.IsEmpty() && ctx.m_DroppedVehicleUids.Contains(entUid))
					twinOfDropped = true;
				if (!twinOfDropped && !entFp.IsEmpty() && ctx.m_DroppedVehicleFingerprints.Contains(entFp))
					twinOfDropped = true;

				if (twinOfDropped)
				{
					Print(string.Format(
						"[TBD][Variants] EXCLUDING entity '%1' -- twin of an excluded vehicle (dependent ref)",
						entName), LogLevel.WARNING);
					continue;
				}

				if (!ent || TBD_MissionVariants.IsRowIncluded(entGate, ctx.m_Declared, ctx.m_Active, "entity", entName))
					keptEntities.Insert(ent);
			}
			ctx.m_Document.entities = keptEntities;
			ctx.m_iEntitiesAfter = keptEntities.Count();
		}
	}

	//! Keep the `slots[]` rows whose skeleton gate the active set includes, dropping every crew
	//! seat of an excluded vehicle with a WARNING. A skeleton that disagrees with the typed row
	//! count leaves the gates unapplied with an ERROR, but the crew seats still drop.
	//! @param ctx the run
	protected static void FilterSlots(TBD_MissionVariantFilterContext ctx)
	{
		if (ctx.m_Document.slots)
		{
			ctx.m_iSlotsBefore = ctx.m_Document.slots.Count();
			ctx.m_iSlotsAfter = ctx.m_iSlotsBefore;
			if (ctx.m_iSlotsBefore > 0)
			{
				int skelCount = -1;
				if (ctx.m_Skeleton && ctx.m_Skeleton.slots)
					skelCount = ctx.m_Skeleton.slots.Count();

				if (ctx.m_Skeleton && ctx.m_Skeleton.slots && skelCount == ctx.m_iSlotsBefore)
				{
					array<ref TBD_MissionSlotStruct> keptSlots = new array<ref TBD_MissionSlotStruct>();
					foreach (int slotIdx, TBD_MissionSlotStruct slot : ctx.m_Document.slots)
					{
						string slotGate = "";
						TBD_VariantRowRefStruct skelRow = ctx.m_Skeleton.slots[slotIdx];
						if (skelRow)
							slotGate = skelRow.variantId;

						string slotName = "";
						string slotUid = "";
						string slotKey = "";
						if (slot)
						{
							slotName = slot.Key();
							slotUid = slot.uid;
							slotKey = slot.Key();
						}

						bool crewOfDropped = false;
						if (!slotUid.IsEmpty() && ctx.m_DroppedCrewSlotIds.Contains(slotUid))
							crewOfDropped = true;
						if (!crewOfDropped && !slotKey.IsEmpty() && ctx.m_DroppedCrewSlotIds.Contains(slotKey))
							crewOfDropped = true;

						if (crewOfDropped)
						{
							Print(string.Format(
								"[TBD][Variants] EXCLUDING slot '%1' -- crew of an excluded vehicle (dependent ref)",
								slotName), LogLevel.WARNING);
							continue;
						}

						if (!slot || TBD_MissionVariants.IsRowIncluded(slotGate, ctx.m_Declared, ctx.m_Active, "slot", slotName))
							keptSlots.Insert(slot);
					}
					ctx.m_Document.slots = keptSlots;
					ctx.m_iSlotsAfter = keptSlots.Count();
				}
				else
				{
					array<ref TBD_MissionSlotStruct> keptSlots = new array<ref TBD_MissionSlotStruct>();
					int droppedCrewKept = 0;
					foreach (TBD_MissionSlotStruct slot : ctx.m_Document.slots)
					{
						string slotName = "";
						string slotUid = "";
						string slotKey = "";
						if (slot)
						{
							slotName = slot.Key();
							slotUid = slot.uid;
							slotKey = slot.Key();
						}

						bool crewOfDropped = false;
						if (!slotUid.IsEmpty() && ctx.m_DroppedCrewSlotIds.Contains(slotUid))
							crewOfDropped = true;
						if (!crewOfDropped && !slotKey.IsEmpty() && ctx.m_DroppedCrewSlotIds.Contains(slotKey))
							crewOfDropped = true;

						if (crewOfDropped)
						{
							droppedCrewKept = droppedCrewKept + 1;
							Print(string.Format(
								"[TBD][Variants] EXCLUDING slot '%1' -- crew of an excluded vehicle (dependent ref)",
								slotName), LogLevel.WARNING);
							continue;
						}

						keptSlots.Insert(slot);
					}
					ctx.m_Document.slots = keptSlots;
					ctx.m_iSlotsAfter = keptSlots.Count();
					Print(string.Format(
						"[TBD][Variants] slot skeleton parse disagreed with the typed parse (typed=%1 skeleton=%2) -- slots NOT variant-gated this load; still dropped %3 crew-of-excluded-vehicle seat(s)",
						ctx.m_iSlotsBefore, skelCount, droppedCrewKept), LogLevel.ERROR);
				}
			}
		}
	}

	//! Log the active set and the kept/total row count of every collection.
	//! @param ctx the finished run
	protected static void LogSummary(TBD_MissionVariantFilterContext ctx)
	{
		Print(string.Format(
			"[TBD][Variants] filter applied -- active=[%1] zones=%2/%3 entities=%4/%5 slots=%6/%7 orbatGroups=%8/%9",
			FormatVariantSet(ctx.m_Active), ctx.m_iZonesAfter, ctx.m_iZonesBefore, ctx.m_iEntitiesAfter, ctx.m_iEntitiesBefore,
			ctx.m_iSlotsAfter, ctx.m_iSlotsBefore, ctx.m_iGroupsAfter, ctx.m_iGroupsBefore));
		Print(string.Format("[TBD][Variants] vehicles=%1/%2", ctx.m_iVehiclesAfter, ctx.m_iVehiclesBefore));
	}

	//! Record an excluded vehicle's uid, fingerprint and crew slot ids so its dependants drop.
	//! @param ctx the run
	//! @param veh the excluded vehicle; null does nothing
	//! @param crewIds its crew slot ids from the raw pass; may be null
	protected static void RememberDroppedVehicle(TBD_MissionVariantFilterContext ctx, TBD_MissionVehicleStruct veh, array<string> crewIds)
	{
		if (!veh)
			return;

		if (!veh.uid.IsEmpty())
			ctx.m_DroppedVehicleUids.Set(veh.uid, true);

		string fp = veh.Fingerprint();
		if (!fp.IsEmpty())
			ctx.m_DroppedVehicleFingerprints.Set(fp, true);

		if (!crewIds || crewIds.Count() == 0)
			return;

		foreach (string slotId : crewIds)
		{
			if (slotId.IsEmpty())
				continue;
			ctx.m_DroppedCrewSlotIds.Set(slotId, true);
		}
	}

	//! Build the active set: the server override when it authors `activeVariants`, else the
	//! document defaults. Fills `ctx.m_Declared` with every well-formed `variants[].id`, warning on
	//! a row with no id and on a duplicate id.
	//! @param ctx the run
	//! @param defaultFlags per-row `default` flags, index-aligned with the typed `variants[]`
	//! @return the active set; override ids that name no declared variant are dropped with a WARNING
	protected static ref map<string, bool> ComputeActiveVariantSet(TBD_MissionVariantFilterContext ctx, array<bool> defaultFlags)
	{
		map<string, bool> active = new map<string, bool>();
		array<ref TBD_MissionVariantStruct> variants = ctx.m_Document.variants;
		if (!variants)
			variants = new array<ref TBD_MissionVariantStruct>();

		foreach (int i, TBD_MissionVariantStruct variant : variants)
		{
			if (!variant || variant.id.IsEmpty())
			{
				Print(string.Format("[TBD][Variants] variants[%1] has no id -- row ignored", i), LogLevel.WARNING);
				continue;
			}
			if (ctx.m_Declared.Contains(variant.id))
				Print(string.Format("[TBD][Variants] variants[%1] duplicates id '%2' -- ids must be unique", i, variant.id), LogLevel.WARNING);
			ctx.m_Declared.Set(variant.id, true);
		}

		bool overrideKeyPresent;
		TBD_VariantConfigWire cfg = TBD_MissionVariantSources.ReadVariantOverride(overrideKeyPresent);
		if (cfg && overrideKeyPresent)
		{
			if (cfg.activeVariants)
			{
				foreach (string overrideId : cfg.activeVariants)
				{
					if (ctx.m_Declared.Contains(overrideId))
						active.Set(overrideId, true);
					else
						Print(string.Format(
							"[TBD][Variants] override names unknown variant '%1' -- entry ignored (not declared in variants[])",
							overrideId), LogLevel.WARNING);
				}
			}
			Print(string.Format("[TBD][Variants] active set = SERVER OVERRIDE (%1): [%2]",
				TBD_MissionVariantSources.OVERRIDE_CONFIG_PATH, FormatVariantSet(active)));
			return active;
		}
		if (cfg)
			Print("[TBD][Variants] " + TBD_MissionVariantSources.OVERRIDE_CONFIG_PATH + " exists but has no activeVariants[] -- using document defaults", LogLevel.WARNING);

		if (defaultFlags.Count() != variants.Count())
			Print(string.Format(
				"[TBD][Variants] default-flag scan found %1 row(s) but the typed parse found %2 -- unmatched rows treated as default=false",
				defaultFlags.Count(), variants.Count()), LogLevel.WARNING);

		foreach (int j, TBD_MissionVariantStruct defVariant : variants)
		{
			if (!defVariant || defVariant.id.IsEmpty())
				continue;
			if (j < defaultFlags.Count() && defaultFlags[j])
				active.Set(defVariant.id, true);
		}

		Print(string.Format("[TBD][Variants] active set = DOCUMENT DEFAULTS: [%1]", FormatVariantSet(active)));
		return active;
	}

	//! A variant id set as "a, b, c" for the log.
	//! @param variantSet the set
	//! @return the joined ids, or "(none)" for the empty set
	protected static string FormatVariantSet(map<string, bool> variantSet)
	{
		string joined = "";
		foreach (string setId, bool setUnused : variantSet)
		{
			if (!joined.IsEmpty())
				joined = joined + ", ";
			joined = joined + setId;
		}

		if (joined.IsEmpty())
			return "(none)";

		return joined;
	}
}
