/**
 * @file TBD_BallisticsOracleOutputFile.c
 * @brief One ballistics oracle output file, written in pieces and hashed as it is written.
 *
 * Role: streams a JSON document to disk and keeps the SHA-256 and byte count of exactly what was
 * written, then writes the sidecar that carries them.  Position: opened by the forward-angle
 * writer and the simulation component; the sidecar is what the calibration bundle builder checks
 * the output against.
 * State: the open file handle, the pending text, the running hash and the byte count, owned by the
 * writer that opened it.  Invariants: text is ASCII, so one character is one byte and the digest
 * matches the file; any failed open or write marks the file failed, and a failed file gets no
 * sidecar.
 */

//! Streams one JSON output to disk, hashing every character written, and writes its sidecar.
class TBD_BallisticsOracleOutputFile
{
	protected static const int FLUSH_CHARACTERS = 8000; //!< pending text written once it reaches this many characters

	protected ref FileHandle m_Handle; //!< open output file; null before Open and after Close
	protected ref TBD_BallisticsOracleSha256 m_Hash; //!< digest of every character appended
	protected string m_sPath; //!< output path under $profile:
	protected string m_sPending; //!< appended text not yet written
	protected int m_iBytes; //!< characters appended, one byte each
	protected bool m_bFailed; //!< true once an open or a write failed

	//! Opens `path` for writing, replacing any file there; returns false and marks the file failed
	//! when it cannot be opened.
	bool Open(string path)
	{
		m_sPath = path;
		m_sPending = string.Empty;
		m_iBytes = 0;
		m_Hash = new TBD_BallisticsOracleSha256();
		m_Handle = FileIO.OpenFile(path, FileMode.WRITE);
		m_bFailed = !m_Handle;
		if (m_bFailed)
			Print("[TBD Ballistics Oracle] Cannot open " + path + " for writing", LogLevel.ERROR);

		return !m_bFailed;
	}

	//! Appends `text` to the document; ignored after a failure.
	void Append(string text)
	{
		if (m_bFailed || !m_Handle)
			return;

		m_Hash.AbsorbText(text);
		m_iBytes += text.Length();
		m_sPending += text;
		if (m_sPending.Length() >= FLUSH_CHARACTERS)
			Flush();
	}

	//! Writes the pending text and closes the file; returns false when any write failed.
	bool Close()
	{
		if (m_Handle)
		{
			Flush();
			m_Handle.Close();
			m_Handle = null;
		}

		return !m_bFailed;
	}

	//! Returns the SHA-256 of every character appended, as lowercase hex.
	string Sha256()
	{
		return m_Hash.HexDigest();
	}

	//! Returns the number of bytes appended.
	int Bytes()
	{
		return m_iBytes;
	}

	//! Returns true once an open or a write failed.
	bool Failed()
	{
		return m_bFailed;
	}

	//! Writes `<name>_meta.json` beside the closed output: the document type, the output file name,
	//! its byte count and SHA-256, the run header and the final status. Returns false, and writes
	//! nothing, when the output failed.
	bool WriteSidecar(string directory, string outputName, string documentType, TBD_BallisticsOracleRun run, string status)
	{
		if (m_bFailed)
			return false;

		string sidecarName = outputName;
		sidecarName.Replace(".json", "_meta.json");
		string json = "{\"document_type\":" + TBD_BallisticsOracleJson.Quote(documentType + "_meta");
		json += ",\"schema_version\":1," + run.HeaderMembers();
		json += ",\"finished_at\":" + TBD_BallisticsOracleJson.Quote(TBD_BallisticsOracleJson.IsoNowUtc());
		json += ",\"file\":" + TBD_BallisticsOracleJson.Quote(outputName);
		json += ",\"bytes\":" + TBD_BallisticsOracleJson.Integer(m_iBytes);
		json += ",\"sha256\":" + TBD_BallisticsOracleJson.Quote(Sha256());
		json += ",\"status\":" + TBD_BallisticsOracleJson.Quote(status) + "}\n";

		FileHandle sidecar = FileIO.OpenFile(directory + sidecarName, FileMode.WRITE);
		if (!sidecar)
		{
			Print("[TBD Ballistics Oracle] Cannot open " + directory + sidecarName + " for writing", LogLevel.ERROR);
			return false;
		}

		int written = sidecar.Write(json);
		sidecar.Close();
		return written > 0;
	}

	//! Writes the pending text; marks the file failed when the write reports no bytes.
	protected void Flush()
	{
		if (m_sPending.IsEmpty() || !m_Handle)
			return;

		int written = m_Handle.Write(m_sPending);
		m_sPending = string.Empty;
		if (written > 0)
			return;

		m_bFailed = true;
		Print("[TBD Ballistics Oracle] Write failed on " + m_sPath, LogLevel.ERROR);
	}
}
