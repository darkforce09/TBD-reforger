//! The effects of the `broadcast` and `kick` fleet commands, run once the platform has admitted
//! `executing` (TBD_FleetCommandExecution):
//!   * `broadcast` sends the message to every connected player's chat and succeeds with outcome
//!     {delivered_to}, the number of players it reached;
//!   * `kick` tells the player the reason in chat, then removes them after KICK_DELAY_MS so the
//!     message can arrive first (the engine's kick carries no text), and succeeds with outcome
//!     {kicked_player_id, arma_id}. A player who left in the meantime fails the kick.
//! @authority server
class TBD_FleetPlayerActions
{
	protected static const int KICK_DELAY_MS = 2000;
	protected static const string TAG = "TBD: ";

	//------------------------------------------------------------------------------------------------
	static void Broadcast(notnull TBD_FleetCommand command)
	{
		string message = command.Argument("message").Trim();
		int reached = TBD_PlayerChat.TellEveryone(message);
		TBD_Log.Kv(TBD_FleetCommandPoller.CH_FLEET, "broadcast", string.Format("command=%1 deliveredTo=%2 message='%3'", command.m_sCommandId, reached, message));
		TBD_FleetCommandExecution.Succeed(command, string.Format("{\"delivered_to\":%1}", reached), false);
	}

	//------------------------------------------------------------------------------------------------
	static void Kick(notnull TBD_FleetCommand command)
	{
		string text = TAG + "an administrator is removing you from this server";
		string reason = command.Argument("reason").Trim();
		if (!reason.IsEmpty())
			text += " - " + reason;

		TBD_PlayerChat.Tell(command.m_iTargetPlayerId, text + ".");

		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
			queue.CallLater(KickNow, KICK_DELAY_MS, false, command.m_sCommandId);
		else
			KickNow(command.m_sCommandId);
	}

	//------------------------------------------------------------------------------------------------
	//! The kick itself, against whoever holds the identity now: a reconnect changes the player id.
	protected static void KickNow(string commandId)
	{
		TBD_FleetCommand command = TBD_FleetCommandExecution.GetCurrent();
		if (!command || command.m_sCommandId != commandId || command.m_eStage != TBD_EFleetCommandStage.EFFECT)
			return;

		string armaId = command.Argument("arma_id").Trim();
		int playerId = TBD_FleetCommandArguments.FindConnectedPlayer(armaId);
		PlayerManager players = GetGame().GetPlayerManager();
		if (playerId < 0 || !players)
		{
			TBD_FleetCommandExecution.Fail(command, "the player disconnected before the kick");
			return;
		}

		string name = players.GetPlayerName(playerId);
		players.KickPlayer(playerId, PlayerManagerKickReason.KICK, 0);
		TBD_Log.Kv(TBD_FleetCommandPoller.CH_FLEET, "kicked", string.Format("command=%1 player=%2 name='%3' armaId=%4 reason='%5'",
			commandId, playerId, name, armaId, command.Argument("reason")));
		TBD_AdminAudit.Record(string.Format("FLEET: platform kick of %1(%2) - %3", name, playerId, command.Argument("reason")), false);

		TBD_FleetCommandExecution.Succeed(command, string.Format("{\"kicked_player_id\":%1,\"arma_id\":\"%2\"}", playerId, TBD_GameRuntimeHttp.JsonEscape(armaId)), false);
	}
}
