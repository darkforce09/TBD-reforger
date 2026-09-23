//! The FIPS 180-4 test vectors, run through TBD_Sha256 once per process before it judges an
//! artifact.
//!
//! An artifact loads only when its computed digest equals the published one, so a defective hasher
//! can refuse a good artifact but cannot accept a wrong one. The self-test tells those two cases
//! apart in the log: when it fails, a SHA-256 mismatch that follows is this engine build hashing
//! wrongly, not a damaged artifact. The vectors cover the empty message, one and two blocks, both
//! sides of the 55/56-byte padding boundary, an exact block, 1000 bytes, input absorbed in uneven
//! pieces, bytes above 127, and bytes that went through a file (`FileHandle.Write` of a string,
//! `ReadArray` back), the way received artifacts are hashed. Bytes above 127 are decoded from a JSON `\u00e9` escape, because the sources
//! are ASCII, and checked against UTF-8 or Latin-1, whichever the engine's JSON decoder produced.
//! @authority server

//! Holder of the decoded non-ASCII test text. The field name is the JSON key.
class TBD_Sha256SelfTestText
{
	string text;
}

class TBD_Sha256SelfTest
{
	//! Greppable channel: `grep '\[TBD\]\[Sha256\]' console.log`.
	protected static const string CH_SHA256 = "Sha256";

	protected static const string DIGEST_896_BIT = "cf5b16a778af8380036ce59e7b0492370b249b11e8f07a51afac45037afee9d1";
	protected static const string ROUND_TRIP_PATH = "$profile:TBD_Sha256SelfTest.bin";

	protected static bool s_bRan;
	protected static bool s_bPassed;
	protected static int s_iChecked;

	//------------------------------------------------------------------------------------------------
	//! Runs the vectors on the first call. True when every vector held.
	static bool Passed()
	{
		if (!s_bRan)
			Run();

		return s_bPassed;
	}

