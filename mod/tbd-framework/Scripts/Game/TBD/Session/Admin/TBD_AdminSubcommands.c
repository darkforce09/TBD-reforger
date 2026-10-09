/**
 * @file TBD_AdminSubcommands.c
 * @brief The argument-taking admin chat powers: `#tbd safestart` and `#tbd identity`.
 *
 * Role: runs the safestart and identity-waiver subcommands for a listed admin.  Position: TBD_AdminCommands.Dispatch calls Safestart and Identity with the sender's player id; they drive
 * TBD_SafestartManager and TBD_SpawnManager and write TBD_AdminAudit.
 * State: none; every call reads the managers afresh.  Invariants: each entry point refuses on a
 * client and refuses a caller TBD_AdminService.IsAdmin rejects, noting the refusal, before it reads
 * or changes anything.
 */

//! Chat-only admin powers whose arguments the menu's single-button actions cannot express. Gated
//! and audited like TBD_AdminService.Execute.
class TBD_AdminSubcommands
{
	//! `#tbd safestart [status|go|<seconds>]`: reads the warm-up, ends it early, or sets its length.
	//! `status` changes nothing, so any admin can ask whether damage is off during a live event.
	//! @param callerId the sender's player id, never taken from the wire
	//! @param arg `status` (also empty), `go`, or a positive number of seconds
	//! @param ok true when the request did what it asked
	//! @return the line to show the admin; a refusal or a usage line when the request cannot run
	//! @authority server
	static string Safestart(int callerId, string arg, out bool ok)
	{
		ok = false;

		// Authority only -- the countdown and every damage mutation are server-owned; a client
		// build reaching here would half-run them locally and protect nobody.
		if (TBD_Authority.IsClient())
			return "TBD: admin actions execute on the server only.";

		if (!TBD_AdminService.IsAdmin(callerId))
		{
			TBD_AdminService.NoteDeniedAccess(callerId, "action 'safestart'");
			return "TBD: refused -- you are not a listed server admin.";
		}

		TBD_SafestartManager safestart = TBD_SafestartManager.GetInstance();
		if (!safestart)
			return "TBD: safestart manager not on this game mode -- SAFE_START cannot be enforced here.";

		string request = arg;
		if (request.IsEmpty())
			request = "status";

		if (request == "status")
		{
			ok = true;
			return safestart.StatusLine();
		}

		if (request == "go")
		{
			if (!safestart.IsArmed())
				return "TBD: safestart is not running -- nothing to end.";

			safestart.GoLive(string.Format("admin %1", TBD_AdminService.Label(callerId)));
			ok = true;
			TBD_AdminAudit.Record(string.Format("%1 ended safestart early", TBD_AdminService.Label(callerId)), false);
			return "TBD: safestart ended -- weapons live.";
		}

		int seconds = request.ToInt();
		if (seconds <= 0)
			return "Usage: #tbd safestart [status|go|<seconds>]";

		bool applied = false;
		string reply = safestart.AdminSetSeconds(seconds, applied);
		ok = applied;
		TBD_AdminAudit.Record(string.Format("%1 set safestart length to %2s -> %3",
			TBD_AdminService.Label(callerId), seconds, applied), !applied);
		return reply;
	}

	//! `#tbd identity [status|override <phrase>|enforce]`: whether this host can enforce ONE LIFE,
	//! and the signed waiver when it cannot. A host without backend identities keys players as
	//! `player:<id>`, and TBD_SpawnManager refuses SAFE_START and LIVE there; the waiver lets a test
	//! session start anyway.
	//!   * `status` changes nothing and is readable by any admin.
	//!   * `override` needs `TBD_SpawnManager.IDENTITY_OVERRIDE_PHRASE` verbatim as a separate
	//!     argument, so the waiver cannot be signed by a stray flag.
	//!   * Signed and refused waivers both land in TBD_AdminAudit.
	//!   * `enforce` re-arms the check without a phrase.
	//! The waiver unblocks only the stage gate; the one-life boundary on deploy and the disconnect
	//! handling stay as they are, and each stage it lets through says so in the log.
	//! @param callerId the sender's player id, never taken from the wire
	//! @param arg `status` (also empty), `override` or `enforce`
	//! @param confirm the phrase that must follow `override`
	//! @param ok true when the request did what it asked
	//! @return the line to show the admin; a refusal or a usage line when the request cannot run
	//! @authority server
	static string Identity(int callerId, string arg, string confirm, out bool ok)
	{
		ok = false;

		// Authority only -- the waiver and the census are server-owned, and off the authority
		// vanilla's GetPlayerIdentityId returns NULL_UUID for everybody, so a client build would
		// read a census that is pure noise.
		if (TBD_Authority.IsClient())
			return "TBD: admin actions execute on the server only.";

		if (!TBD_AdminService.IsAdmin(callerId))
		{
			TBD_AdminService.NoteDeniedAccess(callerId, "action 'identity'");
			return "TBD: refused -- you are not a listed server admin.";
		}

		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		if (!spawn)
			return "TBD: spawn manager not on this game mode -- ONE LIFE is not enforced here at all (see the roll-call).";

		string request = arg;
		if (request.IsEmpty())
			request = "status";

		if (request == "status")
		{
			ok = true;
			return spawn.IdentityStatusLine();
		}

		if (request == "enforce")
		{
			spawn.RequireDurableIdentity(TBD_AdminService.Label(callerId));
			ok = true;
			TBD_AdminAudit.Record(string.Format("%1 re-armed ONE LIFE identity enforcement", TBD_AdminService.Label(callerId)), false);
			return "TBD: identity enforcement re-armed -- SAFE_START/LIVE are refused again while any connected player is on a NUMERIC key.";
		}

		if (request == "override")
		{
			if (!spawn.AcceptNonDurableIdentity(TBD_AdminService.Label(callerId), confirm))
			{
				TBD_AdminAudit.Record(string.Format("%1 identity override REFUSED -- wrong or missing confirmation phrase",
					TBD_AdminService.Label(callerId)), true);
				return string.Format("TBD: refused. This waives ONE LIFE on a host that cannot enforce it, so it needs the phrase verbatim: '#tbd identity override %1'.",
					TBD_SpawnManager.IDENTITY_OVERRIDE_PHRASE);
			}

			ok = true;
			TBD_AdminAudit.Record(string.Format("%1 WAIVED ONE LIFE enforcement (no durable player identity on this host)",
				TBD_AdminService.Label(callerId)), false);
			return "TBD: ONE LIFE enforcement WAIVED. SAFE_START/LIVE may now be entered, deaths will NOT survive a reconnect, and every stage this lets through says so in the log. '#tbd identity enforce' undoes it.";
		}

		return string.Format("Usage: #tbd identity [status|override %1|enforce]", TBD_SpawnManager.IDENTITY_OVERRIDE_PHRASE);
	}
}
