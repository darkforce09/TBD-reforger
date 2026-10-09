/**
 * @file TBD_KitPreviewComponent.c
 * @brief The 3D doll in the kit preview card: a dressed character or a vehicle, turned and zoomed.
 *
 * Role: owns the `Preview` ItemPreviewWidget of TBD_KitPreview.layout; asks TBD_LoadoutPreviewDresser
 * for a preview entity wearing a kit, hands it to the ItemPreviewManagerEntity with the character's
 * SCR_CharacterInventoryPreviewAttributes (full-body framing), and turns the camera on a left drag
 * and zooms it on the wheel, as vanilla's SCR_InventoryCharacterWidgetHelper does.
 * Position: attached by TBD_KitInspectorPanel per preview card and by the briefing's uniform and
 * vehicle cards; input arrives through a workspace handler plus a 30 Hz drag poll, because the
 * vanilla Inventory_Inspect actions are live only inside the inventory context.
 * State: the widgets, manager, entity and attributes of the current doll, the hook and drag state
 * (client UI only), and a process-wide per-prefab zoom tracker.
 * Invariants: Destroy runs before the card is cleared and unhooks the handler and poll; the preview
 * entity stays with the manager; a missing prefab or dress failure captions PREVIEW UNAVAILABLE
 * and logs one WARNING; the tracked zoom stays within ZOOM_TRACK_MIN..ZOOM_TRACK_MAX.
 */

//! Preview driver for one kit preview card.
class TBD_KitPreviewComponent : ScriptedWidgetComponent
{
	static const int TICK_MS = 33; //!< ms between drag polls (30 Hz)
	static const float YAW_DEG_PER_PX = 0.6; //!< degrees of yaw per pointer pixel
	static const float PITCH_DEG_PER_PX = 0.3; //!< degrees of pitch per pointer pixel
	static const float ZOOM_PER_NOTCH = 4; //!< FOV degrees per wheel notch
	//! FOV delta applied on every Show (negative = closer): the character attributes frame a full
	//! body at about 75 % of the widget and -10 fills it. Tracked per prefab because the attributes
	//! object is shared and ZoomCamera is additive with no getter.
	static const float DEFAULT_ZOOM = -10; //!< FOV degrees relative to the prefab's framing
	static const float MIN_FOV = 15; //!< FOV floor, degrees
	static const float MAX_FOV = 90; //!< FOV ceiling, degrees
	static const float ZOOM_TRACK_MIN = -30; //!< lowest tracked zoom delta, degrees
	static const float ZOOM_TRACK_MAX = 60; //!< highest tracked zoom delta, degrees
	//! Vehicles carry no PreviewRenderAttributes; a script-created one lets the camera turn and zoom.
	static const bool SCRIPT_ATTRIBUTES_FOR_VEHICLES = true; //!< default true; false frames a vehicle with the manager's prefab path and no input
	protected static ref map<string, float> s_mZoomApplied; //!< prefab -> zoom delta applied to its shared attributes; null until the first zoom
	protected string m_sPrefabKey; //!< prefab of the current doll, the zoom tracker's key
	static const vector ROTATION_MIN = "-30 -180 0"; //!< camera rotation floor (pitch, yaw, roll), vanilla's limits
	static const vector ROTATION_MAX = "0 180 0"; //!< camera rotation ceiling (pitch, yaw, roll), vanilla's limits

	protected ItemPreviewWidget m_wPreview; //!< the layout's `Preview`; null after Destroy
	protected Widget m_wCaption; //!< the layout's `Label`: PREVIEW or PREVIEW UNAVAILABLE
	protected WorkspaceWidget m_wWorkspace; //!< the workspace the handler is added to while hooked
	protected ItemPreviewManagerEntity m_Manager; //!< the preview manager from TBD_LoadoutPreviewDresser
	protected IEntity m_Entity; //!< the entity on show; null without one
	protected PreviewRenderAttributes m_Attributes; //!< the camera attributes on show; null disables input
	protected ref PreviewRenderAttributes m_OwnedAttributes; //!< the script-created attributes for vehicles, kept alive here
	protected bool m_bHooked; //!< handler and poll are installed
	protected bool m_bDragging; //!< a left drag is held
	protected int m_iLastX; //!< pointer x at the last poll, px
	protected int m_iLastY; //!< pointer y at the last poll, px

