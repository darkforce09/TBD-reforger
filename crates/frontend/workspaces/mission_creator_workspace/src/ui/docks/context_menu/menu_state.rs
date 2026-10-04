//! Menu state.

use super::*;

/// The row set selected for empty ground or an entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuTake {
    /// The click landed on empty ground.
    EmptyGround,
    /// The click landed on an entity.
    OnEntity,
}

impl MenuTake {
    /// Builds the rows in this menu take or state.
    #[must_use]
    pub fn entries(self) -> Vec<MenuEntry> {
        use ContextItem as I;
        match self {
            MenuTake::EmptyGround => vec![
                MenuEntry::on(I::GoHere, "Go Here"),
                MenuEntry::off(I::PlayFromHere, "Play from Here", None),
                MenuEntry::sep(),
                MenuEntry::parent(I::Select, "Select", I::Select.unblocked_by()),
                MenuEntry::parent(I::Edit, "Edit", None),
                MenuEntry::parent(I::Log, "Log", None),
                MenuEntry::sep(),
                MenuEntry::on(I::PlaceComment, "Place Comment"),
                MenuEntry::on(I::ShowConnections, "Connections..."),
            ],
            MenuTake::OnEntity => vec![
                MenuEntry::open_parent(I::Connect, "Connect"),
                MenuEntry::sep(),
                MenuEntry::on(I::GoHere, "Go Here"),
                MenuEntry::off(I::PlayAsCharacter, "Play as the Character", None),
                MenuEntry::sep(),
                MenuEntry::parent(I::Select, "Select", I::Select.unblocked_by()),
                MenuEntry::parent(I::Edit, "Edit", None),
                MenuEntry::open_parent(I::Transform, "Transform"),
                MenuEntry::parent(I::Grid, "Grid", None),
                MenuEntry::parent(I::Log, "Log", None),
                MenuEntry::sep(),
                MenuEntry::on(I::ShowConnections, "Connections..."),
                MenuEntry::sep(),
                MenuEntry::off(
                    I::SaveComposition,
                    "Save Custom Composition...",
                    I::SaveComposition.unblocked_by(),
                ),
                MenuEntry::off(I::FindInAssetBrowser, "Find in Asset Browser...", None),
                MenuEntry::off(I::FindInConfigViewer, "Find in Config Viewer...", None),
                MenuEntry::sep(),
                MenuEntry::on(I::EditLoadout, "Edit Loadout..."),
                MenuEntry::off(I::ResetLoadout, "Reset Loadout", None),
                MenuEntry::sep(),
                MenuEntry::on(I::Attributes, "Attributes..."),
            ],
        }
    }
}

/// The selection and world point targeted by a right click.
#[derive(Debug, Clone, PartialEq)]
pub struct MenuTarget {
    /// What the click landed on.
    pub take: MenuTake,
    /// The ids the menu acts on.
    pub target_ids: Vec<String>,
    /// The entity the selection is retargeted to, when the click landed outside it.
    pub retarget_to: Option<String>,
    /// The clicked world point, when it is on the map.
    pub world: Option<(f64, f64)>,
    /// The armed connection's kind and source, while one is armed.
    pub armed_connect: Option<(String, String)>,
}

impl MenuTarget {
    /// Sets the world point associated with this menu target.
    #[must_use]
    pub fn at_world(mut self, x: f64, z: f64) -> Self {
        self.world = Some((x, z));
        self
    }

    /// Records an active connection command on this target.
    #[must_use]
    pub fn with_armed_connect(mut self, armed: Option<(String, String)>) -> Self {
        self.armed_connect = armed;
        self
    }
}

/// Chooses the right click target from the hit and current selection.
#[must_use]
pub fn resolve_target(hit: Option<&str>, selection: &[String]) -> MenuTarget {
    match hit {
        None => MenuTarget {
            take: MenuTake::EmptyGround,
            target_ids: Vec::new(),
            retarget_to: None,
            world: None,
            armed_connect: None,
        },
        Some(id) => {
            if selection.iter().any(|s| s == id) {
                MenuTarget {
                    take: MenuTake::OnEntity,
                    target_ids: selection.to_vec(),
                    retarget_to: None,
                    world: None,
                    armed_connect: None,
                }
            } else {
                MenuTarget {
                    take: MenuTake::OnEntity,
                    target_ids: vec![id.to_string()],
                    retarget_to: Some(id.to_string()),
                    world: None,
                    armed_connect: None,
                }
            }
        }
    }
}

/// Location, target, and open submenu of the active menu.
#[derive(Debug, Clone, PartialEq)]
pub struct MenuState {
    /// The menu's left edge, in CSS pixels.
    pub x: f64,
    /// The menu's top edge, in CSS pixels.
    pub y: f64,
    /// What the menu acts on.
    pub target: MenuTarget,
    /// The open submenu, if any.
    pub open_submenu: Option<ContextItem>,
}

impl MenuState {
    fn base_entries(&self) -> Vec<MenuEntry> {
        let base = self.target.take.entries();
        if self.target.target_ids.len() < ARRANGE_MIN_SELECTION {
            return base;
        }
        let mut out = Vec::with_capacity(base.len() + 1);
        for entry in base {
            let after_transform = entry.item == Some(ContextItem::Transform);
            out.push(entry);
            if after_transform {
                out.push(MenuEntry::open_parent(ContextItem::Arrange, "Arrange"));
            }
        }
        out
    }

    /// Builds the rows in this menu take or state.
    #[must_use]
    pub fn entries(&self) -> Vec<MenuEntry> {
        let base = self.base_entries();
        let Some(open) = self.open_submenu else {
            return base;
        };
        let mut out = Vec::with_capacity(base.len() + 9);
        for entry in base {
            let is_open_parent = entry.item == Some(open);
            out.push(entry);
            if is_open_parent {
                out.extend(
                    open.submenu_entries(self.target.armed_connect.as_ref())
                        .into_iter()
                        .map(MenuEntry::into_child),
                );
            }
        }
        out
    }
}
