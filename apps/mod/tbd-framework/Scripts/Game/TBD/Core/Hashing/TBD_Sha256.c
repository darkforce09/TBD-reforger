//! SHA-256 (FIPS 180-4) over bytes held one per element of an `array<int>`, as lowercase hex. The
//! engine exposes no cryptographic hash to script, and a mission artifact is accepted only when the
//! SHA-256 of the exact bytes received equals the one the platform published for it.
//!
//! The bytes come in an array because every script call on a `string` copies the whole string
//! (measured: `ToAscii`, `Get` and `Substring` each cost about 7 us on 64 KiB and 110 us on 1 MiB,
//! wherever the index), which makes reading a large string byte by byte quadratic. Large inputs are
//! read from a file with `FileHandle.ReadArray` in one linear call (TBD_MissionArtifactCache);
//! `BytesOf` converts a short string.
//!
//! Script `int` is a signed 32-bit integer; the algorithm needs unsigned 32-bit words. Every
//! operation here is written so the signed representation carries the unsigned value:
//!   * `+` wraps modulo 2^32 and `<<` keeps the low 32 bits, as two's complement does;
//!   * `>>` sign-extends, so a logical right shift by n masks the result with 2^(32-n) - 1 (the
//!     LOW_n_BITS constants), and a right rotation by n is that shift OR-ed with `x << (32 - n)`;
//!   * the constants are written as the signed decimals of their 32-bit patterns.
//! The rotations are written out in `Compress` rather than called, and whole words are absorbed
//! four bytes at a time, because a script call per operation dominates the cost of the hash.
//! Every byte is masked to 0..255, whatever sign it arrives with. TBD_Sha256SelfTest checks every
//! one of these assumptions against FIPS test vectors before an artifact is ever judged.
//!
//! Input may arrive in pieces (`Absorb`); `HexDigest` pads and finishes. Messages up to 256 MiB
//! are supported, since the bit length is kept in one 31-bit word.
class TBD_Sha256
{
	protected static const string HEX_DIGITS = "0123456789abcdef";

	//! 2^n - 1: what survives an arithmetic right shift by 32 - n once the sign copies are masked off.
	protected static const int LOW_7_BITS = 127;
	protected static const int LOW_10_BITS = 1023;
	protected static const int LOW_13_BITS = 8191;
	protected static const int LOW_14_BITS = 16383;
	protected static const int LOW_15_BITS = 32767;
	protected static const int LOW_19_BITS = 524287;
	protected static const int LOW_21_BITS = 2097151;
	protected static const int LOW_22_BITS = 4194303;
	protected static const int LOW_25_BITS = 33554431;
	protected static const int LOW_26_BITS = 67108863;
	protected static const int LOW_29_BITS = 536870911;
	protected static const int LOW_30_BITS = 1073741823;

	//! The 64 round constants K (FIPS 180-4, 4.2.2).
	protected static ref array<int> s_aRoundConstants;

	//! The hash state H0..H7.
	protected int m_iH0;
	protected int m_iH1;
	protected int m_iH2;
	protected int m_iH3;
	protected int m_iH4;
	protected int m_iH5;
	protected int m_iH6;
	protected int m_iH7;

	//! The message schedule W; its first 16 words are the block being filled.
	protected ref array<int> m_aSchedule;
	//! Bytes of the word being assembled, and how many (0..3).
	protected int m_iWord;
	protected int m_iWordBytes;
	//! Words of the block being filled (0..15).
	protected int m_iBlockWords;
	//! Bytes absorbed so far.
	protected int m_iLength;
	protected string m_sDigest;

	//------------------------------------------------------------------------------------------------
	void TBD_Sha256()
	{
		m_aSchedule = new array<int>();
		m_aSchedule.Resize(64);
		Reset();
	}

