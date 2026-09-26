/**
 * @file TBD_DeployedMission.c
 * @brief Boots every world into the mission deployed to this server: deployment read, artifact
 * from cache or platform, SHA-256 verification, load.
 *
 * Role: the boot sequence, once per world from `TBD_MissionLoader.BeginLoad`: (1) read the
 * deployment in effect (the one in flight, else the latest confirmed); (2) take its artifact from
 * `TBD_MissionArtifactCache` when the cached id and SHA-256 match and the cached bytes still hash to
 * them, else fetch it and verify the received bytes before anything reads them
 * (`TBD_MissionArtifactVerification`), then cache them; (3) `TBD_MissionLoader.LoadDocument`;
 * (4) declare the loaded artifact (`TBD_LoadedArtifactReport`), which starts the runtime session
 * and confirms the deployment on the platform; (5) the stage machine then loads the event roster
 * (`TBD_RosterLoader` reads `GetEventId`).  Position: fed by the platform's game-runtime routes;
 * read by the roster, deployment authorization and the results report.
 * State: the deployment this world runs, its source, the verification in flight and a per-world
 * generation, all static on the server and reset by `Begin`.  Invariants: 404 `NO_DEPLOYMENT` runs
 * no mission (ERROR) and there is no default mission; an unreadable deployment runs the last
 * verified cached artifact with a WARNING, else no mission (ERROR) and the read repeats with backoff
 * from 2 s to 60 s after no answer, otherwise every 60 s, re-reading the backend config each time;
 * a failed fetch or a SHA-256 mismatch loads nothing (ERROR) and the sequence repeats; a document
 * that fails validation runs no mission and the session reports none; an answer, verification or
 * retry of an earlier world is dropped.
 */

//! The deployment read on its way to the platform.
class TBD_DeploymentReadCall : TBD_GameRuntimeCall
{
	int m_iGeneration; //!< The world generation the read was sent in.

	//! Hand the answer to `TBD_DeployedMission.OnDeploymentAnswered`.
	//! @param answer the platform's answer
	override void OnAnswered(notnull TBD_GameRuntimeAnswer answer)
	{
		TBD_DeployedMission.OnDeploymentAnswered(this, answer);
	}
}

//! A verification the boot flow started, with what it is for.
class TBD_BootArtifactVerification : TBD_MissionArtifactVerification
{
	int m_iGeneration;                          //!< The world generation the verification started in.
	ref TBD_RuntimeDeploymentStruct m_Identity; //!< The deployment whose artifact is verified.
	bool m_bFallback;                           //!< True when this checks the last verified artifact because the deployment could not be read.
	string m_sUnreadableKey;                    //!< Fallback only: the report key of why the deployment could not be read.
	string m_sUnreadableWhy;                    //!< Fallback only: why the deployment could not be read, for the retry.
	bool m_bUnreadableTransient;                //!< Fallback only: true when the read got no answer (backoff applies).

	//! Hand the verdict to `TBD_DeployedMission.OnVerificationFinished`.
	override void OnFinished()
	{
		TBD_DeployedMission.OnVerificationFinished(m_iGeneration);
	}
}

//! The boot sequence and the deployment this world runs.
class TBD_DeployedMission
{
	protected static const string CH_MISSION = "Mission"; //!< Log channel.
	protected static const int RETRY_BASE_MS = 2000;      //!< First backoff delay after no answer, milliseconds.
	protected static const int RETRY_CAP_MS = 60000;      //!< Longest retry delay, milliseconds; also the delay after a refusal.

	protected static int s_iGeneration; //!< Bumped per world; answers, verifications and retries of an earlier world are dropped.
	protected static bool s_bDecided; //!< This world's outcome is settled: a mission loaded, or none runs.
	protected static string s_sSource = "none"; //!< "platform", "cache", "last-verified-cache" or "none".
	protected static ref TBD_RuntimeDeploymentStruct s_Deployment; //!< The deployment this world runs, or null.
	protected static ref TBD_BootArtifactVerification s_Verification; //!< The verification in flight or last finished; null before one starts.
	protected static int s_iUnanswered; //!< Consecutive unanswered reads or fetches this world; drives the backoff.
	protected static ref map<string, bool> s_mReported; //!< Problems already reported at ERROR this world.

