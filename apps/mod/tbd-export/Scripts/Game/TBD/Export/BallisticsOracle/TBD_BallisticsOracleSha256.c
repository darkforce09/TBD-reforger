/**
 * @file TBD_BallisticsOracleSha256.c
 * @brief SHA-256 (FIPS 180-4) of ASCII text fed in pieces, as lowercase hex.
 *
 * Role: hashes the text of every ballistics oracle output while it is written, so each sidecar
 * carries the digest of the exact bytes on disk.  Position: fed by TBD_BallisticsOracleOutputFile;
 * SelfTest checks it against three FIPS 180-4 vectors before any output file is opened.
 * State: one hash state per instance (H0..H7, the schedule, the partial word and block, the length
 * and the finished digest).  Invariants: one byte per character, masked to 0..255, so only ASCII
 * text hashes to the bytes a file holds; messages up to 256 MiB, since the bit length is kept in
 * one 31-bit word.
 */

//! SHA-256 over ASCII text. The export addon loads without the framework addon, so it carries its
//! own hash. Script `int` is signed 32-bit: `+` and `<<` wrap as two's complement does, and every
//! logical right shift masks off the copied sign bits with a LOW_n_BITS constant.
class TBD_BallisticsOracleSha256
{
	protected static const string HEX_DIGITS = "0123456789abcdef"; //!< lowercase hex digits, indexed by nibble

	protected static const int LOW_7_BITS = 127; //!< 2^7 - 1
	protected static const int LOW_10_BITS = 1023; //!< 2^10 - 1
	protected static const int LOW_13_BITS = 8191; //!< 2^13 - 1
	protected static const int LOW_14_BITS = 16383; //!< 2^14 - 1
	protected static const int LOW_15_BITS = 32767; //!< 2^15 - 1
	protected static const int LOW_19_BITS = 524287; //!< 2^19 - 1
	protected static const int LOW_21_BITS = 2097151; //!< 2^21 - 1
	protected static const int LOW_22_BITS = 4194303; //!< 2^22 - 1
	protected static const int LOW_25_BITS = 33554431; //!< 2^25 - 1
	protected static const int LOW_26_BITS = 67108863; //!< 2^26 - 1
	protected static const int LOW_29_BITS = 536870911; //!< 2^29 - 1
	protected static const int LOW_30_BITS = 1073741823; //!< 2^30 - 1

	protected static ref array<int> s_aRoundConstants; //!< the 64 round constants K (FIPS 180-4, 4.2.2)

	protected int m_iH0; //!< hash state H0
	protected int m_iH1; //!< hash state H1
	protected int m_iH2; //!< hash state H2
	protected int m_iH3; //!< hash state H3
	protected int m_iH4; //!< hash state H4
	protected int m_iH5; //!< hash state H5
	protected int m_iH6; //!< hash state H6
	protected int m_iH7; //!< hash state H7

	protected ref array<int> m_aSchedule; //!< message schedule W; its first 16 words are the block being filled
	protected int m_iWord; //!< bytes of the word being assembled
	protected int m_iWordBytes; //!< bytes the word holds, 0..3
	protected int m_iBlockWords; //!< words of the block being filled, 0..15
	protected int m_iLength; //!< bytes absorbed so far
	protected string m_sDigest; //!< finished digest; empty until HexDigest

	//! Starts a hash at the FIPS 180-4 initial state.
	void TBD_BallisticsOracleSha256()
	{
		m_aSchedule = new array<int>();
		m_aSchedule.Resize(64);
		m_iH0 = 1779033703;
		m_iH1 = -1150833019;
		m_iH2 = 1013904242;
		m_iH3 = -1521486534;
		m_iH4 = 1359893119;
		m_iH5 = -1694144372;
		m_iH6 = 528734635;
		m_iH7 = 1541459225;
		m_iWord = 0;
		m_iWordBytes = 0;
		m_iBlockWords = 0;
		m_iLength = 0;
		m_sDigest = string.Empty;
	}

	//! Returns the SHA-256 of `text` as 64 lowercase hex characters.
	static string OfText(string text)
	{
		TBD_BallisticsOracleSha256 hash = new TBD_BallisticsOracleSha256();
		hash.AbsorbText(text);
		return hash.HexDigest();
	}