	//------------------------------------------------------------------------------------------------
	protected static void Run()
	{
		s_bRan = true;
		s_iChecked = 0;
		array<string> failed = {};

		string message448 = "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
		string message896 = "abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu";

		Check("empty", string.Empty, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", failed);
		Check("abc", "abc", "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad", failed);
		Check("448-bit", message448, "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1", failed);
		Check("896-bit", message896, DIGEST_896_BIT, failed);
		Check("55 x a", Repeat("a", 55), "9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318", failed);
		Check("56 x a", Repeat("a", 56), "b35439a4ac6f0948b6d6f9e3c6af0f5f590ce20f1bde7090ef7970686ec6738a", failed);
		Check("64 x a", Repeat("a", 64), "ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb", failed);
		Check("1000 x a", Repeat("a", 1000), "41edece42d63e8d9bf515a9ba6932e1c20cbc9f5a5d134645adb5db1b9737ea3", failed);
		CheckInPieces(message896, failed);
		string bytesAbove127 = CheckBytesAbove127(failed);
		CheckFileRoundTrip(message896, DIGEST_896_BIT, failed);

		s_bPassed = failed.IsEmpty();
		if (s_bPassed)
		{
			TBD_Log.Kv(CH_SHA256, "self-test-passed", string.Format("vectors=%1 bytesAbove127=%2", s_iChecked, bytesAbove127));
			return;
		}

		string names;
		foreach (string name : failed)
		{
			if (!names.IsEmpty())
				names += ", ";

			names += name;
		}

		TBD_Log.Error(CH_SHA256, string.Format("self-test FAILED for %1 of %2 vectors (%3) - this engine build computes SHA-256 wrongly, so every mission artifact it checks is refused as a mismatch",
			failed.Count(), s_iChecked, names));
	}

	//------------------------------------------------------------------------------------------------
	protected static void Check(string name, string message, string expected, notnull array<string> failed)
	{
		s_iChecked++;
		string computed = TBD_Sha256.Of(TBD_Sha256.BytesOf(message));
		if (computed == expected)
			return;

		failed.Insert(name);
		TBD_Log.Error(CH_SHA256, string.Format("vector '%1' (%2 bytes): computed %3, expected %4", name, message.Length(), computed, expected));
	}

	//------------------------------------------------------------------------------------------------
	//! The 896-bit message absorbed as 1, 7 and 63 bytes and the rest, across block boundaries.
	protected static void CheckInPieces(string message, notnull array<string> failed)
	{
		s_iChecked++;
		array<int> bytes = TBD_Sha256.BytesOf(message);
		TBD_Sha256 hash = new TBD_Sha256();
		int position = hash.Absorb(bytes, 0, 1);
		position = hash.Absorb(bytes, position, 7);
		position = hash.Absorb(bytes, position, 63);
		position = hash.Absorb(bytes, position, bytes.Count());

		string computed = hash.HexDigest();
		if (position == message.Length() && computed == DIGEST_896_BIT)
			return;

		failed.Insert("896-bit in pieces");
		TBD_Log.Error(CH_SHA256, string.Format("vector '896-bit in pieces': computed %1 after %2 of %3 bytes, expected %4",
			computed, position, message.Length(), DIGEST_896_BIT));
	}

	//------------------------------------------------------------------------------------------------
	//! "caf" and U+00E9, decoded by the engine: 5 bytes as UTF-8 or 4 as Latin-1. Returns which one
	//! was checked, or "untested" when the decoder produced neither.
	protected static string CheckBytesAbove127(notnull array<string> failed)
	{
		string text;
		JsonLoadContext context = new JsonLoadContext();
		TBD_Sha256SelfTestText holder = new TBD_Sha256SelfTestText();
		if (context.LoadFromString("{\"text\":\"caf\\u00e9\"}") && context.ReadValue("", holder))
			text = holder.text;

		if (text.Length() == 5)
		{
			Check("utf-8 cafe", text, "850f7dc43910ff890f8879c0ed26fe697c93a067ad93a7d50f466a7028a9bf4e", failed);
			return "utf8";
		}

		if (text.Length() == 4)
		{
			Check("latin-1 cafe", text, "dafd66c0b98965e688be1fc12942c09f0350e6be0685017c3f234e97d0adc92e", failed);
			return "latin1";
		}

		TBD_Log.Warn(CH_SHA256, string.Format("the JSON decoder turned 'caf\\u00e9' into %1 bytes, neither UTF-8 (5) nor Latin-1 (4) - bytes above 127 are untested", text.Length()));
		return "untested";
	}

	//------------------------------------------------------------------------------------------------
	//! `message` written to a file and read back as bytes hashes to `expected`.
	protected static void CheckFileRoundTrip(string message, string expected, notnull array<string> failed)
	{
		s_iChecked++;
		string computed = "not written";
		FileHandle writer = FileIO.OpenFile(ROUND_TRIP_PATH, FileMode.WRITE);
		if (writer)
		{
			writer.Write(message, message.Length());
			writer.Close();

			FileHandle reader = FileIO.OpenFile(ROUND_TRIP_PATH, FileMode.READ);
			if (reader)
			{
				int byteCount = reader.GetLength();
				array<int> bytes = {};
				bytes.Resize(byteCount);
				reader.ReadArray(bytes, 1, byteCount);
				reader.Close();
				computed = TBD_Sha256.Of(bytes);
			}

			FileIO.DeleteFile(ROUND_TRIP_PATH);
		}

		if (computed == expected)
			return;

		failed.Insert("file round trip");
		TBD_Log.Error(CH_SHA256, string.Format("vector 'file round trip' (%1 bytes written, read back with ReadArray): computed %2, expected %3", message.Length(), computed, expected));
	}

	//------------------------------------------------------------------------------------------------
	protected static string Repeat(string unit, int count)
	{
		string repeated;
		for (int i = 0; i < count; i++)
			repeated += unit;

		return repeated;
	}
}
