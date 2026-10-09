/**
 * @file TBD_NetInfo.c
 * @brief One radio net row of the Frequencies page.
 *
 * Role: callsign, label, frequency and auxiliary channels of one net, and their display text.  Position: TBD_BriefingMock fills it into TBD_BriefingCatalog; the Briefing pages read it.
 * State: plain data.  Invariants: frequencies are MHz shown to one decimal; a net with no
 * callsign is titled by its label alone.
 */

//! One radio net row: `A1-2  1st Platoon, 2nd Squad   163.7 MHz` + aux channels.
class TBD_NetInfo
{
	string m_sCallsign;   //!< "A1-1"; empty on the LR net
	string m_sLabel;      //!< "Company HQ (Mission Maker)"
	float m_fFreqMHz; //!< primary frequency, MHz
	ref array<float> m_aAux; //!< auxiliary channels, MHz
	bool m_bLongRange; //!< the long-range command net; default false
	bool m_bOwn;          //!< the reader's own squad net (highlighted)

	//! One net.
	//! @param callsign the callsign; empty on the long-range net
	//! @param label the net label
	//! @param freqMHz the primary frequency, MHz
	//! @param longRange the long-range command net
	//! @param own the reader's own squad net
	void TBD_NetInfo(string callsign, string label, float freqMHz, bool longRange = false, bool own = false)
	{
		m_sCallsign = callsign;
		m_sLabel = label;
		m_fFreqMHz = freqMHz;
		m_bLongRange = longRange;
		m_bOwn = own;
		m_aAux = {};
	}

	//! Append three auxiliary channels.
	//! @return this net, for chaining
	TBD_NetInfo Aux(float a, float b, float c)
	{
		m_aAux.Insert(a);
		m_aAux.Insert(b);
		m_aAux.Insert(c);
		return this;
	}

	//! @return the callsign and label, or the label alone when there is no callsign
	string Title()
	{
		if (m_sCallsign.IsEmpty())
			return m_sLabel;

		return m_sCallsign + " " + m_sLabel;
	}

	//! @return the primary frequency as `<f> MHz`
	string FreqText()
	{
		return string.Format("%1 MHz", m_fFreqMHz.ToString(-1, 1));
	}

	//! @return the auxiliary channels comma-separated with ` MHz`, or empty when there are none
	string AuxText()
	{
		if (m_aAux.IsEmpty())
			return string.Empty;

		string text;
		foreach (int i, float aux : m_aAux)
		{
			if (i > 0)
				text += ", ";
			text += aux.ToString(-1, 1);
		}

		return text + " MHz";
	}
}
