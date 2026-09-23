//! The mission artifact files of this host, in `$profile:TBD_MissionArtifactCache/`:
//!   * `document.json` - the last verified artifact's bytes, exactly as the platform served them;
//!   * `identity.json` - the deployment they belong to (TBD_RuntimeDeploymentStruct): artifact id,
//!     its SHA-256, mission, event, event mission and terrain;
//!   * `received.json` - bytes just received and not verified yet, staged so their hash is taken
//!     from the file in one linear read (TBD_Sha256 explains why not from the string).
//!
//! The cache is written only after a verification succeeded, by the boot flow (TBD_DeployedMission)
//! and by a `load_mission` fleet command before it restarts the scenario (TBD_FleetLoadMissionAction).
//! A write removes the identity first and writes it last, so an interrupted write leaves no cached
//! artifact rather than a mismatched one. A read trusts nothing: the bytes are hashed again and
//! compared with the identity's SHA-256 before anything loads them (TBD_MissionArtifactVerification).
//! @authority server
class TBD_MissionArtifactCache
{
	protected static const string DIRECTORY = "$profile:TBD_MissionArtifactCache";
	protected static const string DOCUMENT_PATH = "$profile:TBD_MissionArtifactCache/document.json";
	protected static const string IDENTITY_PATH = "$profile:TBD_MissionArtifactCache/identity.json";
	protected static const string RECEIVED_PATH = "$profile:TBD_MissionArtifactCache/received.json";

	//------------------------------------------------------------------------------------------------
	//! Where the cache lives, for log lines.
	static string DescribeLocation()
	{
		return DIRECTORY;
	}

	//------------------------------------------------------------------------------------------------
	//! The identity of the cached artifact, or null when nothing usable is cached.
	static TBD_RuntimeDeploymentStruct ReadIdentity()
	{
		if (!FileIO.FileExists(IDENTITY_PATH) || !FileIO.FileExists(DOCUMENT_PATH))
			return null;

		JsonLoadContext context = new JsonLoadContext();
		if (!context.LoadFromFile(IDENTITY_PATH))
			return null;

		TBD_RuntimeDeploymentStruct identity = new TBD_RuntimeDeploymentStruct();
		if (!context.ReadValue("", identity))
			return null;

		if (identity.artifact_id.IsEmpty() || !TBD_Sha256.IsHexDigest(identity.artifact_sha256))
			return null;

		return identity;
	}

	//------------------------------------------------------------------------------------------------
	//! The cached artifact, as text for the parser and as bytes for the hash. False, with `failure`
	//! saying why, when it cannot be read whole.
	static bool ReadDocument(out string document, out array<int> bytes, out string failure)
	{
		document = string.Empty;
		bytes = null;
		if (!ReadText(DOCUMENT_PATH, document, failure))
			return false;

		return ReadBytes(DOCUMENT_PATH, bytes, failure);
	}

	//------------------------------------------------------------------------------------------------
	//! Stage `body`, bytes just received, in `received.json` and read them back as `bytes`: what the
	//! hash judges is exactly what is on disk. False, with `failure` saying why, when the file does not
	//! hold the body whole.
	static bool StageReceived(string body, out array<int> bytes, out string failure)
	{
		bytes = null;
		FileIO.MakeDirectory(DIRECTORY);
		if (!WriteExact(RECEIVED_PATH, body, failure))
			return false;

		return ReadBytes(RECEIVED_PATH, bytes, failure);
	}

