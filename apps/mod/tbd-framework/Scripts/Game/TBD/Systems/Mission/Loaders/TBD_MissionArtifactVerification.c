//! A mission artifact's exact bytes, verified: fetched from the platform
//! (`GET /api/v1/game-runtime/artifacts/{artifact_id}`, mod_runtime credential) or taken from the
//! local cache, then hashed (TBD_Sha256Job) and accepted only when the SHA-256 of the bytes equals
//! the one the platform published for the artifact. Nothing reads the bytes before that. Received
//! bytes are staged in a file and hashed as read back from it (TBD_MissionArtifactCache), so the
//! digest judges exactly the bytes on disk.
//!
//! The artifact route answers the exact compiled bytes with their SHA-256 as strong entity tag and
//! the compile's findings in `x-compile-diagnostics-*` headers. The engine's RestCallback exposes no
//! response header, so the digest is computed here and the findings are not read.
//!
//! The owner subclasses this class and receives the outcome in `OnFinished`.
//! @authority server

//! How a verification ended.
enum TBD_EArtifactVerificationResult
{
	PENDING,
	VERIFIED,   //!< m_sDocument holds the bytes; their SHA-256 is the expected one.
	UNANSWERED, //!< No answer, a timeout or a server-side failure: the same fetch may succeed later.
	REFUSED,    //!< The platform refused the fetch, or its answer cannot be used.
	MISMATCH,   //!< The bytes arrived and their SHA-256 differs from the expected one.
}

//! The artifact fetch on its way to the platform.
class TBD_ArtifactFetchCall : TBD_GameRuntimeCall
{
	TBD_MissionArtifactVerification m_Verification;

	//------------------------------------------------------------------------------------------------
	override void OnAnswered(notnull TBD_GameRuntimeAnswer answer)
	{
		if (m_Verification)
			m_Verification.OnFetchAnswered(answer);
	}
}

//! The hash of the bytes under verification.
class TBD_ArtifactDigestJob : TBD_Sha256Job
{
	TBD_MissionArtifactVerification m_Verification;

	//------------------------------------------------------------------------------------------------
	override void OnHashed(string digest, int elapsedMs)
	{
		if (m_Verification)
			m_Verification.OnHashed(digest, elapsedMs);
	}
}

class TBD_MissionArtifactVerification
{
	string m_sArtifactId;
	string m_sExpectedSha256;
	//! The byte count the platform published, or 0 when unknown (the cache records none).
	int m_iExpectedBytes;
	//! True when the bytes came from the local cache rather than the platform.
	bool m_bFromCache;

	TBD_EArtifactVerificationResult m_eResult = TBD_EArtifactVerificationResult.PENDING;
	//! The verified bytes; empty unless the result is VERIFIED.
	string m_sDocument;
	string m_sComputedSha256;
	//! One log-ready sentence on a result other than VERIFIED.
	string m_sFailure;
	//! Wall-clock milliseconds of the hash, and the part of them spent hashing.
	int m_iHashMs;
	int m_iHashWorkMs;

	protected string m_sPending;
	protected ref TBD_ArtifactDigestJob m_Digest;

	//------------------------------------------------------------------------------------------------
	//! Fetch `artifactId` from the platform and verify it against `expectedSha256`.
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

	//------------------------------------------------------------------------------------------------
	//! Verify the cached `document` of `artifactId`, read as `bytes`, against `expectedSha256`.
	void CheckCachedBytes(string artifactId, string expectedSha256, string document, notnull array<int> bytes)
	{
		m_sArtifactId = artifactId;
		m_sExpectedSha256 = expectedSha256;
		m_iExpectedBytes = 0;
		m_bFromCache = true;
		Verify(document, bytes);
	}

	//------------------------------------------------------------------------------------------------
	//! Stop: no outcome is reported any more.
	void Abandon()
	{
		m_eResult = TBD_EArtifactVerificationResult.REFUSED;
		m_sPending = string.Empty;
		if (m_Digest)
			m_Digest.Cancel();
	}

	//------------------------------------------------------------------------------------------------
	//! Receives the outcome; `m_eResult` says which.
	void OnFinished()
	{
	}

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	//! A short description of where the bytes came from and how the check went, for log lines.
	string Describe()
	{
		string origin = "platform";
		if (m_bFromCache)
			origin = "cache";

		return string.Format("artifact=%1 sha256=%2 bytes=%3 from=%4 hashMs=%5 hashWorkMs=%6",
			m_sArtifactId, m_sComputedSha256, m_sDocument.Length(), origin, m_iHashMs, m_iHashWorkMs);
	}

	//------------------------------------------------------------------------------------------------
	protected void Verify(string document, notnull array<int> bytes)
	{
		m_sPending = document;
		m_Digest = new TBD_ArtifactDigestJob();
		m_Digest.m_Verification = this;
		m_Digest.Start(bytes);
	}

	//------------------------------------------------------------------------------------------------
	protected void Finish(TBD_EArtifactVerificationResult result, string failure)
	{
		m_eResult = result;
		m_sFailure = failure;
		OnFinished();
	}
}
