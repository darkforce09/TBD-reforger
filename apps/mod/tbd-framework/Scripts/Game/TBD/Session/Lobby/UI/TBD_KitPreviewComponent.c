//! Kit-preview pass (2026-09-13) — the 3D doll in the KIT INSPECTOR's preview card.
//!
//! Owns the `Preview` render target of `TBD_KitPreview.layout`: asks `TBD_LoadoutPreviewDresser`
//! for the preview entity wearing the seat's exact kit, hands it to the `ItemPreviewManagerEntity`
//! with the character's own `SCR_CharacterInventoryPreviewAttributes` (full-body framing), and
//! turns / zooms that camera on input the way vanilla's inventory does
//! (`SCR_InventoryCharacterWidgetHelper`: `RotateItemCamera` limits "-30 -180 0" / "0 180 0",
//! `ZoomCamera`, then `SetPreviewItem` again). Input is read the TBD way — a workspace handler for
//! the button / wheel edges plus a 30 Hz `CallLater` poll of `WidgetManager.GetMousePos` while a
//! drag is held (the `TBD_UIScrollBar` recipe) — because the vanilla `Inventory_Inspect*` actions
//! are only live inside the inventory context.
//!
//! Lifecycle: `Attach()` once per mounted preview card, `Show(kit)` per seat, `Destroy()` before
//! the card is cleared. The preview entity itself stays with the manager (vanilla behaviour).
//! When anything is missing the caption says PREVIEW UNAVAILABLE and the reason is one WARNING.
class TBD_KitPreviewComponent : ScriptedWidgetComponent
{
	static const int TICK_MS = 33;
	static const float YAW_DEG_PER_PX = 0.6;
	static const float PITCH_DEG_PER_PX = 0.3;
	static const float ZOOM_PER_NOTCH = 4;
	//! FOV delta applied on every Show (negative = closer). The character attributes frame a full body
	//! at ~75 % of the widget (MEASURED run 1); -10 fills it. Tracked per prefab because the
	//! attributes object is shared and `ZoomCamera` is additive with no getter.
	static const float DEFAULT_ZOOM = -10;
	static const float MIN_FOV = 15;
	static const float MAX_FOV = 90;
	static const float ZOOM_TRACK_MIN = -30;
	static const float ZOOM_TRACK_MAX = 60;
	//! Vehicles carry no PreviewRenderAttributes; a script-created one lets the camera turn / zoom.
	static const bool SCRIPT_ATTRIBUTES_FOR_VEHICLES = true;
	protected static ref map<string, float> s_mZoomApplied;
	protected string m_sPrefabKey;
	static const vector ROTATION_MIN = "-30 -180 0";
	static const vector ROTATION_MAX = "0 180 0";

	protected ItemPreviewWidget m_wPreview;
	protected Widget m_wCaption;
	protected WorkspaceWidget m_wWorkspace;
	protected ItemPreviewManagerEntity m_Manager;
	protected IEntity m_Entity;
	protected PreviewRenderAttributes m_Attributes;
	protected ref PreviewRenderAttributes m_OwnedAttributes; //!< the script-created one for vehicles (kept alive here)
	protected bool m_bHooked;
	protected bool m_bDragging;
	protected int m_iLastX;
	protected int m_iLastY;

	//------------------------------------------------------------------------------------------------
	//! `previewWidget` is the layout's `Preview` (ItemPreviewWidget); `caption` its `Label`.
	static TBD_KitPreviewComponent Attach(Widget previewWidget, Widget caption)
	{
		ItemPreviewWidget preview = ItemPreviewWidget.Cast(previewWidget);
		if (!preview)
			return null;

		TBD_KitPreviewComponent component = new TBD_KitPreviewComponent();
		component.m_wPreview = preview;
		component.m_wCaption = caption;
		return component;
	}

	//------------------------------------------------------------------------------------------------
	//! Dress and show `kit`; null kit shows the plain PREVIEW caption.
	void Show(TBD_KitInfo kit)
	{
		if (!kit)
		{
			Unhook();
			m_Entity = null;
			m_Attributes = null;
			Fallback("PREVIEW");
			return;
		}

		ShowPrefab(kit.BasePrefab(), kit.m_Loadout, kit.m_sKey);
	}