	//! Whether this world's outcome is settled: a mission loaded, or none runs.
	//! @return true once settled
	static bool IsDecided()
	{
		return s_bDecided;
	}

	//! Where the running artifact came from: "platform", "cache", "last-verified-cache" or "none".
	static string GetSource()
	{
		return s_sSource;
	}

	//! The platform event the running mission is deployed for, or empty.
	static string GetEventId()
	{
		if (!s_Deployment)
			return string.Empty;

		return s_Deployment.event_id;
	}

	//! The event mission the running mission is deployed as, or empty.
	static string GetEventMissionId()
	{
		if (!s_Deployment)
			return string.Empty;

		return s_Deployment.event_mission_id;
	}

	//! The catalog mission running, or empty.
	static string GetMissionId()
	{
		if (!s_Deployment)
			return string.Empty;

		return s_Deployment.mission_id;
	}

	//! The artifact running, or empty.
	static string GetArtifactId()
	{
		if (!s_Deployment)
			return string.Empty;

		return s_Deployment.artifact_id;
	}

	//! The terrain key of the running deployment, or empty.
	static string GetTerrainKey()
	{
		if (!s_Deployment)
			return string.Empty;

		return s_Deployment.terrain_key;
	}

	//! Start this world's boot sequence. Everything an earlier world decided is dropped.
	static void Begin()
	{
		s_iGeneration++;
		s_bDecided = false;
		s_sSource = "none";
		s_Deployment = null;
		if (s_Verification)
			s_Verification.Abandon();

		s_Verification = null;
		s_iUnanswered = 0;
		s_mReported = null;

		TBD_LoadedArtifactReport.Reset();
		TBD_RosterLoader.Reset();
		TBD_Sha256SelfTest.Passed();
		TBD_BackendConfig.Load();
		ReadDeployment();
	}

	//! Read the deployment in effect; an unconfigured credential or an unsent request goes to the
	//! cached fallback at once.
	//! @route GET /api/v1/game-runtime/deployment
	//! @authority server
	protected static void ReadDeployment()
	{
		if (!TBD_GameRuntimeHttp.IsConfigured())
		{
			DeploymentUnreadable("unconfigured", "no machine credential is configured (backend=" + TBD_GameRuntimeHttp.DescribeBackend() + ")", false);
			return;
		}

		TBD_Log.Kv(CH_MISSION, "deployment-read", "backend=" + TBD_GameRuntimeHttp.DescribeBackend());

		TBD_DeploymentReadCall call = new TBD_DeploymentReadCall();
		call.m_iGeneration = s_iGeneration;
		string failure;
		if (TBD_GameRuntimeHttp.Get(call, TBD_GameRuntimeHttp.ROUTE_PREFIX + "/deployment", failure))
			return;

		DeploymentUnreadable("unsent", "the deployment read could not be sent (" + failure + ")", true);
	}

	//! Called by TBD_DeploymentReadCall with the platform's answer.
	static void OnDeploymentAnswered(notnull TBD_DeploymentReadCall call, notnull TBD_GameRuntimeAnswer answer)
	{
		if (call.m_iGeneration != s_iGeneration || s_bDecided)
			return;

		if (answer.m_eOutcome == TBD_EGameRuntimeOutcome.SUCCESS)
		{
			string problem;
			TBD_RuntimeDeploymentStruct deployment = TBD_RuntimeDeploymentStruct.Parse(answer.m_sBody, problem);
			if (deployment)
				UseDeployment(deployment);
			else
				DeploymentUnreadable("unusable", "the deployment answer is unusable: " + problem, false);

			return;
		}

		if (answer.m_eCode == HttpCode.HTTP_CODE_404 && answer.m_sErrorCode == "NO_DEPLOYMENT")
		{
			RunNoMission("the platform deploys no mission to this server (404 NO_DEPLOYMENT)");
			return;
		}

		if (answer.m_eOutcome == TBD_EGameRuntimeOutcome.TRANSIENT)
		{
			DeploymentUnreadable("unanswered", "the deployment read got no answer (" + answer.m_sDetail + ")", true);
			return;
		}

		DeploymentUnreadable(typename.EnumToString(HttpCode, answer.m_eCode), "the platform refused the deployment read (" + answer.m_sDetail + ")", false);
	}

