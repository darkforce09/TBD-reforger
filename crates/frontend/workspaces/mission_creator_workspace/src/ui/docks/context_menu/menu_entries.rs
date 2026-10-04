//! Menu entries.

use super::*;

/// A stable identifier for a context menu command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ContextItem {
    /// `Go Here`: centre the camera on the selection, keeping the zoom.
    GoHere,
    /// `Play from Here`: always disabled; preview launch is a mod-side feature.
    PlayFromHere,
    /// The `Select` submenu: disabled, naming the issue that blocks it.
    Select,
    /// The `Edit` submenu: disabled; copy, paste and delete live on the toolbar and keyboard.
    Edit,
    /// The `Log` submenu: disabled; debug logging is not an editor feature.
    Log,

    /// Place a comment at the clicked point, in the active layer.
    PlaceComment,

    /// The `Connect` submenu: the connection kinds, or completing or cancelling the armed one.
    Connect,
    /// Arm a connection of this kind from the clicked entity.
    ConnectStart(ConnKind),
    /// Complete the armed connection on the clicked entity.
    ConnectComplete,
    /// Cancel the armed connection.
    ConnectCancel,
    /// `Connections...`: open the Connections panel.
    ShowConnections,
    /// `Play as the Character`: always disabled; preview launch is a mod-side feature.
    PlayAsCharacter,
    /// The `Transform` submenu: the formations.
    Transform,
    /// Lay the clicked leader's group out in this formation, as one undo step.
    MoveToFormation(FormationKind),
    /// The `Grid` submenu: disabled; object-dimension grid snapping is not an editor feature.
    Grid,
    /// `Save Custom Composition...`: disabled, naming the issue that blocks it.
    SaveComposition,
    /// `Find in Asset Browser...`: disabled; the Asset Browser cannot reveal an asset yet.
    FindInAssetBrowser,
    /// `Find in Config Viewer...`: disabled; the editor has no config viewer.
    FindInConfigViewer,
    /// Open the clicked entity's loadout in the Arsenal.
    EditLoadout,
    /// `Reset Loadout`: disabled; loadout reset is not a standalone editor action yet.
    ResetLoadout,
    /// The `Arrange` submenu: the top strip's arrange commands.
    Arrange,
    /// Run this arrange command on the selection.
    ArrangeRun(ArrangeKind),
    /// Open the clicked entity's Attributes dialog.
    Attributes,
}

