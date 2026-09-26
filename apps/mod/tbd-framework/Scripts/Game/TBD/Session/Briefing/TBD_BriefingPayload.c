/**
 * @file TBD_BriefingPayload.c
 * @brief The briefing one player may read, and the records it is made of.
 *
 * Role: the side-scoped briefing model: mission identity, own seat and kit, ORBAT groups and roles,
 * visible zones, written orders and the round-end condition.  Position: TBD_BriefingService fills
 * it on the server; TBD_BriefingWire moves it; TBD_BriefingClient holds it on the client for the
 * screen.
 * State: plain data, owned by whoever holds the reference.  Invariants: a client holds no mission
 * document, so the payload is all it can know; it never contains another faction's groups, zones
 * or orders, because TBD_BriefingService filters them out on the server before anything is
 * written; the arrays are allocated in the constructor and never null, so presence is tested by
 * count; an unavailable payload says why in `m_sUnavailableReason`.
 */

//! One role line inside a group, such as `RFL x4`, flagged when the reader's own seat is on it.
class TBD_BriefingRole
{
	string m_sRole; //!< role label, such as `RFL`
	int m_iCount; //!< seats of this role in the group
	bool m_bIsOwn; //!< the reader's own seat has this role

	//! One role line.
	//! @param role the role label
	//! @param count the seat count
	//! @param isOwn the reader's seat has this role
	void TBD_BriefingRole(string role, int count, bool isOwn)
	{
		m_sRole = role;
		m_iCount = count;
		m_bIsOwn = isOwn;
	}
}

//! One ORBAT group (squad) of the reader's faction.
class TBD_BriefingGroup
{
	string m_sCallsign; //!< group callsign; may be empty
	int m_iSeats; //!< seats in the group
	bool m_bIsOwn; //!< the reader's own squad, shown expanded
	ref array<ref TBD_BriefingRole> m_aRoles; //!< role lines in first-seen order

	//! An empty group.
	//! @param callsign the group callsign
	void TBD_BriefingGroup(string callsign)
	{
		m_sCallsign = callsign;
		m_aRoles = {};
	}

	//! Fold one more seat of `role` into this group, creating the role line on first sight.
	//! @param role the seat's role label
	//! @param isOwn the seat is the reader's
	void AddSeat(string role, bool isOwn)
	{
		m_iSeats++;

		foreach (TBD_BriefingRole existing : m_aRoles)
		{
			if (existing.m_sRole != role)
				continue;

			existing.m_iCount++;
			if (isOwn)
				existing.m_bIsOwn = true;

			return;
		}

		m_aRoles.Insert(new TBD_BriefingRole(role, 1, isOwn));
	}
}

//! One area the reader is allowed to see.
class TBD_BriefingZone
{
	string m_sTitle; //!< authored label, or humanised type and id
	string m_sDetail; //!< `x, z - r<radius>`, `area - <n> pts` or `area`
	bool m_bIsOwn; //!< belongs to the reader's faction; false for a shared zone

	//! One zone entry.
	//! @param title the zone title
	//! @param detail the shape summary
	//! @param isOwn the zone is the reader's faction's
	void TBD_BriefingZone(string title, string detail, bool isOwn)
	{
		m_sTitle = title;
		m_sDetail = detail;
		m_bIsOwn = isOwn;
	}
}

//! One `label: value` line of the reader's own loadout.
class TBD_BriefingKitLine
{
	string m_sLabel; //!< row label, such as `Primary`
	string m_sValue; //!< short item name, or the cargo summary

	//! One kit line.
	//! @param label the row label
	//! @param value the row value
	void TBD_BriefingKitLine(string label, string value)
	{
		m_sLabel = label;
		m_sValue = value;
	}
}

//! Everything one player may read before the round goes live. Built on the server, shipped as
//! one string plus three orders arrays, rebuilt on the client.
class TBD_BriefingPayload
{
	string m_sMissionName; //!< mission `meta.name`, sanitised
	string m_sTerrain; //!< mission `meta.terrain`, sanitised
	string m_sFactionKey; //!< the reader's faction key, sanitised for display
	string m_sFactionName; //!< the reader's faction display name, or its key

	bool m_bHasSlot; //!< the reader holds a slot
	string m_sOwnGroup; //!< the reader's group callsign
	string m_sOwnRole; //!< the reader's role
	string m_sOwnKit; //!< the reader's kit alias
	ref array<ref TBD_BriefingKitLine> m_aKit; //!< the reader's non-empty gear, then a cargo line

	ref array<ref TBD_BriefingGroup> m_aGroups; //!< the reader's faction's groups in slot order
	ref array<ref TBD_BriefingZone> m_aZones; //!< the reader's faction's zones and the shared ones

	//! The three orders fields, one entry per authored paragraph in document order. Empty means the
	//! side authored none, whether the key was absent or blank, and the screen then shows no
	//! heading at all. Paragraphs travel as RPC arrays, never through the delimited wire.
	ref array<string> m_aSituation; //!< situation paragraphs
	ref array<string> m_aMission; //!< mission paragraphs
	ref array<string> m_aExecution; //!< execution paragraphs

	string m_sWinMode; //!< humanised win-condition mode; both sides share it
	ref array<string> m_aEndConditions; //!< humanised `endOn` triggers

	string m_sUnavailableReason; //!< why the server could not answer (no mission, no slot); empty when available

	//! An empty payload with every array allocated.
	void TBD_BriefingPayload()
	{
		m_aKit = {};
		m_aGroups = {};
		m_aZones = {};
		m_aEndConditions = {};
		m_aSituation = {};
		m_aMission = {};
		m_aExecution = {};
	}

	//! @return true when the server answered, false when `m_sUnavailableReason` says why not
	bool IsAvailable()
	{
		return m_sUnavailableReason.IsEmpty();
	}

	//! @return true when this side authored at least one paragraph of orders; a count test, since
	//! the arrays are never null
	bool HasOrders()
	{
		return OrderParagraphCount() > 0;
	}

	//! @return the paragraphs across situation, mission and execution
	int OrderParagraphCount()
	{
		int n = m_aSituation.Count();
		n += m_aMission.Count();
		n += m_aExecution.Count();
		return n;
	}
}