	//! The deployment is known: load its artifact from the cache when the cached copy is this very
	//! artifact, else from the platform.
	protected static void UseDeployment(notnull TBD_RuntimeDeploymentStruct deployment)
	{
		TBD_Log.Kv(CH_MISSION, "deployment", string.Format("deployment=%1 state=%2 mission=%3 artifact=%4 sha256=%5 bytes=%6 terrain=%7 event=%8 eventMission=%9",
			deployment.deployment_id, deployment.state, deployment.mission_id, deployment.artifact_id, deployment.artifact_sha256,
			deployment.artifact_bytes, deployment.terrain_key, deployment.event_id, deployment.event_mission_id));

		TBD_RuntimeDeploymentStruct cached = TBD_MissionArtifactCache.ReadIdentity();
		if (cached && cached.artifact_id == deployment.artifact_id && cached.artifact_sha256 == deployment.artifact_sha256)
		{
			string document;
			array<int> bytes;
			string failure;
			if (TBD_MissionArtifactCache.ReadDocument(document, bytes, failure))
			{
				NewVerification(deployment).CheckCachedBytes(deployment.artifact_id, deployment.artifact_sha256, document, bytes);
				return;
			}

			TBD_Log.Warn(CH_MISSION, string.Format("the cached copy of artifact %1 cannot be read (%2) - fetching it", deployment.artifact_id, failure));
		}

		NewVerification(deployment).FetchFromPlatform(deployment.artifact_id, deployment.artifact_sha256, deployment.artifact_bytes);
	}

	//! The deployment cannot be read: the last verified artifact runs when the cache holds one.
	protected static void DeploymentUnreadable(string key, string why, bool transient)
	{
		TBD_RuntimeDeploymentStruct cached = TBD_MissionArtifactCache.ReadIdentity();
		if (cached)
		{
			string document;
			array<int> bytes;
			string failure;
			if (TBD_MissionArtifactCache.ReadDocument(document, bytes, failure))
			{
				TBD_BootArtifactVerification verification = NewVerification(cached);
				verification.m_bFallback = true;
				verification.m_sUnreadableKey = key;
				verification.m_sUnreadableWhy = why;
				verification.m_bUnreadableTransient = transient;
				verification.CheckCachedBytes(cached.artifact_id, cached.artifact_sha256, document, bytes);
				return;
			}

			TBD_Log.Warn(CH_MISSION, string.Format("the last verified artifact in %1 cannot be read (%2)", TBD_MissionArtifactCache.DescribeLocation(), failure));
		}

		WaitForDeployment(key, why, transient);
	}

	//! No mission yet: an ERROR once per distinct problem, and the deployment is read again.
	protected static void WaitForDeployment(string key, string why, bool transient)
	{
		int delay = RETRY_CAP_MS;
		if (transient)
		{
			s_iUnanswered++;
			delay = TBD_GameRuntimeHttp.BackoffMs(s_iUnanswered, RETRY_BASE_MS, RETRY_CAP_MS);
		}

		string text = string.Format("NO MISSION YET - %1, and no verified artifact is cached in %2. Reading the deployment again in %3 ms (backoff up to %4 s); a machine credential added to the profile is picked up without a restart.",
			why, TBD_MissionArtifactCache.DescribeLocation(), delay, RETRY_CAP_MS / 1000);
		if (!ReportOnce(key, text) && transient)
			TBD_Log.Warn(CH_MISSION, string.Format("%1 - attempt %2, reading the deployment again in %3 ms", why, s_iUnanswered, delay));

		ScheduleRetry(delay);
	}