	//! Create a driver for one preview card.
	//! @param previewWidget the layout's `Preview` (ItemPreviewWidget)
	//! @param caption the layout's `Label`
	//! @return the driver, or null when `previewWidget` is not an ItemPreviewWidget
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

	//! Dress and show `kit`; a null kit shows the plain PREVIEW caption.
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

	//! Show a character prefab wearing `loadout` (null = as shipped); the uniform cards pass a faction rifleman and no loadout.
	//! @param label names the doll in the one WARNING on failure
	void ShowPrefab(ResourceName prefab, TBD_SlotLoadoutStruct loadout, string label)
	{
		Unhook();
		m_Entity = null;
		m_Attributes = null;

		if (prefab.IsEmpty())
		{
			Print(string.Format("[TBD][lobby] kit preview: %1 unavailable -- prefab unresolved", label), LogLevel.WARNING);
			Fallback("PREVIEW UNAVAILABLE");
			return;
		}

		m_Manager = TBD_LoadoutPreviewDresser.GetOrSpawnManager();

		string why;
		m_Entity = TBD_LoadoutPreviewDresser.Dress(m_Manager, prefab, loadout, why);
		if (!m_Entity)
		{
			Print(string.Format("[TBD][lobby] kit preview: %1 unavailable -- %2", label, why), LogLevel.WARNING);
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

	//! Show a vehicle or item prefab as shipped (the assets page's Vehicle Info box): the item's own attributes, else a script-created one, else the manager's prefab path without input.
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
		// a script-created one; SCRIPT_ATTRIBUTES_FOR_VEHICLES switches that fallback off.
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

	//! Unhook and forget the widgets, manager and entity.
	void Destroy()
	{
		Unhook();
		m_wPreview = null;
		m_wCaption = null;
		m_Manager = null;
		m_Entity = null;
		m_Attributes = null;
	}


	//! Start a drag on a left press inside the preview.
	//! @return true when the press starts a drag
	override bool OnMouseButtonDown(Widget w, int x, int y, int button)
	{
		if (button != 0 || !m_Attributes || !Inside(x, y))
			return false;

		m_bDragging = true;
		WidgetManager.GetMousePos(m_iLastX, m_iLastY);
		return true;
	}

	//! End a drag on the left release.
	//! @return true when a drag ended
	override bool OnMouseButtonUp(Widget w, int x, int y, int button)
	{
		if (button != 0 || !m_bDragging)
			return false;

		m_bDragging = false;
		return true;
	}

	//! Zoom by ZOOM_PER_NOTCH per notch inside the preview.
	//! @return true when the wheel was consumed
	override bool OnMouseWheel(Widget w, int x, int y, int wheel)
	{
		if (!m_Attributes || !Inside(x, y))
			return false;

		ApplyZoom(-wheel * ZOOM_PER_NOTCH);
		Refresh();
		return true;
	}

	//! Drag poll: the pointer delta since the last poll turns the camera within ROTATION_MIN..ROTATION_MAX.
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


	//! Add `delta` to the FOV, clamped by the per-prefab tracker so the next Show can rebase.
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

	//! @return the zoom delta tracked for the current prefab; 0 when none
	protected float ZoomApplied()
	{
		float applied;
		if (s_mZoomApplied && s_mZoomApplied.Find(m_sPrefabKey, applied))
			return applied;

		return 0;
	}

	//! Re-submit the current entity and attributes to the manager.
	protected void Refresh()
	{
		if (m_Manager && m_wPreview && m_Entity)
			m_Manager.SetPreviewItem(m_wPreview, m_Entity, m_Attributes);
	}

	//! @return true when screen point (x, y) lies inside the preview widget
	protected bool Inside(int x, int y)
	{
		if (!m_wPreview)
			return false;

		float posX, posY, sizeX, sizeY;
		m_wPreview.GetScreenPos(posX, posY);
		m_wPreview.GetScreenSize(sizeX, sizeY);
		return x >= posX && x <= posX + sizeX && y >= posY && y <= posY + sizeY;
	}

	//! Hide the preview and show `caption`.
	protected void Fallback(string caption)
	{
		if (m_wPreview)
			m_wPreview.SetVisible(false);

		TBD_UITheme.Write(TextWidget.Cast(m_wCaption), caption);
		TBD_UITheme.Show(m_wCaption, true);
	}

	//! Add the workspace handler and start the drag poll; no-op when hooked or without a workspace.
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

	//! End any drag, remove the handler and stop the poll.
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