	//------------------------------------------------------------------------------------------------
	//! A character prefab wearing `loadout` (null = as shipped): the uniform cards use this with
	//! a faction rifleman and no loadout. `label` names the doll in the one WARNING on failure.
	void ShowPrefab(ResourceName prefab, TBD_SlotLoadoutStruct loadout, string label)
	{
		Unhook();
		m_Entity = null;
		m_Attributes = null;

		if (prefab.IsEmpty())
		{
			Print(string.Format("[TBD][lobby] kit preview: %1 unavailable — prefab unresolved", label), LogLevel.WARNING);
			Fallback("PREVIEW UNAVAILABLE");
			return;
		}

		m_Manager = TBD_LoadoutPreviewDresser.GetOrSpawnManager();

		string why;
		m_Entity = TBD_LoadoutPreviewDresser.Dress(m_Manager, prefab, loadout, why);
		if (!m_Entity)
		{
			Print(string.Format("[TBD][lobby] kit preview: %1 unavailable — %2", label, why), LogLevel.WARNING);
			Fallback("PREVIEW UNAVAILABLE");
			return;
		}

		SCR_CharacterInventoryStorageComponent storage = SCR_CharacterInventoryStorageComponent.Cast(m_Entity.FindComponent(SCR_CharacterInventoryStorageComponent));
		if (storage)
		{
			ItemAttributeCollection collection = storage.GetAttributes();
			if (collection)
				m_Attributes = PreviewRenderAttributes.Cast(collection.FindAttribute(SCR_CharacterInventoryPreviewAttributes));
		}

		m_sPrefabKey = prefab;
		if (m_Attributes)
		{
			m_Attributes.ResetDeltaRotation();
			ApplyZoom(DEFAULT_ZOOM - ZoomApplied());
		}

		m_Manager.SetPreviewItem(m_wPreview, m_Entity, m_Attributes, true);
		m_wPreview.SetVisible(true);
		TBD_UITheme.Show(m_wCaption, false);
		Hook();
	}

	//------------------------------------------------------------------------------------------------
	//! A vehicle (or any item) prefab rendered as shipped — the assets page's Vehicle Info box.
	//! No dresser, no camera input: the manager frames it with the prefab's own attributes.
	void ShowVehicle(ResourceName prefab)
	{
		Unhook();
		m_Entity = null;
		m_Attributes = null;

		m_Manager = TBD_LoadoutPreviewDresser.GetOrSpawnManager();
		if (!m_Manager || prefab.IsEmpty() || !m_wPreview)
		{
			Fallback("PREVIEW UNAVAILABLE");
			return;
		}

		// Vanilla frames a prefab with the attributes on its InventoryItemComponent (vehicles have
		// none). To turn / zoom it we need an attributes object: the item's own when it has one, else
		// a script-created one (MEASURE: if the vehicle frames wrong, flip SCRIPT_ATTRIBUTES_FOR_VEHICLES).
		m_Entity = m_Manager.ResolvePreviewEntityForPrefab(prefab);
		if (m_Entity)
		{
			InventoryItemComponent item = InventoryItemComponent.Cast(m_Entity.FindComponent(InventoryItemComponent));
			if (item)
				m_Attributes = PreviewRenderAttributes.Cast(item.FindAttribute(PreviewRenderAttributes));

			if (!m_Attributes && SCRIPT_ATTRIBUTES_FOR_VEHICLES)
			{
				if (!m_OwnedAttributes)
					m_OwnedAttributes = new PreviewRenderAttributes();
				m_Attributes = m_OwnedAttributes;
			}
		}

		if (m_Entity && m_Attributes)
		{
			m_sPrefabKey = prefab;
			m_Attributes.ResetDeltaRotation();
			m_Manager.SetPreviewItem(m_wPreview, m_Entity, m_Attributes, true);
			Hook();
		}
		else
		{
			m_Manager.SetPreviewItemFromPrefab(m_wPreview, prefab);
		}

		m_wPreview.SetVisible(true);
		TBD_UITheme.Show(m_wCaption, false);
	}

	//------------------------------------------------------------------------------------------------
	void Destroy()
	{
		Unhook();
		m_wPreview = null;
		m_wCaption = null;
		m_Manager = null;
		m_Entity = null;
		m_Attributes = null;
	}

