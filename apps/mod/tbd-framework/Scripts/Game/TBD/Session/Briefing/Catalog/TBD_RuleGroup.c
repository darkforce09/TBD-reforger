/**
 * @file TBD_RuleGroup.c
 * @brief A collapsible group of numbered rules on the Rules page.
 *
 * Role: a titled group of rules and whether it starts open.  Position: TBD_BriefingMock fills it into TBD_BriefingCatalog; the Briefing pages read it.
 * State: plain data.  Invariants: rules keep their insertion order, which is their number.
 */

//! One numbered rule: a title and its body text.
class TBD_RuleInfo
{
	string m_sTitle; //!< rule title
	string m_sBody; //!< rule body text

	//! One rule.
	//! @param title the rule title
	//! @param body the rule body
	void TBD_RuleInfo(string title, string body)
	{
		m_sTitle = title;
		m_sBody = body;
	}
}

//! "Mission Rules" / "General Rules" -- a collapsible group of numbered rules.
class TBD_RuleGroup
{
	string m_sTitle; //!< group title
	bool m_bOpen; //!< the group starts expanded; default true
	ref array<ref TBD_RuleInfo> m_aRules; //!< rules in number order

	//! An empty group.
	//! @param title the group title
	//! @param open the group starts expanded
	void TBD_RuleGroup(string title, bool open = true)
	{
		m_sTitle = title;
		m_bOpen = open;
		m_aRules = {};
	}

	//! Append one rule.
	//! @param title the rule title
	//! @param body the rule body
	void Add(string title, string body)
	{
		m_aRules.Insert(new TBD_RuleInfo(title, body));
	}
}
