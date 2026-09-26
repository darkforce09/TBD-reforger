/**
 * @file TBD_StyledMapMarker.c
 * @brief Placed custom marker that applies an authored size and opacity to its widget.
 *
 * Role: reapplies size and alpha after vanilla builds the marker widget.  Position: created by
 * `TBD_MarkerApplier.ApplyRows` only for a row whose size is not 100 or whose alpha is not 255;
 * every other row is a plain `SCR_MapMarkerBase`.  Colour is a palette entry set on the base class.
 * State: per-marker style values; one static warn-once flag.  Invariants: the size is applied one
 * frame after widget creation; a layout with no scalable image warns once and draws default size.
 */

//! Marker with authored size and opacity.
//! @authority client
class TBD_StyledMapMarker : SCR_MapMarkerBase
{
	int m_iTbdSizeFp = -1; //!< icon scale x100; -1 keeps the default size
	int m_iTbdAlpha255 = -1; //!< opacity 0..255; -1 keeps the default opacity

	protected static bool s_bSizeUnsupportedWarned; //!< true once the unscalable-layout warning was logged

	//! Build the vanilla widget, then set its opacity and schedule the resize for the next frame.
	//! @param skipProfanityFilter passed to the base class
	override void OnCreateMarker(bool skipProfanityFilter = false)
	{
		super.OnCreateMarker(skipProfanityFilter);

		if (!m_wRoot)
			return;

		if (m_iTbdAlpha255 >= 0)
			m_wRoot.SetOpacity(m_iTbdAlpha255 / 255.0);

		if (m_iTbdSizeFp >= 0 && m_iTbdSizeFp != 100)
			GetGame().GetCallqueue().CallLater(ApplyTbdSize, 0, false);
	}

	//! Scale every image widget under the root by the authored factor; warn once when none scaled.
	protected void ApplyTbdSize()
	{
		if (!m_wRoot)
			return;

		float factor = m_iTbdSizeFp / 100.0;
		int scaled = ScaleImagesUnder(m_wRoot, factor);

		if (scaled == 0 && !s_bSizeUnsupportedWarned)
		{
			s_bSizeUnsupportedWarned = true;
			TBD_Log.Warn(TBD_MarkerService.CH_MARKERS,
				"marker size could not be applied -- this build's marker layout has no scalable image; markers drew at default size.");
		}
	}

	//! Resize, around its centre, every sized image that sits in a frame slot under `parent`.
	//! @param parent the widget to walk, recursively
	//! @param factor the scale factor
	//! @return how many images were scaled
	protected int ScaleImagesUnder(Widget parent, float factor)
	{
		int scaled = 0;
		if (!parent)
			return 0;

		Widget child = parent.GetChildren();
		while (child)
		{
			ImageWidget image = ImageWidget.Cast(child);
			if (image && FrameWidget.Cast(parent))
			{
				float w;
				float h;
				image.GetScreenSize(w, h);

				if (w > 0 && h > 0)
				{
					WorkspaceWidget workspace = GetGame().GetWorkspace();
					float baseW = workspace.DPIUnscale(w);
					float baseH = workspace.DPIUnscale(h);

					FrameSlot.SetSize(image, baseW * factor, baseH * factor);
					FrameSlot.SetPos(image,
						FrameSlot.GetPosX(image) - ((baseW * factor) - baseW) / 2,
						FrameSlot.GetPosY(image) - ((baseH * factor) - baseH) / 2);

					scaled++;
				}
			}

			scaled = scaled + ScaleImagesUnder(child, factor);
			child = child.GetSibling();
		}

		return scaled;
	}
}