	// ── input ───────────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	override bool OnMouseButtonDown(Widget w, int x, int y, int button)
	{
		if (button != 0 || !m_Attributes || !Inside(x, y))
			return false;

		m_bDragging = true;
		WidgetManager.GetMousePos(m_iLastX, m_iLastY);
		return true;
	}

	//------------------------------------------------------------------------------------------------
	override bool OnMouseButtonUp(Widget w, int x, int y, int button)
	{
		if (button != 0 || !m_bDragging)
			return false;

		m_bDragging = false;
		return true;
	}

	//------------------------------------------------------------------------------------------------
	override bool OnMouseWheel(Widget w, int x, int y, int wheel)
	{
		if (!m_Attributes || !Inside(x, y))
			return false;

		ApplyZoom(-wheel * ZOOM_PER_NOTCH);
		Refresh();
		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! Drag poll: pointer delta since the last tick turns the camera.
	protected void Tick()
	{
		if (!m_bDragging || !m_Attributes)
			return;

		int mouseX, mouseY;
		WidgetManager.GetMousePos(mouseX, mouseY);
		int dx = mouseX - m_iLastX;
		int dy = mouseY - m_iLastY;
		m_iLastX = mouseX;
		m_iLastY = mouseY;
		if (dx == 0 && dy == 0)
			return;

		m_Attributes.RotateItemCamera(Vector(dy * PITCH_DEG_PER_PX, dx * YAW_DEG_PER_PX, 0), ROTATION_MIN, ROTATION_MAX);
		Refresh();
	}

	// ── helpers ─────────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! Additive FOV change, mirrored into the per-prefab tracker so the next Show can rebase.
	protected void ApplyZoom(float delta)
	{
		if (!m_Attributes || delta == 0)
			return;

		float applied = Math.Clamp(ZoomApplied() + delta, ZOOM_TRACK_MIN, ZOOM_TRACK_MAX);
		delta = applied - ZoomApplied();
		if (delta == 0)
			return;

		m_Attributes.ZoomCamera(delta, MIN_FOV, MAX_FOV);
		if (!s_mZoomApplied)
			s_mZoomApplied = new map<string, float>();

		s_mZoomApplied.Set(m_sPrefabKey, applied);
	}

	//------------------------------------------------------------------------------------------------
	protected float ZoomApplied()
	{
		float applied;
		if (s_mZoomApplied && s_mZoomApplied.Find(m_sPrefabKey, applied))
			return applied;

		return 0;
	}

	//------------------------------------------------------------------------------------------------
	protected void Refresh()
	{
		if (m_Manager && m_wPreview && m_Entity)
			m_Manager.SetPreviewItem(m_wPreview, m_Entity, m_Attributes);
	}

	//------------------------------------------------------------------------------------------------
	protected bool Inside(int x, int y)
	{
		if (!m_wPreview)
			return false;

		float posX, posY, sizeX, sizeY;
		m_wPreview.GetScreenPos(posX, posY);
		m_wPreview.GetScreenSize(sizeX, sizeY);
		return x >= posX && x <= posX + sizeX && y >= posY && y <= posY + sizeY;
	}

	//------------------------------------------------------------------------------------------------
	protected void Fallback(string caption)
	{
		if (m_wPreview)
			m_wPreview.SetVisible(false);

		TBD_UITheme.Write(TextWidget.Cast(m_wCaption), caption);
		TBD_UITheme.Show(m_wCaption, true);
	}

	//------------------------------------------------------------------------------------------------
	protected void Hook()
	{
		if (m_bHooked)
			return;

		m_wWorkspace = GetGame().GetWorkspace();
		if (!m_wWorkspace)
			return;

		m_wWorkspace.AddHandler(this);
		GetGame().GetCallqueue().CallLater(Tick, TICK_MS, true);
		m_bHooked = true;
	}

	//------------------------------------------------------------------------------------------------
	protected void Unhook()
	{
		m_bDragging = false;
		if (!m_bHooked)
			return;

		if (m_wWorkspace)
			m_wWorkspace.RemoveHandler(this);

		GetGame().GetCallqueue().Remove(Tick);
		m_wWorkspace = null;
		m_bHooked = false;
	}
}