	//! The platform deploys nothing here: no mission runs, and the session reports none.
	protected static void RunNoMission(string why)
	{
		s_bDecided = true;
		s_sSource = "none";
		s_Deployment = null;
		TBD_Log.Error(CH_MISSION, "NO MISSION - " + why + ". This world runs no mission and no default mission is loaded; deploy a mission to this server on the platform, or in game with '#tbd missions'.");
		TBD_LoadedArtifactReport.DeclareNone();
	}

	//! Called by TBD_BootArtifactVerification when a verification ends. Acted on in the next frame,
	//! once the verification's own call stack has unwound, since acting may replace it.
	static void OnVerificationFinished(int generation)
	{
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
			queue.CallLater(SettleVerification, 0, false, generation);
	}

	//! Settle a finished verification of this world by its kind: fallback, cached copy or fetch.
	//! @param generation the world generation it finished in; a stale one is ignored
	protected static void SettleVerification(int generation)
	{
		TBD_BootArtifactVerification verification = s_Verification;
		if (generation != s_iGeneration || s_bDecided || !verification)
			return;

		if (verification.m_eResult == TBD_EArtifactVerificationResult.PENDING)
			return;

		if (verification.m_bFallback)
			SettleFallback(verification);
		else if (verification.m_bFromCache)
			SettleCachedCopy(verification);
		else
			SettleFetch(verification);
	}

	//! The last verified artifact checked: load it with a WARNING, or wait for the deployment.
	protected static void SettleFallback(notnull TBD_BootArtifactVerification verification)
	{
		if (verification.m_eResult != TBD_EArtifactVerificationResult.VERIFIED)
		{
			TBD_Log.Warn(CH_MISSION, "the last verified artifact fails verification now: " + verification.m_sFailure);
			WaitForDeployment(verification.m_sUnreadableKey, verification.m_sUnreadableWhy, verification.m_bUnreadableTransient);
			return;
		}

		TBD_RuntimeDeploymentStruct identity = verification.m_Identity;
		TBD_Log.Warn(CH_MISSION, string.Format("RUNNING THE LAST VERIFIED ARTIFACT - %1. artifact=%2 mission=%3 deployment=%4 event=%5; it is reported to the platform once a session can start.",
			verification.m_sUnreadableWhy, identity.artifact_id, identity.mission_id, identity.deployment_id, identity.event_id));
		Load(verification, "last-verified-cache");
	}

	//! The cached copy checked: load it and re-point the identity, or fetch the artifact.
	protected static void SettleCachedCopy(notnull TBD_BootArtifactVerification verification)
	{
		TBD_RuntimeDeploymentStruct deployment = verification.m_Identity;
		if (verification.m_eResult != TBD_EArtifactVerificationResult.VERIFIED)
		{
			TBD_Log.Warn(CH_MISSION, string.Format("the cached copy of artifact %1 fails verification (%2) - fetching it", deployment.artifact_id, verification.m_sFailure));
			NewVerification(deployment).FetchFromPlatform(deployment.artifact_id, deployment.artifact_sha256, deployment.artifact_bytes);
			return;
		}

		// Same bytes, possibly a newer deployment of them: the identity names it from now on.
		TBD_MissionArtifactCache.StoreIdentity(deployment);
		Load(verification, "cache");
	}

