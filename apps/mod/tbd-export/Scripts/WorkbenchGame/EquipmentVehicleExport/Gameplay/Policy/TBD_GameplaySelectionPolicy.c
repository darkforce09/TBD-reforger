// Explicit class/property decisions are generated from the versioned catalog policy.
class TBD_GameplaySelectionRule
{
	string m_sDisposition;
	string m_sSection;
	string m_sReason;
	bool m_bFollowReference;
}

class TBD_GameplaySelectionPolicy
{
	ref map<string, string> m_mClasses = new map<string, string>();
	ref map<string, ref TBD_GameplaySelectionRule> m_mRules = new map<string, ref TBD_GameplaySelectionRule>();
	ref map<string, int> m_mObserved = new map<string, int>();
	ref array<string> m_aUnknown = {};
	ref array<string> m_aErrors = {};

	void TBD_GameplaySelectionPolicy()
	{
		TBD_GameplayPolicyGenerated.Apply(this);
	}

	void AddClass(string className, string section)
	{
		m_mClasses.Insert(className, section);
	}

	void AddRule(string className, string fields, string disposition, string section, bool followReference, string reason)
	{
		TBD_GameplaySelectionRule rule = new TBD_GameplaySelectionRule();
		rule.m_sDisposition = disposition;
		rule.m_sSection = section;
		rule.m_bFollowReference = followReference;
		rule.m_sReason = reason;
		array<string> entries = {};
		fields.Split("|", entries, true);
		foreach (string entry : entries)
			m_mRules.Insert(className + "\t" + entry, rule);
	}

	bool Selected(string className)
	{
		if (!m_mClasses.Contains(className))
		{
			Unknown("class:" + className);
			return false;
		}
		return m_mClasses.Get(className) != "excluded";
	}

	void ObserveExcluded(BaseContainer container)
	{
		for (int index = 0; index < container.GetNumVars(); index++)
		{
			string property = container.GetVarName(index);
			string nativeType = typename.EnumToString(DataVarType, container.GetDataVarType(index));
			Rule(container.GetClassName(), property, nativeType, true);
		}
	}

	TBD_GameplaySelectionRule Rule(string className, string property, string nativeType, bool observe = false)
	{
		string key = className + "\t" + property + "\t" + nativeType;
		TBD_GameplaySelectionRule rule = m_mRules.Get(key);
		if (!rule) Unknown(key);
		else if (observe) m_mObserved.Set(key, m_mObserved.Get(key) + 1);
		return rule;
	}

	protected void Unknown(string key)
	{
		if (m_aUnknown.Find(key) >= 0) return;
		m_aUnknown.Insert(key);
		m_aErrors.Insert("Unreviewed gameplay selection: " + key);
	}

	static bool GameplayReference(string name)
	{
		return name.EndsWith(".et") || name.EndsWith(".conf") || name.EndsWith(".gamemat");
	}

	string Report(int resources, int nodes, int facts)
	{
		array<string> keys = {};
		foreach (string key, TBD_GameplaySelectionRule rule : m_mRules) keys.Insert(key);
		keys.Sort();
		array<string> decisions = {};
		foreach (string fieldKey : keys)
		{
			array<string> parts = {};
			fieldKey.Split("\t", parts, true);
			TBD_GameplaySelectionRule decision = m_mRules.Get(fieldKey);
			string row = "{\"class_name\":" + TBD_SourceExportJson.Quote(parts[0]);
			row += ",\"property\":" + TBD_SourceExportJson.Quote(parts[1]);
			row += ",\"native_type\":" + TBD_SourceExportJson.Quote(parts[2]);
			row += ",\"disposition\":" + TBD_SourceExportJson.Quote(decision.m_sDisposition);
			row += ",\"section\":" + TBD_SourceExportJson.Quote(decision.m_sSection);
			row += ",\"reason\":" + TBD_SourceExportJson.Quote(decision.m_sReason);
			row += ",\"observed_count\":" + m_mObserved.Get(fieldKey).ToString() + "}";
			decisions.Insert(row);
		}
		string json = "{\"schema_version\":1,\"policy_version\":1,\"policy_sha256\":" + TBD_SourceExportJson.Quote(TBD_GameplayPolicyGenerated.DIGEST);
		json += ",\"baseline_field_combinations\":" + m_mRules.Count().ToString();
		json += ",\"resource_count\":" + resources.ToString() + ",\"node_count\":" + nodes.ToString();
		json += ",\"fact_count\":" + facts.ToString() + ",\"unreviewed\":" + TBD_SourceExportJson.Strings(m_aUnknown);
		json += ",\"decisions\":[" + TBD_SourceExportJson.Join(decisions) + "]}";
		return json;
	}
}
