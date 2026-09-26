/**
 * @file TBD_BriefingService.c
 * @brief Builds the briefing one player is entitled to read, from server-owned mission and slot state.
 *
 * Role: assembles a TBD_BriefingPayload for one player: mission identity, own seat and kit, the own
 * side's written orders, ORBAT and zones, and the shared win condition.  Position: called by the
 * briefing RPC handlers on SCR_PlayerController (the server half, or in place on a listen host);
 * reads TBD_MissionLoader and TBD_SpawnManager; the payload goes to TBD_BriefingWire, or straight
 * to TBD_BriefingClient on a listen host.
 * State: none of its own; truncation warnings go through TBD_WarnOnce on the Briefing channel.
 * Invariants: the side comes from TBD_SpawnManager.GetAssignedSlot, never from the client; a player
 * with no slot gets an unavailable payload with no ORBAT, orders or zones (fail closed); another
 * faction's slots, zones and orders never enter the payload; every authored display string passes
 * TBD_WireCodec.Sanitise; one side's orders stay within MAX_ORDER_CHARS bytes and
 * MAX_ORDER_PARAGRAPHS paragraphs per field, and each cut warns once per faction and field.
 */

//! Server-side briefing builder, filtered to the requesting player's side.
class TBD_BriefingService
{
	static const string CH_BRIEFING = "Briefing"; //!< log channel of the briefing feature

	//! Byte budget shared by one side's situation, mission and execution. `string.Length()` counts
	//! bytes, so accented prose spends it slightly faster than its glyph count.
	protected static const int MAX_ORDER_CHARS = 6000; //!< bytes; default 6000
	protected static const int MAX_ORDER_PARAGRAPHS = 16; //!< paragraphs kept per orders field
	protected static const int MIN_ORDER_TAIL = 24; //!< bytes; below this remainder a field is dropped, not stubbed
	protected static const int MAX_WARN_STATES = 64; //!< warning keys kept on the Briefing channel before its set clears

	//! Build the briefing `playerId` is entitled to read.
	//! @param playerId the requesting player; their side comes from their assigned slot
	//! @return a payload; it carries `m_sUnavailableReason` and nothing side-scoped when the mission
	//! is not loaded and valid or the player holds no slot
	//! @authority server -- reads TBD_SpawnManager and TBD_MissionLoader, which hold no data on a client
	static TBD_BriefingPayload BuildForPlayer(int playerId)
	{
		TBD_BriefingPayload payload = new TBD_BriefingPayload();

		TBD_MissionDocumentStruct doc = TBD_MissionLoader.GetMission();
		if (!doc || !TBD_MissionLoader.IsValid())
		{
			payload.m_sUnavailableReason = "Mission is still loading.";
			return payload;
		}

		if (doc.meta)
		{
			payload.m_sMissionName = TBD_WireCodec.Sanitise(doc.meta.name);
			payload.m_sTerrain = TBD_WireCodec.Sanitise(doc.meta.terrain);
		}

		// Answered from server-owned state only. The client never supplies it.
		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		TBD_MissionSlotStruct own;
		if (spawn)
			own = spawn.GetAssignedSlot(playerId);

		if (!own)
		{
			// Fail closed: no slot means no side, and no side means no ORBAT.
			payload.m_sUnavailableReason = "No slot assigned yet. Claim a slot in the lobby first.";
			return payload;
		}

		// A mission staged by hand in the profile cache meets no schema validation, so the faction
		// key is sanitised like every other authored display string. The comparisons in
		// `BuildOrders`, `BuildOrbat` and `BuildZones` keep the raw key: a faction must match itself
		// exactly for side filtering to hold.
		payload.m_sFactionKey = TBD_WireCodec.Sanitise(own.faction);
		payload.m_sFactionName = TBD_WireCodec.Sanitise(TBD_MissionFactionNames.DisplayName(doc, own.faction));
		payload.m_bHasSlot = true;
		payload.m_sOwnGroup = TBD_WireCodec.Sanitise(own.groupCallsign);
		payload.m_sOwnRole = TBD_WireCodec.Sanitise(own.role);
		payload.m_sOwnKit = TBD_WireCodec.Sanitise(own.kit);

		BuildKit(payload, own);
		BuildOrders(payload, own.faction);
		BuildOrbat(payload, doc, own);
		BuildZones(payload, doc, own.faction);
		BuildEndConditions(payload, doc);

		return payload;
	}

	//! Fill the payload's orders with `factionKey`'s written orders and no other side's. A missing
	//! `briefings` block, a missing entry and blank fields all produce no paragraphs, so the screen
	//! shows no empty heading. The strings are tested for content, never for null.
	//! @param payload the payload being built
	//! @param factionKey the raw faction key of the reader's slot
	//! @authority server
	protected static void BuildOrders(TBD_BriefingPayload payload, string factionKey)
	{
		TBD_MissionBriefingStruct briefing = TBD_MissionLoader.GetBriefingForFaction(factionKey);
		if (!briefing)
			return; // this mission authored no orders for this side, which is legal

		// One shared budget across the three fields so a single pathological paragraph cannot
		// crowd out the other two sections, and the whole block stays bounded on a reliable channel.
		int budget = MAX_ORDER_CHARS;
		budget = AppendParagraphs(payload.m_aSituation, briefing.situation, budget, factionKey, "situation");
		budget = AppendParagraphs(payload.m_aMission, briefing.mission, budget, factionKey, "mission");
		AppendParagraphs(payload.m_aExecution, briefing.execution, budget, factionKey, "execution");
	}

