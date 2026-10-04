//! The vehicle form's text, the target it writes to, and the validation that turns the text into a
//! write body.
//!
//! **Role:** holds what an administrator typed into the vehicle form ([`VehicleDraft`]), seeds it
//! from the form's [`FormTarget`] (a new vehicle, or a stored row), and checks it under the
//! backend's field rules into a [`VehicleWrite`] or the [`FieldProblem`]s that stop it.
//! **Position:** read by the form dialog on submit; the checked body feeds the create or replace
//! request of [`super::vehicle_writes`] that the target picks.
//! **Signals & state:** none; pure functions over the draft.
//! **Invariants:**
//! - The rules are the backend validator's
//!   (`crates/api/api_community_content/src/handlers/vehicle_database/validation.rs`): every
//!   value is trimmed of whitespace and the byte order mark before its rule applies, and a limit
//!   counts characters. `name` (at most 120), `faction` (at most 60) and `armor_type` (at most 60)
//!   are required and never blank; `amphibious` (at most 60) and `primary_threat` (at most 120) are
//!   optional; `profile_image_url` is empty or a safe image source
//!   ([`frontend_ui::safe_url::safe_image_src`]).
//! - A draft that passes here is one the backend accepts field for field, so a refusal after it is
//!   about the session, the role or the request, never a field rule the form did not show.
//! - The write body carries the trimmed values, so the stored row equals what the form sent.

#[cfg(any(target_arch = "wasm32", test))]
use super::vehicle_writes::VehicleRequest;
#[cfg(any(target_arch = "wasm32", test))]
use frontend_api_dtos::vehicles::{Vehicle, VehicleWrite};
#[cfg(any(target_arch = "wasm32", test))]
use frontend_ui::safe_url::safe_image_src;

/// One field of the vehicle form, in the order the form shows them.
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum VehicleField {
    /// The vehicle's display name.
    Name,
    /// The side the vehicle belongs to.
    Faction,
    /// The vehicle's armour class.
    ArmorType,
    /// Whether and how the vehicle crosses water.
    Amphibious,
    /// The weapon that most endangers the vehicle.
    PrimaryThreat,
    /// The vehicle's picture.
    ProfileImageUrl,
}

#[cfg(any(target_arch = "wasm32", test))]
impl VehicleField {
    /// Every field, in the order the form shows them.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(super) const ALL: [VehicleField; 6] = [
        VehicleField::Name,
        VehicleField::Faction,
        VehicleField::ArmorType,
        VehicleField::Amphibious,
        VehicleField::PrimaryThreat,
        VehicleField::ProfileImageUrl,
    ];

    /// The field's label on the form, which also opens its problem sentences.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(super) fn label(self) -> &'static str {
        match self {
            VehicleField::Name => "Name",
            VehicleField::Faction => "Faction",
            VehicleField::ArmorType => "Armor type",
            VehicleField::Amphibious => "Amphibious",
            VehicleField::PrimaryThreat => "Primary threat",
            VehicleField::ProfileImageUrl => "Profile image URL",
        }
    }

    /// An example value shown in the empty input.
    #[cfg(target_arch = "wasm32")]
    pub(super) fn placeholder(self) -> &'static str {
        match self {
            VehicleField::Name => "BTR-70",
            VehicleField::Faction => "USSR",
            VehicleField::ArmorType => "Light Armour",
            VehicleField::Amphibious => "Yes",
            VehicleField::PrimaryThreat => "Autocannon — 14.5 mm KPVT",
            VehicleField::ProfileImageUrl => "https://… or /uploads/…",
        }
    }

    /// The most characters the trimmed value may hold; `None` for the image URL, which the URL
    /// policy bounds instead.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(super) fn max_chars(self) -> Option<usize> {
        match self {
            VehicleField::Name | VehicleField::PrimaryThreat => Some(120),
            VehicleField::Faction | VehicleField::ArmorType | VehicleField::Amphibious => Some(60),
            VehicleField::ProfileImageUrl => None,
        }
    }

    /// Whether the field must hold text.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(super) fn is_required(self) -> bool {
        matches!(
            self,
            VehicleField::Name | VehicleField::Faction | VehicleField::ArmorType
        )
    }
}

/// Why one field stops the form, worded for the administrator.
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FieldProblem {
    /// The field the sentence is about.
    pub field: VehicleField,
    /// The sentence shown under the field.
    pub message: String,
}

/// The form's text, one string per field, exactly as typed.
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct VehicleDraft {
    /// The typed name.
    pub name: String,
    /// The typed faction.
    pub faction: String,
    /// The typed armour class.
    pub armor_type: String,
    /// The typed amphibious value.
    pub amphibious: String,
    /// The typed primary threat.
    pub primary_threat: String,
    /// The typed profile image URL.
    pub profile_image_url: String,
}

