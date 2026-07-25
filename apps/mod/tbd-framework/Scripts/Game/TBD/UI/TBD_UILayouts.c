//! T-181.7 — every `.layout` the UI framework owns, named once.
//!
//! Enfusion resources are addressed as `"{GUID}relative/path.layout"`. The GUID is the primary
//! key and lives in the file's `.meta`; the path is the fallback. Keeping both in one place means
//! that when Workbench next opens the project and rewrites `resourceDatabase.rdb` (the only thing
//! that can regenerate a GUID index), a changed GUID is a one-line edit here rather than a hunt
//! through screens.
//!
//! ── OPERATOR NOTE — one Workbench pass is required, and this is why ─────────────────────────
//! These `.layout`/`.meta` files were authored as text; Workbench cannot be driven from the
//! headless lane. The mod's committed `resourceDatabase.rdb` therefore does not list them.
//!
//! Measured, on the headless server:
//!   * **Scripts do not need an rdb entry.** A new `.c` absent from the rdb still compiled — and
//!     still reported its deliberate error. Script discovery is a directory scan.
//!   * **The addon does need an rdb to exist at all.** Delete `resourceDatabase.rdb` and the mod's
//!     script count drops from 5660 back to vanilla's 5633 — nothing in the addon loads.
//!   * **Non-script resources are not directory-scanned.** A new `Configs/System/chimeraMenus.conf`
//!     stayed invisible (`GUI (E): Menu preset '…' not found!`) at the vanilla path, at a custom
//!     path, with and without a `.meta`.
//!
//! So: the code here is complete, and the resources become live the first time the project is
//! opened in Workbench, which rewrites the index. `Create()` falls back to the bare path anyway,
//! which costs nothing and removes one class of first-run failure.
class TBD_UILayouts
{
	//! The chrome every TBD screen sits in: backdrop, header, content frame, one primary action.
	static const ResourceName SCREEN_SHELL = "{7BD1A70000000701}UI/layouts/TBD_ScreenShell.layout";

	//! One pooled row of a TBD_ListBox.
	static const ResourceName LIST_ROW     = "{7BD1A70000000702}UI/layouts/TBD_ListRow.layout";

	//------------------------------------------------------------------------------------------------
	//! Instantiate a layout under `parent`, retrying without the GUID prefix if the GUID does not
	//! resolve. Returns null on a dead workspace (server-side) or an unresolvable layout — every
	//! caller must handle null, because on a dedicated server there is no workspace at all.
	static Widget Create(ResourceName layout, Widget parent)
	{
		WorkspaceWidget workspace = GetGame().GetWorkspace();
		if (!workspace)
			return null; // headless / server — nothing to draw on

		if (layout.IsEmpty())
			return null;

		Widget created = workspace.CreateWidgets(layout, parent);
		if (created)
			return created;

		string bare = StripGuid(layout);
		if (bare == layout)
			return null; // no GUID prefix to strip; the layout is genuinely missing

		Print(string.Format("[TBD][ui] layout GUID did not resolve, retrying by path: %1", bare), LogLevel.WARNING);
		return workspace.CreateWidgets(bare, parent);
	}

	//------------------------------------------------------------------------------------------------
	//! `"{GUID}UI/x.layout"` -> `"UI/x.layout"`. Returns the input unchanged when there is no
	//! `{...}` prefix.
	static string StripGuid(string resource)
	{
		int close = resource.IndexOf("}");
		if (close < 0)
			return resource;

		return resource.Substring(close + 1, resource.Length() - close - 1);
	}
}