	//! Split one authored orders field into sanitised, trimmed paragraphs and append them, spending
	//! the shared byte budget. Blank paragraphs are dropped; a paragraph over the remaining budget is
	//! clipped at a word and marked `...`; a remainder under MIN_ORDER_TAIL ends the field. Every
	//! cut warns once per faction and field.
	//! @param destination the payload array for this field
	//! @param raw the authored text, newlines separating paragraphs
	//! @param budget bytes left for the orders
	//! @param factionKey the reader's faction, for the warning key and text
	//! @param field `situation`, `mission` or `execution`, for the warning key and text
	//! @return the bytes left after this field
	protected static int AppendParagraphs(array<string> destination, string raw, int budget, string factionKey, string field)
	{
		if (raw.IsEmpty())
			return budget; // content test: an absent key and an authored blank read the same

		array<string> parts = TBD_BriefingText.SplitLines(raw);
		int kept = 0;

		foreach (string part : parts)
		{
			string paragraph = TBD_BriefingText.TrimSpaces(TBD_WireCodec.Sanitise(part));
			if (paragraph.IsEmpty())
				continue;

			if (kept >= MAX_ORDER_PARAGRAPHS)
			{
				TBD_WarnOnce.Warn(CH_BRIEFING, factionKey + "|" + field, string.Format(
					"faction '%1' authored more than %2 paragraphs of %3; the rest are not shown.",
					factionKey, MAX_ORDER_PARAGRAPHS, field), MAX_WARN_STATES);
				break;
			}

			// Not `budget <= 0`: a handful of bytes left would render a meaningless stub. Below a
			// useful remainder the rest of the field is dropped and the cut is logged.
			if (budget < MIN_ORDER_TAIL)
			{
				TBD_WarnOnce.Warn(CH_BRIEFING, factionKey + "|" + field, string.Format(
					"faction '%1' orders exceed the %2-byte budget; %3 was cut short.",
					factionKey, MAX_ORDER_CHARS, field), MAX_WARN_STATES);
				break;
			}

			if (paragraph.Length() > budget)
			{
				paragraph = TBD_BriefingText.ClipToWord(paragraph, budget) + "...";
				TBD_WarnOnce.Warn(CH_BRIEFING, factionKey + "|" + field, string.Format(
					"faction '%1' orders exceed the %2-byte budget; %3 was truncated.",
					factionKey, MAX_ORDER_CHARS, field), MAX_WARN_STATES);
			}

			destination.Insert(paragraph);
			budget -= paragraph.Length();
			kept++;
		}

		return budget;
	}

	//! Copy the win mode and every non-empty `endOn` trigger, humanised. Both sides share one win
	//! condition, so nothing is filtered; every declared trigger is shown whether or not the round
	//! evaluates it.
	//! @param payload the payload being built
	//! @param doc the loaded mission
	protected static void BuildEndConditions(TBD_BriefingPayload payload, TBD_MissionDocumentStruct doc)
	{
		if (!doc.winConditions)
			return;

		payload.m_sWinMode = TBD_BriefingText.Humanise(doc.winConditions.mode);

		if (!doc.winConditions.endOn)
			return;

		foreach (string trigger : doc.winConditions.endOn)
		{
			if (!trigger.IsEmpty())
				payload.m_aEndConditions.Insert(TBD_BriefingText.Humanise(trigger));
		}
	}

	//! List the reader's non-empty gear in the order Primary, Launcher, Handgun, Throwable, Optic,
	//! Magazine, Uniform, Vest, Helmet, Backpack, then one Cargo summary line. Pants, boots and
	//! handwear are not listed.
	//! @param payload the payload being built
	//! @param own the reader's slot; nothing is added when it has no loadout
	protected static void BuildKit(TBD_BriefingPayload payload, TBD_MissionSlotStruct own)
	{
		if (!own.loadout)
			return;

		TBD_SlotGearStruct gear = own.loadout.gear;
		if (gear)
		{
			AddKitLine(payload, "Primary", gear.primary);
			AddKitLine(payload, "Launcher", gear.launcher);
			AddKitLine(payload, "Handgun", gear.handgun);
			AddKitLine(payload, "Throwable", gear.throwable);
			AddKitLine(payload, "Optic", gear.optic);
			AddKitLine(payload, "Magazine", gear.magazine);
			AddKitLine(payload, "Uniform", gear.uniform);
			AddKitLine(payload, "Vest", gear.vest);
			AddKitLine(payload, "Helmet", gear.helmet);
			AddKitLine(payload, "Backpack", gear.backpack);
		}

		if (!own.loadout.cargo || own.loadout.cargo.IsEmpty())
			return;

		// Cargo is summarised, not enumerated: the briefing answers "am I carrying supplies",
		// and the full manifest belongs in the arsenal, not on a planning screen.
		int units = 0;
		foreach (TBD_SlotCargoStruct row : own.loadout.cargo)
		{
			if (row)
				units += row.qty;
		}

		payload.m_aKit.Insert(new TBD_BriefingKitLine("Cargo",
			string.Format("%1 item(s), %2 unit(s)", own.loadout.cargo.Count(), units)));
	}

