/**
 * @file TBD_PlayerChat.c
 * @brief Server-to-player chat: one player's feed, or every connected player's.
 *
 * Role: the channel for TBD replies, refusals and announcements.  Chat reaches a player on a
 * dedicated server without a menu preset, so every player-facing server message goes through it.
 * Position: called by the framework manager, safestart, admin commands, fleet actions and the
 * mission selector on the server; sends through each controller's `SCR_ChatComponent`.
 * State: none.  Invariants: a player without a controller or chat component is skipped, never an
 * error; `Broadcast` sends nothing from a client.
 */

//! Server-to-player chat delivery.
//! @authority server
class TBD_PlayerChat
{
	//! Send `text` to one player's chat feed.
	//! @return false when the player has no controller or chat component
	//! @authority server
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

	//! Send `text` to every connected player's chat feed.
	//! @return how many players it reached
	//! @authority server
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

	//! Send `text` to every connected player from the authority, first logging
	//! `[TBD][<tag>] broadcast: <text>` when `tag` is not empty.
	//! @param tag the log channel of the broadcast line; empty writes no line
	//! @param text the chat line
	//! @return how many players it reached; 0 on a client, which sends nothing
	//! @authority server
	static int Broadcast(string tag, string text)
	{
		if (TBD_Authority.IsClient())
			return 0;

		if (!tag.IsEmpty())
			TBD_Log.Event(tag, "broadcast: " + text);

		return TellEveryone(text);
	}
}
