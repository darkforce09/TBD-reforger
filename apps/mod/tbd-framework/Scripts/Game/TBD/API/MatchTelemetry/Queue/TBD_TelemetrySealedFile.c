/**
 * @file TBD_TelemetrySealedFile.c
 * @brief A profile file whose record is sealed by a trailer carrying its byte length and SHA-256.
 *
 * Role: writes and reads the files of the telemetry queue so that a torn write is detected on
 * reload.  Position: used by `TBD_TelemetryQueueStorage` for every entry file and both state
 * slots; hashes through `TBD_Sha256`.
 * State: none; pure file operations.  Invariants: a file is `<record><trailer>`, the trailer being
 * exactly `TRAILER_BYTES` bytes, `\nTBDQ <length, 10 digits> <64 hex>\n`; `FileIO` has no rename,
 * so the record is written first, read back byte for byte and hashed, and only then is the trailer
 * appended: a write cut short anywhere leaves a file whose trailer is missing or does not match;
 * a read accepts a file only when its size, its length field and the hash of its record all
 * agree; bytes are read with one linear `ReadArray`, never byte by byte from a string.
 */

//! Sealed record files: write with a length and SHA-256 trailer, read back only when it matches.
//! @authority server
class TBD_TelemetrySealedFile
{
	static const int TRAILER_BYTES = 82; //!< `\n` + `TBDQ ` + 10 digits + ` ` + 64 hex + `\n`
	static const int RECORD_MAX_BYTES = 4194304; //!< largest record accepted, 4 MiB; bounds one read
	protected static const string TRAILER_TAG = "\nTBDQ "; //!< opening bytes of every trailer

	//! Write `record` to `path` and seal it.
	//! @param failure why the file is not sealed; empty on success
	//! @return true when the file holds `record` and a matching trailer
	static bool Write(string path, string record, out string failure)
	{
		failure = string.Empty;
		int length = record.Length();
		if (length <= 0 || length > RECORD_MAX_BYTES)
		{
			failure = string.Format("record of %1 bytes, outside 1..%2", length, RECORD_MAX_BYTES);
			return false;
		}

		if (!WriteWithMode(path, record, FileMode.WRITE, length, failure))
			return false;

		array<int> bytes;
		if (!ReadRecordBytes(path, length, bytes, failure))
			return false;

		string trailer = TRAILER_TAG + PadDecimal(length, 10) + " " + TBD_Sha256.Of(bytes) + "\n";
		return WriteWithMode(path, trailer, FileMode.APPEND, length + TRAILER_BYTES, failure);
	}

	//! The record of the sealed file at `path`.
	//! @param record the record without its trailer; empty on failure
	//! @param failure why the file is not a sealed record; empty on success
	//! @return true when the size, the trailer's length and its SHA-256 all match the record
	static bool Read(string path, out string record, out string failure)
	{
		record = string.Empty;
		failure = string.Empty;

		FileHandle handle = FileIO.OpenFile(path, FileMode.READ);
		if (!handle)
		{
			failure = "cannot open " + path;
			return false;
		}

		int fileBytes = handle.GetLength();
		int length = fileBytes - TRAILER_BYTES;
		if (length <= 0 || length > RECORD_MAX_BYTES)
		{
			handle.Close();
			failure = string.Format("%1 holds %2 bytes, too few or too many for a sealed record", path, fileBytes);
			return false;
		}

		string text;
		handle.Read(text, fileBytes);
		handle.Close();
		if (text.Length() != fileBytes)
		{
			failure = string.Format("read %1 of the %2 bytes of %3", text.Length(), fileBytes, path);
			return false;
		}

		string trailer = text.Substring(length, TRAILER_BYTES);
		if (!trailer.StartsWith(TRAILER_TAG) || !trailer.EndsWith("\n"))
		{
			failure = path + " has no trailer";
			return false;
		}

		int declared = trailer.Substring(TRAILER_TAG.Length(), 10).ToInt();
		string digest = trailer.Substring(TRAILER_TAG.Length() + 11, 64);
		if (declared != length || !TBD_Sha256.IsHexDigest(digest))
		{
			failure = string.Format("%1 trailer declares %2 bytes for a record of %3", path, declared, length);
			return false;
		}

		array<int> bytes;
		if (!ReadRecordBytes(path, length, bytes, failure))
			return false;

		if (TBD_Sha256.Of(bytes) != digest)
		{
			failure = path + " record does not match its SHA-256";
			return false;
		}

		record = text.Substring(0, length);
		return true;
	}

	//! Write `data` to `path` in `mode` and check the file then holds `expectedBytes` bytes.
	//! @return false, with `failure` saying why, when the file is not that size
	protected static bool WriteWithMode(string path, string data, FileMode mode, int expectedBytes, out string failure)
	{
		FileHandle handle = FileIO.OpenFile(path, mode);
		if (!handle)
		{
			failure = "cannot open " + path + " for writing";
			return false;
		}

		handle.Write(data, data.Length());
		handle.Close();

		int written = FileSize(path);
		if (written != expectedBytes)
		{
			failure = string.Format("%1 holds %2 bytes, expected %3", path, written, expectedBytes);
			return false;
		}

		return true;
	}

	//! The first `length` bytes of `path`, one per element, in one linear read.
	//! @return false, with `failure` saying why, when fewer arrive
	protected static bool ReadRecordBytes(string path, int length, out array<int> bytes, out string failure)
	{
		bytes = null;
		FileHandle handle = FileIO.OpenFile(path, FileMode.READ);
		if (!handle)
		{
			failure = "cannot open " + path;
			return false;
		}

		array<int> read = {};
		read.Resize(length);
		int count = handle.ReadArray(read, 1, length);
		handle.Close();
		if (count != length)
		{
			failure = string.Format("read %1 of the %2 record bytes of %3", count, length, path);
			return false;
		}

		bytes = read;
		return true;
	}

	//! The size of `path` in bytes.
	//! @return the size, or -1 when the file cannot be opened
	protected static int FileSize(string path)
	{
		FileHandle handle = FileIO.OpenFile(path, FileMode.READ);
		if (!handle)
			return -1;

		int byteCount = handle.GetLength();
		handle.Close();
		return byteCount;
	}

	//! `value` as decimal digits left-padded with zeros to `width`; `string.Format` has no width.
	//! @return the padded digits; a value wider than `width` is returned unpadded
	static string PadDecimal(int value, int width)
	{
		string digits = value.ToString();
		while (digits.Length() < width)
			digits = "0" + digits;

		return digits;
	}
}
