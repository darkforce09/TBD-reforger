//! Server -> player chat: one player's feed, or every connected player's. Chat is the one channel
//! that reaches a player on a dedicated server without a menu preset, so TBD replies, refusals and
//! announcements all go through here.
//! @authority server
class TBD_PlayerChat
{
	//------------------------------------------------------------------------------------------------
	//! Send `text` to one player's chat feed. False when the player has no controller or chat.
	static bool Tell(int playerId, string text)
	{
		PlayerManager players = GetGame().GetPlayerManager();
		if (!players)
			return false;

		PlayerController controller = players.GetPlayerController(playerId);
		if (!controller)
			return false;

		SCR_ChatComponent chat = SCR_ChatComponent.Cast(controller.FindComponent(SCR_ChatComponent));
		if (!chat)
			return false;

		chat.SendPrivateMessage(text, playerId);
		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! Send `text` to every connected player's chat feed. Returns how many players it reached.
	static int TellEveryone(string text)
	{
		PlayerManager players = GetGame().GetPlayerManager();
		if (!players)
			return 0;

		array<int> ids = {};
		players.GetPlayers(ids);

		int reached = 0;
		foreach (int playerId : ids)
		{
			if (Tell(playerId, text))
				reached++;
		}

		return reached;
	}
}
