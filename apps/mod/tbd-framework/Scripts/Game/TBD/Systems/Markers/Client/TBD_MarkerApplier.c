/**
 * @file TBD_MarkerApplier.c
 * @brief Puts served marker rows on this client's map and takes them back off.
 *
 * Role: builds one placed custom marker per row with vanilla's placed-marker recipe (type
 * `PLACED_CUSTOM`, icon entry, colour entry, custom text) and inserts it local-only.
 * Position: called by `TBD_MarkerClient.Accept` and `TBD_MarkerClient.Shutdown`; draws through
 * `SCR_MapMarkerManagerComponent`; resolves icons through `TBD_MarkerIcons`.
 * State: the applied markers and the once-per-world area-fill note, static, on the client.
 * Invariants: every marker is inserted with `isLocal = true` and never gets a marker id, so it
 * never replicates and removal never asks the server; mission markers cannot be removed by the
 * player; a short parallel array stops the row walk instead of reading out of range.
 */

//! The markers this client has drawn.
//! @authority client
class TBD_MarkerApplier
{
	protected static ref array<ref SCR_MapMarkerBase> s_aApplied; //!< markers on the map; the manager holds a ref too
	protected static bool s_bAreaFillNoted; //!< true once the area-fill warning was logged; default false

	//! @return how many markers are on the map
	static int Count()
	{
		if (!s_aApplied)
			return 0;

		return s_aApplied.Count();
	}

	//! Let the area-fill warning log again (a new world or a new mission).
	static void RearmAreaFillNote()
	{
		s_bAreaFillNoted = false;
	}

	//! Build one marker per row and insert it on this client's map. Rows carry no ids; call
	//! `Clear` first for a full replace.
	//! @param xs world X per marker
	//! @param zs world Z per marker; its count is the row count
	//! @param icons authored icon per marker
	//! @param labels caption per marker
	//! @param sizeFp icon scale x100 per row, -1 absent; may be empty
	//! @param rotationFp rotation in degrees x100 per row, -1 absent; may be empty
	//! @param shapeIdx `TBD_MarkerStyleCodec.ShapeIndex` per row; may be empty
	//! @param brushIdx `TBD_MarkerStyleCodec.BrushIndex` per row; may be empty
	//! @param colorRgb 0xRRGGBB per row, -1 absent; may be empty
	//! @param alpha255 opacity 0..255 per row, -1 absent; may be empty
	//! @param unknownIcons set to how many rows named an icon `TBD_MarkerIcons` does not know
	//! @return false, with a warning and nothing drawn, when the machine has no marker manager
	static bool ApplyRows(array<int> xs, array<int> zs, array<string> icons,
		array<string> labels, array<int> sizeFp, array<int> rotationFp, array<int> shapeIdx,
		array<int> brushIdx, array<int> colorRgb, array<int> alpha255, out int unknownIcons)
	{
		unknownIcons = 0;
		SCR_MapMarkerManagerComponent mgr = TBD_MarkerClient.FindMarkerManager();
		if (!mgr)
		{
			// A warning, not an error: a machine without a marker manager is not a broken mission.
			TBD_Log.Warn(TBD_MarkerService.CH_MARKERS,
				string.Format("no SCR_MapMarkerManagerComponent on the game mode - %1 marker(s) cannot be drawn.",
					xs.Count()));
			return false;
		}

		EnsureApplied();

		for (int i = 0, count = zs.Count(); i < count; i++)
		{
			// The arrays are built and sent together; a short one stops the walk.
			if (!xs.IsIndexValid(i) || !icons.IsIndexValid(i) || !labels.IsIndexValid(i))
				break;

			bool recognised;
			int iconEntry = TBD_MarkerIcons.Resolve(icons[i], recognised);
			if (!recognised)
			{
				unknownIcons++;
				TBD_MarkerIcons.ReportUnknown(icons[i]);
			}

			iconEntry = TBD_MarkerIcons.ClampToLoadedConfig(iconEntry);

			int rowSizeFp = -1;
			int rowRotFp = -1;
			int rowShapeIdx = 0;
			int rowBrushIdx = -1;
			int rowRgb = -1;
			int rowAlpha255 = -1;
			if (sizeFp && sizeFp.IsIndexValid(i))
				rowSizeFp = sizeFp[i];
			if (rotationFp && rotationFp.IsIndexValid(i))
				rowRotFp = rotationFp[i];
			if (shapeIdx && shapeIdx.IsIndexValid(i))
				rowShapeIdx = shapeIdx[i];
			if (brushIdx && brushIdx.IsIndexValid(i))
				rowBrushIdx = brushIdx[i];
			if (colorRgb && colorRgb.IsIndexValid(i))
				rowRgb = colorRgb[i];
			if (alpha255 && alpha255.IsIndexValid(i))
				rowAlpha255 = alpha255[i];

			NoteAreaShape(rowShapeIdx, rowBrushIdx);

			bool scaleIcon = false;
			if (rowSizeFp >= 0 && rowSizeFp != 100)
				scaleIcon = true;

			bool fadeIcon = false;
			if (rowAlpha255 >= 0 && rowAlpha255 != 255)
				fadeIcon = true;

			SCR_MapMarkerBase marker;
			TBD_StyledMapMarker styled;
			if (scaleIcon || fadeIcon)
			{
				styled = new TBD_StyledMapMarker();
				if (scaleIcon)
					styled.m_iTbdSizeFp = rowSizeFp;
				if (fadeIcon)
					styled.m_iTbdAlpha255 = rowAlpha255;
				marker = styled;
			}
			else
			{
				marker = new SCR_MapMarkerBase();
			}

			marker.SetType(SCR_EMapMarkerType.PLACED_CUSTOM);
			marker.SetWorldPos(xs[i], zs[i]);
			marker.SetIconEntry(iconEntry);

			int colorEntry = TBD_MarkerIcons.MARKER_COLOR;
			if (rowRgb >= 0)
				colorEntry = NearestColorEntry(rowRgb);
			marker.SetColorEntry(colorEntry);

			if (rowRotFp >= 0)
				marker.SetRotation((rowRotFp + 50) / 100);

			marker.SetCustomText(labels[i]);

			// Briefing material: the vanilla remove action must not delete it.
			marker.SetCanBeRemovedByOwner(false);

			// isLocal: this client only; the marker never enters replication.
			mgr.InsertStaticMarker(marker, true);

			s_aApplied.Insert(marker);
		}

		return true;
	}

