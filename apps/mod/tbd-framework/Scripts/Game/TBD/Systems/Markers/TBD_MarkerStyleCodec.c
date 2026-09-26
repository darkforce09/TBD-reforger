/**
 * @file TBD_MarkerStyleCodec.c
 * @brief Packs the six marker style columns into the marker RPC's `xs` array and back.
 *
 * Role: encodes size, rotation, shape, brush, colour and alpha as a trailer of 6-int records on
 * `xs`.  Position: `PackIntoX` runs on the server after `TBD_MarkerService.BuildForPlayer`;
 * `UnpackFromX` runs in `TBD_MarkerClient.Accept`.
 * State: none.  Invariants: `Rpc()` takes at most eight parameters and the marker RPC uses all
 * eight, so style cannot be extra arguments. Record i is (size x100, rotation degrees x100, shape
 * index, brush index, 0xRRGGBB, alpha 0..255), -1 meaning absent; a set whose markers all use
 * the default style appends nothing, so `xs.Count() == zs.Count()` means unstyled.
 */

//! Style trailer codec.
class TBD_MarkerStyleCodec
{
	static const int COLS = 6; //!< ints per style record
	static const string HEX_DIGITS = "0123456789abcdef"; //!< lower-case hex digits by value

	//! @return `values[index]`, or -1 when the array is null or the index is out of range
	static int IntAt(array<int> values, int index)
	{
		if (!values || !values.IsIndexValid(index))
			return -1;

		return values[index];
	}

	//! @return `values[index]`, or empty when the array is null or the index is out of range
	static string StrAt(array<string> values, int index)
	{
		if (!values || !values.IsIndexValid(index))
			return string.Empty;

		return values[index];
	}

	//! @return 0 icon, 1 rectangle, 2 ellipse, 3 polyline; 0 for an empty or unknown shape
	static int ShapeIndex(string authored)
	{
		if (authored.IsEmpty())
			return 0;

		if (authored == "icon")
			return 0;

		if (authored == "rectangle")
			return 1;

		if (authored == "ellipse")
			return 2;

		if (authored == "polyline")
			return 3;

		return 0;
	}

	//! @return 0 solid, 1 border, 2 diagonal, 3 cross_diagonal, 4 horizontal, 5 vertical, 6 grid,
	//! 7 fill_diagonal; -1 for an empty or unknown brush
	static int BrushIndex(string authored)
	{
		if (authored.IsEmpty())
			return -1;

		if (authored == "solid")
			return 0;

		if (authored == "border")
			return 1;

		if (authored == "diagonal")
			return 2;

		if (authored == "cross_diagonal")
			return 3;

		if (authored == "horizontal")
			return 4;

		if (authored == "vertical")
			return 5;

		if (authored == "grid")
			return 6;

		if (authored == "fill_diagonal")
			return 7;

		return -1;
	}

	//! Parse `#rrggbb` (case-insensitive, surrounding spaces trimmed).
	//! @return 0xRRGGBB; -1 when empty (absent); -2 when malformed
	static int ParseHexRgb(string authored)
	{
		if (authored.IsEmpty())
			return -1;

		string hex = authored;
		hex.TrimInPlace();
		hex.ToLower();

		if (hex.Length() != 7)
			return -2;

		if (hex.Get(0) != "#")
			return -2;

		int value = 0;
		for (int i = 1; i < 7; i++)
		{
			int digit = HEX_DIGITS.IndexOf(hex.Get(i));
			if (digit < 0)
				return -2;

			value = (value * 16) + digit;
		}

		return value;
	}

	//! Append one 6-int record per marker to `wire.m_aX` when any marker carries a style; a
	//! malformed colour is sent as absent. Does nothing for a null or empty wire, or when every
	//! marker uses the default style.
	static void PackIntoX(TBD_MarkerWire wire)
	{
		if (!wire || !wire.m_aX || !wire.m_aZ)
			return;

		int n = wire.m_aZ.Count();
		if (n <= 0)
			return;

		array<int> extra = new array<int>();
		bool any = false;

		for (int i = 0; i < n; i++)
		{
			int sizeFp = IntAt(wire.m_aSizeFp, i);
			int rotationFp = IntAt(wire.m_aRotationFp, i);
			int shapeIdx = ShapeIndex(StrAt(wire.m_aShape, i));
			int brushIdx = BrushIndex(StrAt(wire.m_aBrush, i));
			int colorRgb = ParseHexRgb(StrAt(wire.m_aColorHex, i));
			if (colorRgb == -2)
				colorRgb = -1;

			int alpha255 = IntAt(wire.m_aAlpha255, i);

			extra.Insert(sizeFp);
			extra.Insert(rotationFp);
			extra.Insert(shapeIdx);
			extra.Insert(brushIdx);
			extra.Insert(colorRgb);
			extra.Insert(alpha255);

			if (sizeFp >= 0)
				any = true;
			if (rotationFp >= 0)
				any = true;
			if (shapeIdx != 0)
				any = true;
			if (brushIdx >= 0)
				any = true;
			if (colorRgb >= 0)
				any = true;
			if (alpha255 >= 0)
				any = true;
		}

		if (!any)
			return;

		foreach (int value : extra)
			wire.m_aX.Insert(value);
	}

	//! Fill the six style columns from the `xs` trailer. An unstyled or malformed `xs` leaves the
	//! outputs empty, so every row draws with the default style.
	//! @param xs world X per marker, then the trailer
	//! @param zs world Z per marker; its count is the marker count
	//! @return false when `xs` has neither the marker count nor the marker count plus one record
	//! per marker; true otherwise
	static bool UnpackFromX(array<int> xs, array<int> zs, array<int> sizeFp, array<int> rotationFp,
		array<int> shapeIdx, array<int> brushIdx, array<int> colorRgb, array<int> alpha255)
	{
		int n = 0;
		if (zs)
			n = zs.Count();

		if (!xs || n <= 0)
			return true;

		if (xs.Count() == n)
			return true;

		if (xs.Count() != n + (n * COLS))
			return false;

		int cursor = n;
		for (int i = 0; i < n; i++)
		{
			sizeFp.Insert(xs[cursor]);
			cursor++;
			rotationFp.Insert(xs[cursor]);
			cursor++;
			shapeIdx.Insert(xs[cursor]);
			cursor++;
			brushIdx.Insert(xs[cursor]);
			cursor++;
			colorRgb.Insert(xs[cursor]);
			cursor++;
			alpha255.Insert(xs[cursor]);
			cursor++;
		}

		return true;
	}
}