	//------------------------------------------------------------------------------------------------
	//! Start a new message.
	void Reset()
	{
		// FIPS 180-4, 5.3.3.
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

	//------------------------------------------------------------------------------------------------
	//! The SHA-256 of `bytes`, in one call.
	static string Of(notnull array<int> bytes)
	{
		TBD_Sha256 hash = new TBD_Sha256();
		hash.Absorb(bytes, 0, bytes.Count());
		return hash.HexDigest();
	}

	//------------------------------------------------------------------------------------------------
	//! The bytes of a SHORT string, one per element. Quadratic in the string's length (see above),
	//! so large inputs are read from a file instead.
	static array<int> BytesOf(string text)
	{
		array<int> bytes = {};
		int length = text.Length();
		bytes.Resize(length);
		for (int i = 0; i < length; i++)
			bytes[i] = text.ToAscii(i) & 255;

		return bytes;
	}

	//------------------------------------------------------------------------------------------------
	//! True for 64 lowercase hex characters, the form of every digest here and on the platform.
	static bool IsHexDigest(string text)
	{
		if (text.Length() != 64)
			return false;

		for (int i = 0; i < 64; i++)
		{
			if (!HEX_DIGITS.Contains(text.Get(i)))
				return false;
		}

		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! Add up to `maxBytes` bytes of `bytes` from index `start` on. Returns the index after the last
	//! byte taken, which is where the next call continues.
	int Absorb(notnull array<int> bytes, int start, int maxBytes)
	{
		int end = bytes.Count();
		if (start + maxBytes < end)
			end = start + maxBytes;

		// A word already begun is finished byte by byte.
		int i = start;
		while (i < end && m_iWordBytes != 0)
		{
			AbsorbByte(bytes[i]);
			i++;
		}

		// Whole words go straight into the block, big-endian.
		array<int> w = m_aSchedule;
		while (i + 4 <= end)
		{
			w[m_iBlockWords] = ((bytes[i] & 255) << 24) | ((bytes[i + 1] & 255) << 16) | ((bytes[i + 2] & 255) << 8) | (bytes[i + 3] & 255);
			i += 4;
			m_iLength += 4;
			m_iBlockWords++;
			if (m_iBlockWords == 16)
			{
				Compress();
				m_iBlockWords = 0;
			}
		}

		while (i < end)
		{
			AbsorbByte(bytes[i]);
			i++;
		}

		return end;
	}

	//------------------------------------------------------------------------------------------------
	//! Pad, finish and return the digest as 64 lowercase hex characters. Repeated calls return the
	//! same digest; Reset starts over.
	string HexDigest()
	{
		if (!m_sDigest.IsEmpty())
			return m_sDigest;

		int lengthBits = m_iLength * 8;

		// FIPS 180-4, 5.1.1: one 1 bit, zeros up to 448 bits mod 512, then the 64-bit length,
		// whose high word is zero for every supported message.
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	//! Process the 16 words of the current block (FIPS 180-4, 6.2.2).
	protected void Compress()
	{
		array<int> w = m_aSchedule;
		for (int t = 16; t < 64; t++)
		{
			// sigma0 = ROTR7 ^ ROTR18 ^ SHR3, sigma1 = ROTR17 ^ ROTR19 ^ SHR10.
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
			// Sigma1 = ROTR6 ^ ROTR11 ^ ROTR25, Sigma0 = ROTR2 ^ ROTR13 ^ ROTR22; Ch and Maj in their
			// reduced forms g ^ (e & (f ^ g)) and (a & b) | (c & (a | b)).
			int bigSigma1 = (((e >> 6) & LOW_26_BITS) | (e << 26)) ^ (((e >> 11) & LOW_21_BITS) | (e << 21)) ^ (((e >> 25) & LOW_7_BITS) | (e << 7));
			int choice = g ^ (e & (f ^ g));
			int temp1 = h + bigSigma1 + choice + k[r] + w[r];
			int bigSigma0 = (((a >> 2) & LOW_30_BITS) | (a << 30)) ^ (((a >> 13) & LOW_19_BITS) | (a << 19)) ^ (((a >> 22) & LOW_10_BITS) | (a << 10));
			int majority = (a & b) | (c & (a | b));
			int temp2 = bigSigma0 + majority;
			h = g;
			g = f;
			f = e;
			e = d + temp1;
			d = c;
			c = b;
			b = a;
			a = temp1 + temp2;
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

	//------------------------------------------------------------------------------------------------
	//! Eight lowercase hex digits of a 32-bit word, most significant first.
	protected static string Hex(int word)
	{
		string digits;
		for (int shift = 28; shift >= 0; shift -= 4)
			digits += HEX_DIGITS.Get((word >> shift) & 15);

		return digits;
	}

	//------------------------------------------------------------------------------------------------
	protected static array<int> RoundConstants()
	{
		if (s_aRoundConstants)
			return s_aRoundConstants;

		// FIPS 180-4, 4.2.2, as signed decimals of the 32-bit patterns.
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