	//! Remove every applied marker from the map and forget it.
	static void Clear()
	{
		if (!s_aApplied || s_aApplied.IsEmpty())
		{
			EnsureApplied();
			return;
		}

		SCR_MapMarkerManagerComponent mgr = TBD_MarkerClient.FindMarkerManager();

		foreach (SCR_MapMarkerBase marker : s_aApplied)
		{
			if (!marker)
				continue;

			// A local marker keeps `GetMarkerID() == -1`: `RemoveStaticMarker` deletes its widget
			// and drops the manager's reference without asking the server.
			if (mgr)
				mgr.RemoveStaticMarker(marker);
		}

		s_aApplied.Clear();
	}

	//! Allocate the applied list on first use.
	protected static void EnsureApplied()
	{
		if (!s_aApplied)
			s_aApplied = new array<ref SCR_MapMarkerBase>();
	}

	//! Warn once per world or mission when a row carries an area shape: `SCR_MapMarkerBase` has no
	//! fill surface, so the icon is drawn with the authored colour, rotation, size and alpha.
	//! @param rowShapeIdx the row's shape index; 0 (icon) never warns
	//! @param rowBrushIdx the row's brush index, logged
	protected static void NoteAreaShape(int rowShapeIdx, int rowBrushIdx)
	{
		if (s_bAreaFillNoted)
			return;

		if (rowShapeIdx <= 0)
			return;

		s_bAreaFillNoted = true;

		TBD_Log.Warn(TBD_MarkerService.CH_MARKERS,
			string.Format("area marker shapeIdx=%1 brushIdx=%2 -- placed-marker widget cannot fill a shape; icon drawn with authored colour/rotation/size/alpha.",
				rowShapeIdx, rowBrushIdx));
	}

	//! Map an authored 0xRRGGBB onto the nearest entry of the placed-marker colour palette by
	//! squared RGB distance; `SetColorEntry` takes the palette index.
	//! @param rgb 0xRRGGBB, or negative when absent
	//! @return the palette index, or `TBD_MarkerIcons.MARKER_COLOR` for an absent colour, a missing
	//! manager or config, or an empty palette
	protected static int NearestColorEntry(int rgb)
	{
		if (rgb < 0)
			return TBD_MarkerIcons.MARKER_COLOR;

		int wantR = rgb / 65536;
		int wantG = (rgb / 256) % 256;
		int wantB = rgb % 256;

		SCR_MapMarkerManagerComponent mgr = TBD_MarkerClient.FindMarkerManager();
		if (!mgr)
			return TBD_MarkerIcons.MARKER_COLOR;

		SCR_MapMarkerConfig cfg = mgr.GetMarkerConfig();
		if (!cfg)
			return TBD_MarkerIcons.MARKER_COLOR;

		SCR_MapMarkerEntryPlaced placed = SCR_MapMarkerEntryPlaced.Cast(
			cfg.GetMarkerEntryConfigByType(SCR_EMapMarkerType.PLACED_CUSTOM));
		if (!placed)
			return TBD_MarkerIcons.MARKER_COLOR;

		array<ref SCR_MarkerColorEntry> entries = placed.GetColorEntries();
		if (!entries || entries.IsEmpty())
			return TBD_MarkerIcons.MARKER_COLOR;

		int bestIndex = TBD_MarkerIcons.MARKER_COLOR;
		int bestDist = 2147483647;

		for (int i = 0, n = entries.Count(); i < n; i++)
		{
			SCR_MarkerColorEntry entry = entries[i];
			if (!entry)
				continue;

			Color palette = entry.GetColor();
			if (!palette)
				continue;

			int packed = palette.PackToInt();
			int pr = (packed / 65536) % 256;
			int pg = (packed / 256) % 256;
			int pb = packed % 256;

			int dr = pr - wantR;
			int dg = pg - wantG;
			int db = pb - wantB;
			int dist = (dr * dr) + (dg * dg) + (db * db);
			if (dist < bestDist)
			{
				bestDist = dist;
				bestIndex = i;
			}
		}

		return bestIndex;
	}
}