	//! Add one kit line showing the resource's short name; an empty resource adds nothing.
	//! @param payload the payload being built
	//! @param label the row label
	//! @param resource the prefab resource name
	protected static void AddKitLine(TBD_BriefingPayload payload, string label, string resource)
	{
		if (resource.IsEmpty())
			return;

		payload.m_aKit.Insert(new TBD_BriefingKitLine(label, TBD_BriefingText.PrettyResourceName(resource)));
	}

	//! Fold the flattened slot array into groups and roles for the reader's faction only. A slot of
	//! another faction is skipped before anything about it is recorded.
	//! @param payload the payload being built
	//! @param doc the loaded mission
	//! @param own the reader's slot; its seat and group are flagged as own
	protected static void BuildOrbat(TBD_BriefingPayload payload, TBD_MissionDocumentStruct doc, TBD_MissionSlotStruct own)
	{
		array<ref TBD_MissionSlotStruct> slots = TBD_MissionLoader.GetSlots();
		if (!slots)
			return;

		string ownKey = own.Key();

		foreach (TBD_MissionSlotStruct slot : slots)
		{
			if (!slot)
				continue;

			if (slot.faction != own.faction)
				continue;

			TBD_BriefingGroup group = AcquireGroup(payload, TBD_WireCodec.Sanitise(slot.groupCallsign));
			bool isOwnSeat = slot.Key() == ownKey;

			group.AddSeat(TBD_WireCodec.Sanitise(slot.role), isOwnSeat);

			if (isOwnSeat)
				group.m_bIsOwn = true;
		}
	}

	//! The group named `callsign`, created at the end of the list on first sight.
	//! @param payload the payload being built
	//! @param callsign the sanitised group callsign
	//! @return the existing or new group; never null
	protected static TBD_BriefingGroup AcquireGroup(TBD_BriefingPayload payload, string callsign)
	{
		foreach (TBD_BriefingGroup existing : payload.m_aGroups)
		{
			if (existing.m_sCallsign == callsign)
				return existing;
		}

		payload.m_aGroups.Insert(new TBD_BriefingGroup(callsign));
		return payload.m_aGroups[payload.m_aGroups.Count() - 1];
	}

	//! Add the zones the reader may see: their faction's and every shared one. Another faction's
	//! zone, such as the enemy spawn, is skipped. The detail is `x, z - r<radius>` for a circle with
	//! a positive radius, `area - <n> pts` for a polygon, else `area`.
	//! @param payload the payload being built
	//! @param doc the loaded mission
	//! @param factionKey the raw faction key of the reader's slot
	protected static void BuildZones(TBD_BriefingPayload payload, TBD_MissionDocumentStruct doc, string factionKey)
	{
		if (!doc.zones)
			return;

		foreach (TBD_MissionZoneStruct zone : doc.zones)
		{
			if (!zone)
				continue;

			bool isOwn = zone.faction == factionKey;
			bool isShared = zone.faction.IsEmpty();

			if (!isOwn && !isShared)
				continue; // another side's zone

			// `JsonLoadContext` allocates a nested `ref` field whether or not its JSON key is
			// present, so `zone.shape.circle` is never null on a polygon zone. The circle and
			// polygon branches therefore test content (a positive radius, a vertex count), never
			// nullness; see TBD_MissionShapeStruct.
			string detail = "area";
			if (zone.shape)
			{
				if (zone.shape.circle && zone.shape.circle.r > 0)
				{
					detail = string.Format("%1, %2 - r%3",
						Math.Round(zone.shape.circle.x),
						Math.Round(zone.shape.circle.z),
						Math.Round(zone.shape.circle.r));
				}
				else if (zone.shape.polygon && zone.shape.polygon.Count() > 0)
				{
					detail = string.Format("area - %1 pts", zone.shape.polygon.Count());
				}
			}

			payload.m_aZones.Insert(new TBD_BriefingZone(PrettyZoneTitle(zone), detail, isOwn));
		}
	}

	//! The zone's authored label, else its humanised type followed by ` -- <id>` when it has an id.
	//! @param zone the zone to title
	//! @return the sanitised title
	protected static string PrettyZoneTitle(TBD_MissionZoneStruct zone)
	{
		if (!zone.label.IsEmpty())
			return TBD_WireCodec.Sanitise(zone.label);

		string label = TBD_BriefingText.Humanise(zone.type);
		if (zone.id.IsEmpty())
			return label;

		return string.Format("%1 -- %2", label, TBD_WireCodec.Sanitise(zone.id));
	}
}
