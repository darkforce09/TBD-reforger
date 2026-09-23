//! The mission this world runs: the artifact of the mission deployed to this server on the platform,
//! loaded only when the SHA-256 of its exact bytes equals the digest the platform published.
//!
//! Boot order, once per world (TBD_MissionLoader.BeginLoad, from TBD_FrameworkManager.OnPostInit):
//!   1. read the deployment in effect, `GET /api/v1/game-runtime/deployment` (mod_runtime
//!      credential): the one in flight, else the latest confirmed one;
//!   2. take its artifact from the local cache (TBD_MissionArtifactCache) when the cached id and
//!      SHA-256 are the deployment's and the cached bytes still hash to them; otherwise fetch it,
//!      `GET /api/v1/game-runtime/artifacts/{artifact_id}`, verify the received bytes before
//!      anything reads them (TBD_MissionArtifactVerification), and cache them;
//!   3. parse and validate the document (TBD_MissionLoader.LoadDocument);
//!   4. declare the loaded artifact (TBD_LoadedArtifactReport): the runtime session starts,
//!      reporting it, which is what confirms the deployment on the platform;
//!   5. when the deployment names an event, the stage machine loads the event roster and its slot
//!      table (TBD_RosterLoader reads GetEventId), and deployment authorization applies to it.
//!
//! Failure modes:
//!   * 404 `NO_DEPLOYMENT`: no mission runs (ERROR), and the session starts reporting none. There is
//!     no default mission.
//!   * the deployment cannot be read (no machine credential, no answer, a refusal): the last
//!     verified artifact in the cache runs, with a WARNING, and is reported once a session can
//!     start. Without one no mission runs (ERROR) and the read is repeated, after no answer with
//!     backoff from 2 s to 60 s, otherwise every 60 s, re-reading the backend config each time.
//!   * the fetch fails, or the SHA-256 differs: nothing loads (ERROR), and the sequence repeats from
//!     step 1 on the same schedule.
//!   * the verified document fails validation: no mission runs, and the session reports none.
//! @authority server

//! The deployment read on its way to the platform.
class TBD_DeploymentReadCall : TBD_GameRuntimeCall
{
	int m_iGeneration;

	//------------------------------------------------------------------------------------------------
	override void OnAnswered(notnull TBD_GameRuntimeAnswer answer)
	{
		TBD_DeployedMission.OnDeploymentAnswered(this, answer);
	}
}

//! A verification the boot flow started, with what it is for.
class TBD_BootArtifactVerification : TBD_MissionArtifactVerification
{
	int m_iGeneration;
	ref TBD_RuntimeDeploymentStruct m_Identity;
	//! The last verified artifact, checked because the deployment could not be read; the fields
	//! below say why, for the retry when the check fails.
	bool m_bFallback;
	string m_sUnreadableKey;
	string m_sUnreadableWhy;
	bool m_bUnreadableTransient;

	//------------------------------------------------------------------------------------------------
	override void OnFinished()
	{
		TBD_DeployedMission.OnVerificationFinished(m_iGeneration);
	}
}

class TBD_DeployedMission
{
	protected static const string CH_MISSION = "Mission";
	protected static const int RETRY_BASE_MS = 2000;
	protected static const int RETRY_CAP_MS = 60000;

	//! Bumped per world; answers, verifications and retries of an earlier world are dropped.
	protected static int s_iGeneration;
	//! This world's outcome is settled: a mission loaded, or none runs.
	protected static bool s_bDecided;
	//! "platform", "cache", "last-verified-cache" or "none".
	protected static string s_sSource = "none";
	//! The deployment this world runs, or null.
	protected static ref TBD_RuntimeDeploymentStruct s_Deployment;
	protected static ref TBD_BootArtifactVerification s_Verification;
	protected static int s_iUnanswered;
	//! Problems already reported at ERROR this world.
	protected static ref map<string, bool> s_mReported;

	//------------------------------------------------------------------------------------------------
	// STATE FOR OTHER SYSTEMS
	//------------------------------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	static bool IsDecided()
	{
		return s_bDecided;
	}

	//------------------------------------------------------------------------------------------------
	//! Where the running artifact came from: "platform", "cache", "last-verified-cache" or "none".
	static string GetSource()
	{
		return s_sSource;
	}

	//------------------------------------------------------------------------------------------------
	//! The platform event the running mission is deployed for, or empty.
	static string GetEventId()
	{
		if (!s_Deployment)
			return string.Empty;

		return s_Deployment.event_id;
	}

	//------------------------------------------------------------------------------------------------
	//! The event mission the running mission is deployed as, or empty.
	static string GetEventMissionId()
	{
		if (!s_Deployment)
			return string.Empty;

		return s_Deployment.event_mission_id;
	}

	//------------------------------------------------------------------------------------------------
	//! The catalog mission running, or empty.
	static string GetMissionId()
	{
		if (!s_Deployment)
			return string.Empty;

		return s_Deployment.mission_id;
	}

	//------------------------------------------------------------------------------------------------
	static string GetArtifactId()
	{
		if (!s_Deployment)
			return string.Empty;

		return s_Deployment.artifact_id;
	}

	//------------------------------------------------------------------------------------------------
	static string GetTerrainKey()
	{
		if (!s_Deployment)
			return string.Empty;

		return s_Deployment.terrain_key;
	}

	//------------------------------------------------------------------------------------------------
	// BOOT
	//------------------------------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	//! The platform deploys nothing here: no mission runs, and the session reports none.
	protected static void RunNoMission(string why)
	{
		s_bDecided = true;
		s_sSource = "none";
		s_Deployment = null;
		TBD_Log.Error(CH_MISSION, "NO MISSION - " + why + ". This world runs no mission and no default mission is loaded; deploy a mission to this server on the platform, or in game with '#tbd missions'.");
		TBD_LoadedArtifactReport.DeclareNone();
	}

	//------------------------------------------------------------------------------------------------
	//! Called by TBD_BootArtifactVerification when a verification ends. Acted on in the next frame,
	//! once the verification's own call stack has unwound, since acting may replace it.
	static void OnVerificationFinished(int generation)
	{
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
			queue.CallLater(SettleVerification, 0, false, generation);
	}

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

		string documentMissionId = TBD_MissionLoader.GetMission().meta.id;
		if (!identity.mission_id.IsEmpty() && documentMissionId != identity.mission_id)
			TBD_Log.Warn(CH_MISSION, string.Format("the artifact's meta.id '%1' is not the deployment's mission '%2'", documentMissionId, identity.mission_id));

		TBD_LoadedArtifactReport.DeclareLoaded(identity.artifact_id, identity.artifact_sha256);
	}

	//------------------------------------------------------------------------------------------------
	protected static TBD_BootArtifactVerification NewVerification(notnull TBD_RuntimeDeploymentStruct identity)
	{
		s_Verification = new TBD_BootArtifactVerification();
		s_Verification.m_iGeneration = s_iGeneration;
		s_Verification.m_Identity = identity;
		return s_Verification;
	}

	//------------------------------------------------------------------------------------------------
	protected static void ScheduleRetry(int delayMs)
	{
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
			queue.CallLater(Retry, delayMs, false, s_iGeneration);
	}

	//------------------------------------------------------------------------------------------------
	protected static void Retry(int generation)
	{
		if (generation != s_iGeneration || s_bDecided)
			return;

		TBD_BackendConfig.Reload();
		ReadDeployment();
	}

	//------------------------------------------------------------------------------------------------
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
