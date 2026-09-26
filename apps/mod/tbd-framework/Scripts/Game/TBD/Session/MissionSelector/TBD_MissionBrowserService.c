/**
 * @file TBD_MissionBrowserService.c
 * @brief The mission browser's list payload: built on the server, split back into lines on the client.
 *
 * Role: turns the deployable mission list into one newline-delimited string and back.  Position: TBD_RpcAsk_MissionList on SCR_PlayerController calls BuildListPayload on the server;
 * TBD_RpcDo_ReceiveMissionList calls ParseListPayload on the owning client; the lines come from
 * TBD_DeployableMissionList.
 * State: none; pure functions.  Invariants: at most MAX_LIST_LINES entries are written, plus one
 * line naming how many more there are; selection numbers stay 1-based over the full list.
 */

//! Serialises the deployable mission list to a newline-delimited payload and parses it back, so
//! the RPC carries a single string.
class TBD_MissionBrowserService
{
	protected static const int MAX_LIST_LINES = 100; //!< entries per payload; only the display is clipped, selection numbers cover the full list

	//! The `n) Title [terrain]` lines of the deployable mission list, or why it is empty.
	//! @return the payload
	//! @authority server
	static string BuildListPayload()
	{
		int count = TBD_DeployableMissionList.Count();
		if (count == 0)
			return TBD_DeployableMissionList.DescribeEmpty();

		int shown = count;
		if (shown > MAX_LIST_LINES)
			shown = MAX_LIST_LINES;

		string result;
		for (int i = 0; i < shown; i++)
		{
			if (i > 0)
				result = result + "\n";
			result = result + TBD_DeployableMissionList.DescribeEntry(i + 1);
		}
		if (count > shown)
			result = result + string.Format("\n... and %1 more (list capped at %2).", count - shown, MAX_LIST_LINES);
		return result;
	}

	//! Split a payload back into display lines.
	//! @param payload the string BuildListPayload produced
	//! @return the lines, never null
	//! @authority owner
	static array<string> ParseListPayload(string payload)
	{
		array<string> lines = new array<string>();
		payload.Split("\n", lines, false);
		return lines;
	}
}
