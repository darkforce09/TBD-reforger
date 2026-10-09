/**
 * @file TBD_FleetPlayerActions.c
 * @brief The effects of the `broadcast` and `kick` fleet commands.
 *
 * Role: `broadcast` sends the message to every connected player's chat and succeeds with outcome
 * {delivered_to}; `kick` tells the player the reason in chat, removes them after `KICK_DELAY_MS`,
 * and succeeds with outcome {kicked_player_id, arma_id}.  Position: started by
 * `TBD_FleetCommandExecution` once the platform has admitted `executing`; reports back through
 * `Succeed`/`Fail`; sends through `TBD_PlayerChat`.
 * State: none; the command carries its own.  Invariants: the kick waits so the chat line arrives
 * first (the engine's kick carries no text); it targets whoever holds the identity at kick time,
 * and a player who left in the meantime fails the kick.
 */

//! Chat and kick effects of fleet commands.
//! @authority server
class TBD_FleetPlayerActions
{
	protected static const int KICK_DELAY_MS = 2000; //!< delay between the chat line and the kick, in milliseconds
	protected static const string TAG = "TBD: "; //!< prefix of the kicked player's chat line

	//! Send the `message` argument to every connected player and succeed with {delivered_to}.
	//! @authority server
	static void Broadcast(notnull TBD_FleetCommand command)
	{
		string message = command.Argument("message").Trim();
		int reached = TBD_PlayerChat.Broadcast(string.Empty, message);
		TBD_Log.Kv(TBD_FleetCommandPoller.CH_FLEET, "broadcast", string.Format("command=%1 deliveredTo=%2 message='%3'", command.m_sCommandId, reached, message));
		TBD_FleetCommandExecution.Succeed(command, string.Format("{\"delivered_to\":%1}", reached), false);
	}

	//! Tell the target the reason, then kick after `KICK_DELAY_MS` (at once without a call queue).
	//! @authority server
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

	//! The kick itself, against whoever holds the identity now: a reconnect changes the player id.
	//! Does nothing when `commandId` is not the command in its effect stage.
	//! @authority server
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

		TBD_FleetCommandExecution.Succeed(command, string.Format("{\"kicked_player_id\":%1,\"arma_id\":\"%2\"}", playerId, TBD_BackendText.JsonEscape(armaId)), false);
	}
}