	//! The fetched bytes checked: cache and load them, retry with backoff after no answer, or report
	//! the failure once and retry every 60 s.
	protected static void SettleFetch(notnull TBD_BootArtifactVerification verification)
	{
		TBD_RuntimeDeploymentStruct deployment = verification.m_Identity;
		if (verification.m_eResult == TBD_EArtifactVerificationResult.VERIFIED)
		{
			string failure;
			if (!TBD_MissionArtifactCache.Store(verification.m_sDocument, deployment, failure))
				TBD_Log.Warn(CH_MISSION, string.Format("artifact %1 is verified but not cached (%2) - the next boot fetches it again", deployment.artifact_id, failure));

			Load(verification, "platform");
			return;
		}

		if (verification.m_eResult == TBD_EArtifactVerificationResult.UNANSWERED)
		{
			s_iUnanswered++;
			int delay = TBD_GameRuntimeHttp.BackoffMs(s_iUnanswered, RETRY_BASE_MS, RETRY_CAP_MS);
			TBD_Log.Warn(CH_MISSION, string.Format("artifact %1 not loaded: %2 - attempt %3, reading the deployment again in %4 ms",
				deployment.artifact_id, verification.m_sFailure, s_iUnanswered, delay));
			ScheduleRetry(delay);
			return;
		}

		ReportOnce("artifact:" + verification.m_sFailure, string.Format("NO MISSION YET - artifact %1 NOT LOADED: %2. Nothing is loaded from it; reading the deployment again every %3 s.",
			deployment.artifact_id, verification.m_sFailure, RETRY_CAP_MS / 1000));
		ScheduleRetry(RETRY_CAP_MS);
	}

	//! Parse and validate the verified bytes; declare what the session reports.
	protected static void Load(notnull TBD_BootArtifactVerification verification, string source)
	{
		TBD_RuntimeDeploymentStruct identity = verification.m_Identity;
		s_bDecided = true;
		TBD_Log.Kv(CH_MISSION, "artifact-verified", verification.Describe());

		if (!TBD_MissionLoader.LoadDocument(verification.m_sDocument, source))
		{
			s_sSource = "none";
			TBD_Log.Error(CH_MISSION, string.Format("NO MISSION - artifact %1 (sha256 %2) is verified but does not load in this build; the [TBD][Validate] lines say why. The session reports no artifact.",
				identity.artifact_id, identity.artifact_sha256));
			TBD_LoadedArtifactReport.DeclareNone();
			return;
		}

		s_Deployment = identity;
		s_sSource = source;

		string documentMissionId = TBD_MissionLoader.GetMissionId();
		if (!identity.mission_id.IsEmpty() && documentMissionId != identity.mission_id)
			TBD_Log.Warn(CH_MISSION, string.Format("the artifact's meta.id '%1' is not the deployment's mission '%2'", documentMissionId, identity.mission_id));

		TBD_LoadedArtifactReport.DeclareLoaded(identity.artifact_id, identity.artifact_sha256);
	}

	//! Start a verification for `identity` in this world, replacing the previous one.
	//! @return the new verification
	protected static TBD_BootArtifactVerification NewVerification(notnull TBD_RuntimeDeploymentStruct identity)
	{
		s_Verification = new TBD_BootArtifactVerification();
		s_Verification.m_iGeneration = s_iGeneration;
		s_Verification.m_Identity = identity;
		return s_Verification;
	}

	//! Read the deployment again after `delayMs` milliseconds, in this world only.
	protected static void ScheduleRetry(int delayMs)
	{
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
			queue.CallLater(Retry, delayMs, false, s_iGeneration);
	}

	//! Re-read the backend config and the deployment, unless the world moved on or settled.
	protected static void Retry(int generation)
	{
		if (generation != s_iGeneration || s_bDecided)
			return;

		TBD_BackendConfig.Reload();
		ReadDeployment();
	}

	//! An ERROR the first time `key` is reported this world. False for a repeat, which logs nothing.
	protected static bool ReportOnce(string key, string text)
	{
		if (!s_mReported)
			s_mReported = new map<string, bool>();

		if (s_mReported.Contains(key))
			return false;

		s_mReported.Set(key, true);
		TBD_Log.Error(CH_MISSION, text);
		return true;
	}
}