impl ContextItem {
    /// Returns a blocking issue label for unavailable commands.
    #[must_use]
    pub const fn unblocked_by(self) -> Option<&'static str> {
        match self {
            ContextItem::PlaceComment => None,
            ContextItem::Connect | ContextItem::Transform => None,
            ContextItem::SaveComposition => Some("COMP-SAVE-001"),
            ContextItem::Select => Some("KEY-WP-001"),
            _ => None,
        }
    }

    /// Reports whether this item opens a submenu.
    #[must_use]
    pub const fn is_submenu_parent(self) -> bool {
        matches!(
            self,
            ContextItem::Connect | ContextItem::Transform | ContextItem::Arrange
        )
    }

    /// Builds the child rows offered by this submenu.
    #[must_use]
    pub fn submenu_entries(self, armed: Option<&(String, String)>) -> Vec<MenuEntry> {
        match self {
            ContextItem::Connect => match armed {
                None => ConnKind::ALL
                    .iter()
                    .map(|k| MenuEntry::on(ContextItem::ConnectStart(*k), k.label()))
                    .collect(),
                Some((kind, from)) => vec![
                    MenuEntry::on(ContextItem::ConnectComplete, "Complete Connection")
                        .with_note(format!("{kind} from {from}")),
                    MenuEntry::on(ContextItem::ConnectCancel, "Cancel Connection"),
                ],
            },
            ContextItem::Transform => FormationKind::ALL
                .iter()
                .map(|f| MenuEntry::on(ContextItem::MoveToFormation(*f), f.label()))
                .collect(),
            ContextItem::Arrange => ARRANGE
                .iter()
                .map(|e| {
                    MenuEntry::on(ContextItem::ArrangeRun(e.kind), e.label).with_shortcut(e.chord)
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Explains why this command is unavailable.
    #[must_use]
    pub const fn why(self) -> Option<&'static str> {
        Some(match self {
            ContextItem::PlayFromHere | ContextItem::PlayAsCharacter => {
                "Preview launch is a mod-side feature, not available in the editor"
            }
            ContextItem::Edit => {
                "Copy, paste and delete are on the toolbar and keyboard; this submenu is not built yet"
            }
            ContextItem::Log => "Debug logging to the clipboard is not an editor feature",
            ContextItem::Grid => "Object-dimension grid snapping is not an editor feature",
            ContextItem::FindInAssetBrowser => {
                "The Asset Browser has no reveal-and-scroll surface yet"
            }
            ContextItem::FindInConfigViewer => "The editor has no config viewer surface",
            ContextItem::ResetLoadout => "Loadout reset is not a standalone editor action yet",
            _ => return None,
        })
    }
}

/// The label, availability, and payload of one menu row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuEntry {
    /// The command the row runs; `None` for a heading.
    pub item: Option<ContextItem>,
    /// The row's label.
    pub label: &'static str,
    /// The keyboard chord shown beside the label; empty for none.
    pub shortcut: &'static str,
    /// Whether the row can be run.
    pub enabled: bool,
    /// The issue that blocks the row, when it is unavailable.
    pub blocked: Option<&'static str>,
    /// Whether the row opens a submenu.
    pub submenu: bool,
    /// Whether the row sits in a submenu.
    pub child: bool,
    /// A note shown under the label.
    pub note: Option<String>,
}

impl MenuEntry {
    /// Builds a separator row.
    pub(super) const fn sep() -> Self {
        Self {
            item: None,
            label: "",
            shortcut: "",
            enabled: false,
            blocked: None,
            submenu: false,
            child: false,
            note: None,
        }
    }

    /// Builds an enabled command row.
    pub(super) const fn on(item: ContextItem, label: &'static str) -> Self {
        Self {
            item: Some(item),
            label,
            shortcut: "",
            enabled: true,
            blocked: None,
            submenu: false,
            child: false,
            note: None,
        }
    }

    /// Builds a disabled command row with its reason.
    pub(super) const fn off(
        item: ContextItem,
        label: &'static str,
        blocked: Option<&'static str>,
    ) -> Self {
        Self {
            item: Some(item),
            label,
            shortcut: "",
            enabled: false,
            blocked,
            submenu: false,
            child: false,
            note: None,
        }
    }

    /// Builds a submenu parent row.
    pub(super) const fn parent(
        item: ContextItem,
        label: &'static str,
        blocked: Option<&'static str>,
    ) -> Self {
        Self {
            item: Some(item),
            label,
            shortcut: "",
            enabled: false,
            blocked,
            submenu: true,
            child: false,
            note: None,
        }
    }

    /// Adds the displayed keyboard shortcut to a row.
    pub(super) const fn with_shortcut(mut self, s: &'static str) -> Self {
        self.shortcut = s;
        self
    }

    /// Builds an enabled submenu parent row.
    pub(super) const fn open_parent(item: ContextItem, label: &'static str) -> Self {
        Self {
            item: Some(item),
            label,
            shortcut: "",
            enabled: true,
            blocked: None,
            submenu: true,
            child: false,
            note: None,
        }
    }

    /// Adds a context note to a row.
    pub(super) fn with_note(mut self, note: String) -> Self {
        self.note = Some(note);
        self
    }

    /// Marks a row as a submenu child.
    pub(super) const fn into_child(mut self) -> Self {
        self.child = true;
        self
    }
}
