/**
 * @file TBD_PlanInfo.c
 * @brief One tactical plan the Markers panel can offer.
 *
 * Role: id and label of one plan.  Position: TBD_BriefingMock fills it into TBD_BriefingCatalog; the Briefing pages read it.
 * State: plain data.  Invariants: the id is what Load Plan logs; no plan store exists.
 */

//! One tactical plan entry.
class TBD_PlanInfo
{
	string m_sId; //!< plan id
	string m_sLabel; //!< dropdown label

	//! One plan.
	//! @param id the plan id
	//! @param label the dropdown label
	void TBD_PlanInfo(string id, string label)
	{
		m_sId = id;
		m_sLabel = label;
	}
}