#[cfg(any(target_arch = "wasm32", test))]
impl VehicleDraft {
    /// The draft an edit opens with: the stored row's values, an absent optional field as empty.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(super) fn from_vehicle(vehicle: &Vehicle) -> Self {
        Self {
            name: vehicle.name.clone(),
            faction: vehicle.faction.clone(),
            armor_type: vehicle.armor_type.clone(),
            amphibious: vehicle.amphibious.clone(),
            primary_threat: vehicle.primary_threat.clone(),
            profile_image_url: vehicle.profile_image_url.clone(),
        }
    }

    /// The text of `field`.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(super) fn text(&self, field: VehicleField) -> &str {
        match field {
            VehicleField::Name => &self.name,
            VehicleField::Faction => &self.faction,
            VehicleField::ArmorType => &self.armor_type,
            VehicleField::Amphibious => &self.amphibious,
            VehicleField::PrimaryThreat => &self.primary_threat,
            VehicleField::ProfileImageUrl => &self.profile_image_url,
        }
    }

    /// Replaces the text of `field` with `value`.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(super) fn set_text(&mut self, field: VehicleField, value: String) {
        let slot = match field {
            VehicleField::Name => &mut self.name,
            VehicleField::Faction => &mut self.faction,
            VehicleField::ArmorType => &mut self.armor_type,
            VehicleField::Amphibious => &mut self.amphibious,
            VehicleField::PrimaryThreat => &mut self.primary_threat,
            VehicleField::ProfileImageUrl => &mut self.profile_image_url,
        };
        *slot = value;
    }

    /// The write body this draft sends, with every value trimmed, or one problem per field that
    /// breaks a rule, in form order.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(super) fn validate(&self) -> Result<VehicleWrite, Vec<FieldProblem>> {
        let mut problems = Vec::new();
        let mut checked = VehicleDraft::default();
        for field in VehicleField::ALL {
            match check_field(field, self.text(field)) {
                Ok(value) => checked.set_text(field, value.to_owned()),
                Err(message) => problems.push(FieldProblem { field, message }),
            }
        }
        if !problems.is_empty() {
            return Err(problems);
        }
        Ok(VehicleWrite {
            name: checked.name,
            faction: checked.faction,
            armor_type: checked.armor_type,
            amphibious: checked.amphibious,
            primary_threat: checked.primary_threat,
            profile_image_url: checked.profile_image_url,
        })
    }
}

/// What the vehicle form writes: a new vehicle, or the stored row it was opened on.
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone, Debug, PartialEq)]
pub(super) enum FormTarget {
    /// A vehicle that does not exist yet; the form sends `POST /vehicle-database`.
    Create,
    /// A stored vehicle; the form sends `PUT /vehicle-database/{id}` and replaces every field.
    Edit(Vehicle),
}

#[cfg(any(target_arch = "wasm32", test))]
impl FormTarget {
    /// The dialog's title.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(super) fn title(&self) -> &'static str {
        match self {
            FormTarget::Create => "Add vehicle",
            FormTarget::Edit(_) => "Edit vehicle",
        }
    }

    /// The submit button's label.
    #[cfg(target_arch = "wasm32")]
    pub(super) fn submit_label(&self) -> &'static str {
        match self {
            FormTarget::Create => "Add vehicle",
            FormTarget::Edit(_) => "Save changes",
        }
    }

    /// The draft the form opens with: empty for a new vehicle, the stored values for an edit.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(super) fn draft(&self) -> VehicleDraft {
        match self {
            FormTarget::Create => VehicleDraft::default(),
            FormTarget::Edit(vehicle) => VehicleDraft::from_vehicle(vehicle),
        }
    }

    /// The request that writes `body` to this target.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(super) fn request(&self, body: VehicleWrite) -> VehicleRequest {
        match self {
            FormTarget::Create => VehicleRequest::create(body),
            FormTarget::Edit(vehicle) => VehicleRequest::replace(vehicle.id.as_str(), body),
        }
    }
}

/// `raw` without leading and trailing whitespace, the byte order mark included, exactly as the
/// backend trims before its rules apply.
#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn trimmed(raw: &str) -> &str {
    raw.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}')
}

/// One field's trimmed value, or the sentence saying which rule it breaks.
#[cfg(any(target_arch = "wasm32", test))]
fn check_field(field: VehicleField, raw: &str) -> Result<&str, String> {
    let value = trimmed(raw);
    if value.is_empty() {
        return if field.is_required() {
            Err(format!("{} is required.", field.label()))
        } else {
            Ok(value)
        };
    }
    if let Some(limit) = field.max_chars()
        && value.chars().count() > limit
    {
        return Err(format!(
            "{} must be at most {limit} characters.",
            field.label()
        ));
    }
    if field == VehicleField::ProfileImageUrl && safe_image_src(value).is_none() {
        return Err(format!(
            "{} must be an https:// URL or a site path such as /uploads/….",
            field.label()
        ));
    }
    Ok(value)
}

#[cfg(test)]
#[path = "tests/vehicle_draft.rs"]
mod tests;
