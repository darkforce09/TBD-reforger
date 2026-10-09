/**
 * @file TBD_MissionArtifactVerification.c
 * @brief A mission artifact's exact bytes, fetched or cached, hashed and accepted only when their
 * SHA-256 equals the digest the platform published.
 *
 * Role: one verification: fetch the artifact from the platform or take the cached bytes, stage
 * received bytes in a file (`TBD_MissionArtifactCache.StageReceived`), hash them across frames
 * (`TBD_Sha256Job`) and compare.  Position: subclassed by `TBD_BootArtifactVerification`
 * (`TBD_DeployedMission`) and the `load_mission` fleet command, which receive the outcome in
 * `OnFinished`.
 * State: the artifact id, expected digest, pending bytes, digest job and result of one
 * verification, owned by its caller on the server.  Invariants: nothing reads the bytes before the
 * digest matches; received bytes are hashed as read back from disk, so the digest judges exactly the
 * bytes stored; a body over `MISSION_FILE_MAX_BYTES` is refused unhashed; the engine's
 * `RestCallback` exposes no response header, so the route's entity tag and
 * `x-compile-diagnostics-*` headers are not read and the digest is computed here.
 */

//! How a verification ended.
enum TBD_EArtifactVerificationResult
{
	PENDING,    //!< Not finished; the initial result.
	VERIFIED,   //!< m_sDocument holds the bytes; their SHA-256 is the expected one.
	UNANSWERED, //!< No answer, a timeout or a server-side failure: the same fetch may succeed later.
	REFUSED,    //!< The platform refused the fetch, or its answer cannot be used.
	MISMATCH,   //!< The bytes arrived and their SHA-256 differs from the expected one.
}

//! The artifact fetch on its way to the platform.
class TBD_ArtifactFetchCall : TBD_GameRuntimeCall
{
	TBD_MissionArtifactVerification m_Verification; //!< The verification to answer; not owned.

	//! Hand the answer to the verification, when it still exists.
	//! @param answer the platform's answer
	override void OnAnswered(notnull TBD_GameRuntimeAnswer answer)
	{
		if (m_Verification)
			m_Verification.OnFetchAnswered(answer);
	}
}

//! The hash of the bytes under verification.
class TBD_ArtifactDigestJob : TBD_Sha256Job
{
	TBD_MissionArtifactVerification m_Verification; //!< The verification to answer; it owns this job.

	//! Hand the digest to the verification, when it still exists.
	//! @param digest the lowercase hex SHA-256
	//! @param elapsedMs wall-clock milliseconds of the hash
	override void OnHashed(string digest, int elapsedMs)
	{
		if (m_Verification)
			m_Verification.OnHashed(digest, elapsedMs);
	}
}

//! One artifact verification; the owner subclasses it and overrides `OnFinished`.
//! @authority server
class TBD_MissionArtifactVerification
{
	string m_sArtifactId;     //!< The artifact being verified.
	string m_sExpectedSha256; //!< The SHA-256 the platform published, lowercase hex.
	int m_iExpectedBytes; //!< The byte count the platform published, or 0 when unknown (the cache records none).
	bool m_bFromCache; //!< True when the bytes came from the local cache rather than the platform.

	TBD_EArtifactVerificationResult m_eResult = TBD_EArtifactVerificationResult.PENDING; //!< How the verification ended; PENDING until then.
	string m_sDocument; //!< The verified bytes; empty unless the result is VERIFIED.
	string m_sComputedSha256; //!< The SHA-256 of the bytes; empty until hashed.
	string m_sFailure; //!< One log-ready sentence on a result other than VERIFIED.
	int m_iHashMs;     //!< Wall-clock milliseconds of the hash.
	int m_iHashWorkMs; //!< The part of `m_iHashMs` spent hashing.

	protected string m_sPending;                  //!< The bytes being hashed; empty otherwise.
	protected ref TBD_ArtifactDigestJob m_Digest; //!< The hash in flight or last run; null before one starts.

	//! Fetch `artifactId` from the platform and verify it against `expectedSha256`; an unsent
	//! request finishes UNANSWERED at once.
	//! @param artifactId the artifact id
	//! @param expectedSha256 the published SHA-256
	//! @param expectedBytes the published byte count, or 0 when unknown
	//! @route GET /api/v1/game-runtime/artifacts/{artifactId}
	void FetchFromPlatform(string artifactId, string expectedSha256, int expectedBytes)
	{
		m_sArtifactId = artifactId;
		m_sExpectedSha256 = expectedSha256;
		m_iExpectedBytes = expectedBytes;
		m_bFromCache = false;

		TBD_ArtifactFetchCall call = new TBD_ArtifactFetchCall();
		call.m_Verification = this;

		string failure;
		if (TBD_GameRuntimeHttp.Get(call, TBD_GameRuntimeHttp.ROUTE_PREFIX + "/artifacts/" + artifactId, failure))
			return;

		Finish(TBD_EArtifactVerificationResult.UNANSWERED, "the artifact fetch could not be sent: " + failure);
	}

