**Status:** live

# Eden asset browser interactions

The Arma 3 Eden editor's asset browser, its right panel: the six modes and their F-keys, the side
submode, the search syntax and the vehicle crew switch. Each entry follows the Eden reference
format of the [feature entry schema](/documentation_v2/website/frontend/apps/editor/feature_inventory/feds_schema.md#eden-reference-entries);
the panel's layout is in the [UI anatomy](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/ui_anatomy.md#asset-browser--right-panel-assetbrowser).

## RIGHT — Asset Browser

#### RIGHT-MODE-001 — Object mode (F1)

| Field | Value |
|-------|-------|
| **Domain** | RIGHT |
| **UI Surface** | AssetBrowser |
| **Feature kind** | browser_mode |
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Asset_Browser#Mode |
| **Shortcut** | F1 |
| **Goal** | Browse/place objects (units, vehicles, props) |
| **Trigger** | Click Object mode tab or press F1 |
| **Preconditions** | Asset Browser visible |
| **Procedure** | 1. Mode filter = Object. 2. Submode shows faction/side filters. 3. List shows categorized assets. |
| **Postconditions** | Object assets available for placement |
| **Inputs** | F1, tab click |
| **Outputs** | Browser list content |
| **Edge cases** | Addon assets appear when mods loaded |
| **Acceptance** | `- [ ] F1 shows unit/vehicle/prop tree` |

#### RIGHT-MODE-002 — Composition mode (F2)

| Field | Value |
|-------|-------|
| **Domain** | RIGHT |
| **UI Surface** | AssetBrowser |
| **Feature kind** | browser_mode |
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Asset_Browser#Mode |
| **Shortcut** | F2 |
| **Goal** | Place predefined/custom group compositions |
| **Trigger** | F2 or Composition tab |
| **Procedure** | Mode = Composition; tree includes Compositions > Custom; composition toolbar active. |
| **Acceptance** | `- [ ] Custom compositions under Compositions > Custom` |

#### RIGHT-MODE-003 — Trigger mode (F3)

| Field | Value |
|-------|-------|
| **Domain** | RIGHT |
| **UI Surface** | AssetBrowser |
| **Feature kind** | browser_mode |
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Asset_Browser#Mode |
| **Shortcut** | F3 |
| **Goal** | Place trigger entities |
| **Procedure** | Mode = Trigger; no submode. |
| **Acceptance** | `- [ ] Trigger list; no submode` |

#### RIGHT-MODE-004 — Waypoint mode (F4)

| Field | Value |
|-------|-------|
| **Domain** | RIGHT |
| **UI Surface** | AssetBrowser |
| **Feature kind** | browser_mode |
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Asset_Browser#Mode |
| **Shortcut** | F4 |
| **Acceptance** | `- [ ] Waypoint assets listed` |

#### RIGHT-MODE-005 — System mode (F5)

| Field | Value |
|-------|-------|
| **Domain** | RIGHT |
| **UI Surface** | AssetBrowser |
| **Feature kind** | browser_mode |
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Asset_Browser#Mode |
| **Shortcut** | F5 |
| **Acceptance** | `- [ ] Systems modules listed` |

#### RIGHT-MODE-006 — Marker mode (F6)

| Field | Value |
|-------|-------|
| **Domain** | RIGHT |
| **UI Surface** | AssetBrowser |
| **Feature kind** | browser_mode |
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Asset_Browser#Mode |
| **Shortcut** | F6 |
| **Acceptance** | `- [ ] Marker icons in list` |

#### RIGHT-SUBMODE-001 — Asset submode (faction filter)

| Field | Value |
|-------|-------|
| **Domain** | RIGHT |
| **UI Surface** | AssetBrowser |
| **Feature kind** | browser_mode |
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Asset_Browser#Submode |
| **Shortcut** | Edit → Toggle Asset Sub-type; `SubmodeToggle` |
| **Goal** | Filter objects by side (BLUFOR, OPFOR, …) |
| **Procedure** | Unavailable for Trigger/Waypoint modes. |
| **Acceptance** | `- [ ] Faction tabs on Object mode` |

#### RIGHT-SEARCH-001 — Search by asset name

| Field | Value |
|-------|-------|
| **Domain** | RIGHT |
| **UI Surface** | AssetBrowser |
| **Feature kind** | interaction |
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Entity_Placing#Asset_Search |
| **Trigger** | Type in search field |
| **Procedure** | Persists across mode/submode switches. |
| **Acceptance** | `- [ ] Search survives mode change` |

#### RIGHT-SEARCH-002 — class: prefix search

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Asset_Browser#Search |
| **Trigger** | `class B_Soldier` |
| **Acceptance** | `- [ ] class prefix filters` |

#### RIGHT-SEARCH-003 — mod: prefix search

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Asset_Browser#Search |
| **Trigger** | `mod kart` or dropdown |
| **Acceptance** | `- [ ] mod prefix filters` |

#### RIGHT-SEARCH-004 — Wildcard search

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Asset_Browser#Search |
| **Trigger** | `house*ruin` (2.22+) |
| **Acceptance** | `- [ ] Wildcards work` |

#### RIGHT-SEARCH-005 — Regex search

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Asset_Browser#Search |
| **Trigger** | `/` prefix e.g. `/(Brick` |
| **Acceptance** | `- [ ] Regex filter works` |

#### RIGHT-CREW-001 — Vehicle crew toggle

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Entity_Placing#Placing_vehicles_with_crew |
| **Shortcut** | Alt inverts while placing |
| **Trigger** | Switch below browser |
| **Acceptance** | `- [ ] Manned/unmanned toggle` |
