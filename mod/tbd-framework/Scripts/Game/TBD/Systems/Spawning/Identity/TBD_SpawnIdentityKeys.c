/**
 * @file TBD_SpawnIdentityKeys.c
 * @brief The durable "who is this player" key the spawn manager binds seats, bodies and lives to.
 *
 * Role: resolves a numeric player id to a bind key and classifies the key.  Position: owned by
 * TBD_SpawnManager; read by every spawn helper that keys state on a person rather than on a number.
 * State: the per-id key cache and the per-mode degraded-warning latch, server only.
 * Invariants: SCR_PlayerIdentityUtils.GetPlayerIdentityId answers only on the authority and only
 * after OnPlayerAuditSuccess, so every caller resolves at or after that hook, or at disconnect where
 * the cache answers; a cached key is dropped at disconnect so a recycled id never inherits it.
 */

//! Bind keys come in three modes, named by KeyModeLabel:
//! BACKEND - a backend identity (a registered dedicated server); durable.
//! NAME-HASH - vanilla's synthesized `00bbbddd-` uuid, hashed from the display name on a listen or
//! hosted server; an identity, but a rename changes it and a shared name shares it.
//! NUMERIC - `player:<id>`, a lease on a number (a dedicated server with no backend identity, or a
//! player already torn down with no cached key); never an identity.
class TBD_SpawnIdentityKeys : Managed
{
	protected ref map<int, string> m_mBindKeyCache; //!< playerId to the key last resolved for it; dropped at disconnect
	protected ref map<string, bool> m_mIdentityDegradedLogged; //!< KeyModeLabel values already warned about this session

	//! Create empty tables.
	void TBD_SpawnIdentityKeys()
	{
		m_mBindKeyCache = new map<int, string>();
		m_mIdentityDegradedLogged = new map<string, bool>();
	}

	//! The bind key of `playerId`: the live identity when the engine answers (cached), else the key
	//! cached for this id, else `player:<id>`. Warns once per degraded mode.
	//! @return never empty
	//! @authority server
	string PlayerBindKey(int playerId)
	{
		UUID identity = SCR_PlayerIdentityUtils.GetPlayerIdentityId(playerId);
		if (!identity.IsNull())
		{
			string identityId = string.Format("%1", identity);
			if (!identityId.IsEmpty())
			{
				if (IsSyntheticIdentity(identityId))
					NoteIdentityDegraded(playerId, "NAME-HASH", "this host issues NAME-DERIVED identities (vanilla's 00bbbddd- peer-tool fallback, listen/hosted server), so changing your display name buys a fresh life and two players sharing a name share one life and one seat");

				m_mBindKeyCache.Set(playerId, identityId);
				return identityId;
			}
		}

		// Live lookup unavailable (teardown, or an identity-less host). A key we resolved
		// earlier for this numeric id is strictly better than inventing a new one.
		string cached;
		if (m_mBindKeyCache.Find(playerId, cached))
			return cached;

		NoteIdentityDegraded(playerId, "NUMERIC", "this host issues NO identity at all -- a DEDICATED server that is not registered with the BI backend (vanilla's own diagnostic names the server config's publicAddress), or a player already being torn down. NOTE: this is NOT what local PIE looks like -- a PIE/listen host is not Dedicated, so vanilla synthesizes a name hash there instead. ONE LIFE is only as durable as the numeric playerId and a reconnect buys a fresh life");
		return string.Format("player:%1", playerId);
	}

	//! Drop the cached key of `playerId`, so the next resolve asks the engine again.
	void ForgetCachedKey(int playerId)
	{
		m_mBindKeyCache.Remove(playerId);
	}

	//! True for vanilla's synthesized name-hash identity, which always carries the `00bbbddd-` prefix.
	static bool IsSyntheticIdentity(string key)
	{
		return key.StartsWith("00bbbddd-");
	}

	//! True when `key` names the same person across a reconnect and a rename: a backend identity.
	//! Gates the identity-to-slot reclaim table, where a wrong answer seats the wrong person.
	static bool IsDurableKey(string key)
	{
		return !key.IsEmpty() && !key.StartsWith("player:") && !IsSyntheticIdentity(key);
	}

	//! True when `key` names a person (BACKEND or NAME-HASH) rather than a seat number (NUMERIC).
	//! Gates every match that must never pair a player with a recycled id.
	static bool IsIdentityKey(string key)
	{
		return !key.IsEmpty() && !key.StartsWith("player:");
	}

	//! Which mode produced `key`, as one word: BACKEND, NAME-HASH or NUMERIC.
	static string KeyModeLabel(string key)
	{
		if (!IsIdentityKey(key))
			return "NUMERIC";

		if (IsSyntheticIdentity(key))
			return "NAME-HASH";

		return "BACKEND";
	}

	//! Warn once per `mode` (a KeyModeLabel value) that ONE LIFE is not durably enforceable here.
	//! @param why the mode's consequence, for the operator
	//! @authority server
	void NoteIdentityDegraded(int playerId, string mode, string why)
	{
		if (m_mIdentityDegradedLogged.Contains(mode))
			return;
		m_mIdentityDegradedLogged.Set(mode, true);

		string verdict = "Expected on a local/listen host; NOT acceptable for an event server.";
		if (mode == "NUMERIC")
			verdict = "This is NOT a supported state: vanilla itself reports a dedicated server with no backend identity as a MISCONFIGURATION. SAFE_START/LIVE are refused until it is fixed -- run '#tbd identity' for the verdict and the escape hatch.";

		Print(string.Format("[TBD][Spawn] player=%1 has NO durable identity (keyMode=%2) -- %3. %4", playerId, mode, why, verdict), LogLevel.WARNING);
	}
}