	//------------------------------------------------------------------------------------------------
	//! Replace the cached artifact with the verified `document` of `identity`. False, with `failure`
	//! saying why, when it cannot be written whole; nothing is cached then.
	static bool Store(string document, notnull TBD_RuntimeDeploymentStruct identity, out string failure)
	{
		failure = string.Empty;
		FileIO.MakeDirectory(DIRECTORY);

		if (FileIO.FileExists(IDENTITY_PATH) && !FileIO.DeleteFile(IDENTITY_PATH))
		{
			failure = "cannot remove the previous " + IDENTITY_PATH;
			return false;
		}

		if (!WriteExact(DOCUMENT_PATH, document, failure))
			return false;

		if (FileIO.FileExists(RECEIVED_PATH))
			FileIO.DeleteFile(RECEIVED_PATH);

		if (!StoreIdentity(identity))
		{
			failure = "cannot write " + IDENTITY_PATH;
			return false;
		}

		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! Rewrite the identity of the cached artifact, keeping its bytes: a later read of the same
	//! artifact's deployment names its event and mission afresh.
	static bool StoreIdentity(notnull TBD_RuntimeDeploymentStruct identity)
	{
		JsonSaveContext context = new JsonSaveContext();
		if (!context.WriteValue("", identity))
			return false;

		return context.SaveToFile(IDENTITY_PATH);
	}

	//------------------------------------------------------------------------------------------------
	//! Write exactly the bytes of `data` to `path`, and check the file holds all of them.
	protected static bool WriteExact(string path, string data, out string failure)
	{
		failure = string.Empty;
		FileHandle handle = FileIO.OpenFile(path, FileMode.WRITE);
		if (!handle)
		{
			failure = "cannot open " + path + " for writing";
			return false;
		}

		int length = data.Length();
		handle.Write(data, length);
		handle.Close();

		int written = FileSize(path);
		if (written != length)
		{
			failure = string.Format("%1 holds %2 bytes after writing %3", path, written, length);
			return false;
		}

		return true;
	}

	//------------------------------------------------------------------------------------------------
	protected static bool ReadText(string path, out string text, out string failure)
	{
		text = string.Empty;
		failure = string.Empty;
		FileHandle handle = FileIO.OpenFile(path, FileMode.READ);
		if (!handle)
		{
			failure = "cannot open " + path;
			return false;
		}

		int byteCount = handle.GetLength();
		if (byteCount <= 0 || byteCount > TBD_MissionLoader.MISSION_FILE_MAX_BYTES)
		{
			handle.Close();
			failure = string.Format("%1 holds %2 bytes, outside 1..%3", path, byteCount, TBD_MissionLoader.MISSION_FILE_MAX_BYTES);
			return false;
		}

		handle.Read(text, byteCount);
		handle.Close();
		if (text.Length() != byteCount)
		{
			failure = string.Format("read %1 of the %2 bytes of %3", text.Length(), byteCount, path);
			text = string.Empty;
			return false;
		}

		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! The bytes of `path`, one per element, in one linear read.
	protected static bool ReadBytes(string path, out array<int> bytes, out string failure)
	{
		bytes = null;
		failure = string.Empty;
		FileHandle handle = FileIO.OpenFile(path, FileMode.READ);
		if (!handle)
		{
			failure = "cannot open " + path;
			return false;
		}

		int byteCount = handle.GetLength();
		if (byteCount <= 0 || byteCount > TBD_MissionLoader.MISSION_FILE_MAX_BYTES)
		{
			handle.Close();
			failure = string.Format("%1 holds %2 bytes, outside 1..%3", path, byteCount, TBD_MissionLoader.MISSION_FILE_MAX_BYTES);
			return false;
		}

		array<int> read = {};
		read.Resize(byteCount);
		int count = handle.ReadArray(read, 1, byteCount);
		handle.Close();
		if (count != byteCount)
		{
			failure = string.Format("read %1 of the %2 bytes of %3", count, byteCount, path);
			return false;
		}

		bytes = read;
		return true;
	}

	//------------------------------------------------------------------------------------------------
	protected static int FileSize(string path)
	{
		FileHandle handle = FileIO.OpenFile(path, FileMode.READ);
		if (!handle)
			return -1;

		int byteCount = handle.GetLength();
		handle.Close();
		return byteCount;
	}
}
