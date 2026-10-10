# Eden_Editor:_Composition

Source: https://community.bohemia.net/wiki/Eden_Editor:_Composition

---

Apart from pre-defined groups and compositions, you can save and share your own custom compositions.

The following things will be saved with the composition: 

- Attribute values (inventory, simulation, simple object, etc.)

- Layer structure (i.e. one can place comments in their own sub-layer for easy removal)

- Layer visibility and transformation

- Connections

Since  2.06, custom Eden Editor compositions are also available in Zeus.

### Saving Compositions

To save a composition, select the entities in the scene, click right mouse button on one of them and pick the Save Custom Composition option (or use the  button in the Asset Browser).

A window will be opened where you can set a title, author and category for your composition.

In the list on the left, you can either choose to create a new composition, or overwite one of the existing ones. This way, you can edit already created compositions.
To edit a composition's attributes, double-click on it, or select it and click on the Edit button (or use the  button in the Asset Browser). It will open a window where you can change the title, author and category again.

In order to delete a composition permanently, select it in list and then use the  button.

### Placing Compositions

You can find compositions in Compositions > Custom.
Place it just like any other entity - either select it and then click in the scene on desired position, or drag it from the list to the scene.

⚠Vertical mode defines how the composition will be placed.
-  Terrain Level - composition will be snapped to the surface underneath. Please note that objects which are placed on another objects may no longer be positioned precisely.

-  Sea Level - the composition will be placed exactly as it was saved, ignoring surface slope. This option may work better for complex compositions, especially when manually lowering the entities after placing.

If you need to correct the position or rotation of a placed composition it is recommended to use the Translation Widget and Rotation Widget.
This way, relative position and orientation of all entities in the composition will be preserved.

If you do not want each composition to be placed in its own layer turn off Automatic Composition Layering which can be found in Settings... -> Preferences... -> Misc.

### Publishing Compositions

You can also publish your compositions to Steam Workshop. Do so by selecting the saved composition in the Asset Browser, and then using the  button there.

A publishing window will be opened where you can enter a name, description, visibility setting, image, and various tags. After agreeing with the Workshop license, you are free to publish the composition and share the link with others.

#### Creating a Screenshot

In order for your composition to be found easier in the Steam Workshop you should add a meaningful screenshot. It is very easy to do so:

- Place your composition

- Move the Eden Editor camera so that only the composition is visible

- Press the Backspace button to hide the Eden Editor interface

- Press F12 (or the key you have assigned for screenshots in your steam application)

- Select your screenshot in the publish composition UI by pressing the Browse... button, navigate to Steam screenshots and select the previously taken screenshot

### Republishing Compositions

Updating a composition that was already published is effectively the same as the first time. Select the composition and use .
In the publishing window, select your already published composition on the left. Consider entering relevant change notes and publish.

### Unpublishing Compositions

In order to unpublish a composition from within Eden Editor, open the publishing window by selecting any saved composition and using the  button.
Now select the published composition in the list on the left and press DELETE.

### Subscribing to Compositions

You can easily find all published compositions made by yourself and others by selecting the  Steam subscribed content entry in the Asset Browser, and then pressing the  button.
This will open a Steam overlay with the Workshop opened and compositions filtered.
Soon after subscribing to any number of composition items there, the game should download these in the background. Large compositions may take a brief while to download and appear.
They will then be listed in the Asset Browser, ready for regular placement. Note that certain compositions require placement using a specific vertical mode to work correctly (see above).

### Unsubscribing from Compositions

To unsubscribe from a Workshop composition from within Eden Editor, select it in the Asset Browser and press the  button.
Note that you should look for compositions listed under  Steam subscribed content, otherwise you may accidentally delete a local composition instead.

### Local Files

Compositions are saved in the Compositions folder in your Profile directory. 

They can be shared freely - if you pack a composition folder into a *.zip file and make it available for download, people who place the unpacked composition to their Compositions folder will have it available as well.

 2.10

### Modding Compositions

It is possible for mods to add their own compositions.
The mod needs to register them in config (see the Example below) for them to be loaded and displayed in groups tabs in Eden and Zeus.

ⓘInit scripts and all other composition features are supported.
Modded Compositions can be used as replacement for CfgGroups and the arsenal can be used in compositions, thus removing the need to make custom hidden unit classes with custom loadouts in order to construct a group.
Note that placement-wise, compositions are slightly less efficient than using CfgGroups.

#### Example

class CfgEditorCategories
{
	class EdCat_NATO // CfgGroups NATO
	{
		displayName = "$STR_A3_CfgGroups_West_BLU_F0";
	};
};

class CfgEditorSubcategories
{
	class EdSubcat_Armored // CfgGroups Armored
	{
		displayName = "$STR_A3_CfgGroups_West_BLU_F_Armored0";
	};
};

class Cfg3DEN
{
	class Compositions
	{
		class ModTag_MyComposition1 // one class per composition
		{
			path = "edenCompositionTestmod\compositionTank";	// pbo path to a folder containing header.sqe/composition.sqe files
			side = 0;											// 0 opfor, 1 blufor, 2 indfor, 3 civ, 8 Empty/Props
			editorCategory = "EdCat_NATO";						// link to CfgEditorCategories
			editorSubcategory = "EdSubcat_Armored";				// link to CfgEditorSubcategories
			displayName = "$STR_Composition_Armored01";
			icon = "\A3\ui_f\data\map\markers\nato\b_inf.paa";	// left side icon in groups list
			useSideColorOnIcon = 1;								// 1 == icon is always colored in faction color
		};
	};
};

### Automating Composition Creation

It's possible to automate the creation of compositions. This is extremly helpful for maintaining a large number of compositions.

⚠Always make a backup of your composition folder before running this script. It will overwrite existing compositions!

0 spawn
{
	private _compositionLayers = [];

	// Get all layers that follow a naming scheme
	all3DENEntities # 6 apply
	{
		private _name = _x get3DENAttribute "name" select 0;
		if ("SPE_Comp_" in _name) then
		{
			_compositionLayers pushBack [_name, _x];
		};
	};

	// Loop though all layers
	{
		_x params ["_name", "_ID"];

		// Get the entities
		private _entities = get3DENLayerEntities _ID;

		// Select the entities so do3DENAction works
		set3DENSelected _entities;

		// Open the create composition dialog
		do3DENAction "CreateCustomComposition";

		// Make sure it's open
		waitUntil { !(isNull findDisplay 317) };

		// Set the layer name as composition name in the dialog
		findDisplay 317 displayCtrl 95 ctrlSetText _name;

		// Close the dialog by activating the OK button
		ctrlActivate (findDisplay 317 displayCtrl 1);

		// Add a small delay
		sleep 0.1;
	} forEach _compositionLayers;
};

Retrieved from "https://community.bistudio.com/wiki?title=Eden_Editor:_Custom_Composition&oldid=368928"
					Category: - Eden Editor: Asset Types