	//! Verify the cached `document` of `artifactId`, read as `bytes`, against `expectedSha256`.
	void CheckCachedBytes(string artifactId, string expectedSha256, string document, notnull array<int> bytes)
	{
		m_sArtifactId = artifactId;
		m_sExpectedSha256 = expectedSha256;
		m_iExpectedBytes = 0;
		m_bFromCache = true;
		Verify(document, bytes);
	}

	//! Stop: no outcome is reported any more.
	void Abandon()
	{
		m_eResult = TBD_EArtifactVerificationResult.REFUSED;
		m_sPending = string.Empty;
		if (m_Digest)
			m_Digest.Cancel();
	}

	//! Receives the outcome; `m_eResult` says which.
	void OnFinished()
	{
	}

	//! Called by TBD_ArtifactFetchCall with the platform's answer.
	void OnFetchAnswered(notnull TBD_GameRuntimeAnswer answer)
	{
		if (m_eResult != TBD_EArtifactVerificationResult.PENDING)
			return;

		if (answer.m_eOutcome == TBD_EGameRuntimeOutcome.TRANSIENT)
		{
			Finish(TBD_EArtifactVerificationResult.UNANSWERED, "the artifact fetch got no answer (" + answer.m_sDetail + ")");
			return;
		}

		if (answer.m_eOutcome != TBD_EGameRuntimeOutcome.SUCCESS)
		{
			string status = typename.EnumToString(HttpCode, answer.m_eCode);
			if (!answer.m_sErrorCode.IsEmpty())
				status += " " + answer.m_sErrorCode;

			Finish(TBD_EArtifactVerificationResult.REFUSED, string.Format("the platform refused the artifact fetch (%1): %2", status, answer.m_sDetail));
			return;
		}

		string document = answer.m_sBody;
		if (document.IsEmpty())
		{
			Finish(TBD_EArtifactVerificationResult.REFUSED, "the artifact fetch answered with no bytes");
			return;
		}

		if (document.Length() > TBD_MissionLoader.MISSION_FILE_MAX_BYTES)
		{
			Finish(TBD_EArtifactVerificationResult.REFUSED, string.Format("the artifact is %1 bytes, over the %2-byte mission cap",
				document.Length(), TBD_MissionLoader.MISSION_FILE_MAX_BYTES));
			return;
		}

		// A byte count that differs from the published one is already a failed verification; the hash is
		// computed anyway so the log names both digests.
		if (m_iExpectedBytes > 0 && document.Length() != m_iExpectedBytes)
			TBD_Log.Warn(TBD_Log.CH_MISSION, string.Format("artifact %1 arrived with %2 bytes; the platform published %3", m_sArtifactId, document.Length(), m_iExpectedBytes));

		array<int> bytes;
		string failure;
		if (!TBD_MissionArtifactCache.StageReceived(document, bytes, failure))
		{
			Finish(TBD_EArtifactVerificationResult.REFUSED, "the received artifact could not be staged for hashing: " + failure);
			return;
		}

		Verify(document, bytes);
	}

	//! Called by TBD_ArtifactDigestJob with the SHA-256 of the pending bytes.
	void OnHashed(string digest, int elapsedMs)
	{
		if (m_eResult != TBD_EArtifactVerificationResult.PENDING)
			return;

		m_sComputedSha256 = digest;
		m_iHashMs = elapsedMs;
		if (m_Digest)
			m_iHashWorkMs = m_Digest.GetWorkMs();

		if (digest != m_sExpectedSha256)
		{
			string origin = "received";
			if (m_bFromCache)
				origin = "cached";

			int length = m_sPending.Length();
			m_sPending = string.Empty;
			Finish(TBD_EArtifactVerificationResult.MISMATCH, string.Format("SHA-256 MISMATCH for artifact %1: the %2 %3 bytes hash to %4, the platform published %5",
				m_sArtifactId, length, origin, digest, m_sExpectedSha256));
			return;
		}

		m_sDocument = m_sPending;
		m_sPending = string.Empty;
		Finish(TBD_EArtifactVerificationResult.VERIFIED, string.Empty);
	}

	//! A short description of where the bytes came from and how the check went, for log lines.
	string Describe()
	{
		string origin = "platform";
		if (m_bFromCache)
			origin = "cache";

		return string.Format("artifact=%1 sha256=%2 bytes=%3 from=%4 hashMs=%5 hashWorkMs=%6",
			m_sArtifactId, m_sComputedSha256, m_sDocument.Length(), origin, m_iHashMs, m_iHashWorkMs);
	}

	//! Hash `bytes` and hold `document` pending until the digest answers.
	//! @param document the bytes as text
	//! @param bytes the same bytes, one per element
	protected void Verify(string document, notnull array<int> bytes)
	{
		m_sPending = document;
		m_Digest = new TBD_ArtifactDigestJob();
		m_Digest.m_Verification = this;
		m_Digest.Start(bytes);
	}

	//! Record the result and report it to the owner.
	//! @param result how the verification ended
	//! @param failure one log-ready sentence, empty on VERIFIED
	protected void Finish(TBD_EArtifactVerificationResult result, string failure)
	{
		m_eResult = result;
		m_sFailure = failure;
		OnFinished();
	}
}