	//! Returns true when the empty message, "abc" and the 448-bit two-block message hash to their
	//! FIPS 180-4 digests; the oracle writes no output when it returns false.
	static bool SelfTest()
	{
		if (OfText(string.Empty) != "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
			return false;

		if (OfText("abc") != "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
			return false;

		string twoBlocks = "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
		return OfText(twoBlocks) == "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1";
	}

	//! Adds every character of `text` as one byte; ignored once the digest is finished.
	void AbsorbText(string text)
	{
		if (!m_sDigest.IsEmpty())
			return;

		int count = text.Length();
		for (int index = 0; index < count; index++)
			AbsorbByte(text.ToAscii(index));
	}

	//! Pads, finishes and returns the digest as 64 lowercase hex characters; later calls return
	//! the same digest.
	string HexDigest()
	{
		if (!m_sDigest.IsEmpty())
			return m_sDigest;

		int lengthBits = m_iLength * 8;
		AbsorbByte(128);
		while (m_iWordBytes != 0 || m_iBlockWords != 14)
			AbsorbByte(0);

		m_aSchedule[14] = 0;
		m_aSchedule[15] = lengthBits;
		Compress();
		m_iBlockWords = 0;

		m_sDigest = Hex(m_iH0) + Hex(m_iH1) + Hex(m_iH2) + Hex(m_iH3);
		m_sDigest += Hex(m_iH4) + Hex(m_iH5) + Hex(m_iH6) + Hex(m_iH7);
		return m_sDigest;
	}

	//! Adds one byte to the current word; a full word enters the block and a full block is
	//! compressed.
	protected void AbsorbByte(int value)
	{
		m_iWord = (m_iWord << 8) | (value & 255);
		m_iWordBytes++;
		m_iLength++;
		if (m_iWordBytes < 4)
			return;

		m_aSchedule[m_iBlockWords] = m_iWord;
		m_iWord = 0;
		m_iWordBytes = 0;
		m_iBlockWords++;
		if (m_iBlockWords < 16)
			return;

		Compress();
		m_iBlockWords = 0;
	}

	//! Processes the 16 words of the current block (FIPS 180-4, 6.2.2).
	protected void Compress()
	{
		array<int> w = m_aSchedule;
		for (int t = 16; t < 64; t++)
		{
			int x = w[t - 15];
			int y = w[t - 2];
			int s0 = (((x >> 7) & LOW_25_BITS) | (x << 25)) ^ (((x >> 18) & LOW_14_BITS) | (x << 14)) ^ ((x >> 3) & LOW_29_BITS);
			int s1 = (((y >> 17) & LOW_15_BITS) | (y << 15)) ^ (((y >> 19) & LOW_13_BITS) | (y << 13)) ^ ((y >> 10) & LOW_22_BITS);
			w[t] = w[t - 16] + s0 + w[t - 7] + s1;
		}

		array<int> k = RoundConstants();
		int a = m_iH0;
		int b = m_iH1;
		int c = m_iH2;
		int d = m_iH3;
		int e = m_iH4;
		int f = m_iH5;
		int g = m_iH6;
		int h = m_iH7;

		for (int r = 0; r < 64; r++)
		{
			int bigSigma1 = (((e >> 6) & LOW_26_BITS) | (e << 26)) ^ (((e >> 11) & LOW_21_BITS) | (e << 21)) ^ (((e >> 25) & LOW_7_BITS) | (e << 7));
			int choice = g ^ (e & (f ^ g));
			int temp1 = h + bigSigma1 + choice + k[r] + w[r];
			int bigSigma0 = (((a >> 2) & LOW_30_BITS) | (a << 30)) ^ (((a >> 13) & LOW_19_BITS) | (a << 19)) ^ (((a >> 22) & LOW_10_BITS) | (a << 10));
			int majority = (a & b) | (c & (a | b));
			h = g;
			g = f;
			f = e;
			e = d + temp1;
			d = c;
			c = b;
			b = a;
			a = temp1 + bigSigma0 + majority;
		}

		m_iH0 = m_iH0 + a;
		m_iH1 = m_iH1 + b;
		m_iH2 = m_iH2 + c;
		m_iH3 = m_iH3 + d;
		m_iH4 = m_iH4 + e;
		m_iH5 = m_iH5 + f;
		m_iH6 = m_iH6 + g;
		m_iH7 = m_iH7 + h;
	}

	//! Returns the eight lowercase hex digits of a 32-bit word, most significant first.
	protected static string Hex(int word)
	{
		string digits;
		for (int shift = 28; shift >= 0; shift -= 4)
			digits += HEX_DIGITS.Get((word >> shift) & 15);

		return digits;
	}

	//! Returns the 64 round constants K, built on first use, as signed decimals of the 32-bit
	//! patterns of FIPS 180-4, 4.2.2.
	protected static array<int> RoundConstants()
	{
		if (s_aRoundConstants)
			return s_aRoundConstants;

		s_aRoundConstants = {
			1116352408, 1899447441, -1245643825, -373957723, 961987163, 1508970993, -1841331548, -1424204075,
			-670586216, 310598401, 607225278, 1426881987, 1925078388, -2132889090, -1680079193, -1046744716,
			-459576895, -272742522, 264347078, 604807628, 770255983, 1249150122, 1555081692, 1996064986,
			-1740746414, -1473132947, -1341970488, -1084653625, -958395405, -710438585, 113926993, 338241895,
			666307205, 773529912, 1294757372, 1396182291, 1695183700, 1986661051, -2117940946, -1838011259,
			-1564481375, -1474664885, -1035236496, -949202525, -778901479, -694614492, -200395387, 275423344,
			430227734, 506948616, 659060556, 883997877, 958139571, 1322822218, 1537002063, 1747873779,
			1955562222, 2024104815, -2067236844, -1933114872, -1866530822, -1538233109, -1090935817, -965641998
		};
		return s_aRoundConstants;
	}
}
