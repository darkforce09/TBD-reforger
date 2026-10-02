//! The identifiers of a prefab in a terrain's prefab catalogue.
//!
//! **Role:** declares [`PrefabId`], the 32-bit catalogue identifier the prefab catalogue and the
//! building blueprint archive carry, and [`InstancePrefabId`], the 16-bit form the
//! [`crate::pod::instance::ObjectInstancePod`] row carries, with the conversions between them.
//! **Position:** the archive records in [`crate::archives::prefabs`] and
//! [`crate::archives::blueprints`] and the object row in [`crate::pod::instance`] hold these
//! types; the developer tools build them and the map engine reads them.
//! **Signals & state:** none; plain `Copy` data types.
//! **Invariants:** each identifier has its integer's exact byte layout (rkyv archives the field
//! alone; the row form is `#[repr(transparent)]` and `Pod`); widening the row form to the archive
//! form never fails, narrowing is checked and fails above `u16::MAX`.

use newtype_ids::integer_id;

integer_id! {
    /// A prefab's identifier in the terrain's prefab catalogue (`objects/prefabs.rkyv`), as the
    /// catalogue entries, occluder descriptors and building blueprints carry it.
    #[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
    #[rkyv(derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord))]
    pub struct PrefabId(u32);
}

integer_id! {
    /// A prefab's catalogue identifier in the 16-bit width of an
    /// [`crate::pod::instance::ObjectInstancePod`] row (its `prefab_idx` column).
    #[repr(transparent)]
    #[derive(Default, bytemuck::Pod, bytemuck::Zeroable)]
    pub struct InstancePrefabId(u16);
}

impl ArchivedPrefabId {
    /// The integer inside the archived identifier.
    #[must_use]
    pub fn get(&self) -> u32 {
        self.0.to_native()
    }

    /// The identifier copied out of the archive buffer.
    #[must_use]
    pub fn to_native(&self) -> PrefabId {
        PrefabId::new(self.0.to_native())
    }
}

impl core::fmt::Display for ArchivedPrefabId {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(&self.get(), formatter)
    }
}

impl PartialEq<PrefabId> for ArchivedPrefabId {
    fn eq(&self, other: &PrefabId) -> bool {
        self.get() == other.get()
    }
}

impl PartialEq<ArchivedPrefabId> for PrefabId {
    fn eq(&self, other: &ArchivedPrefabId) -> bool {
        self.get() == other.get()
    }
}

impl From<InstancePrefabId> for PrefabId {
    fn from(id: InstancePrefabId) -> Self {
        Self::new(u32::from(id.get()))
    }
}

impl TryFrom<PrefabId> for InstancePrefabId {
    type Error = core::num::TryFromIntError;

    /// Narrows a catalogue identifier to the row width.
    ///
    /// # Errors
    /// [`core::num::TryFromIntError`] when the identifier is above `u16::MAX`, which a row
    /// cannot carry.
    fn try_from(id: PrefabId) -> Result<Self, Self::Error> {
        u16::try_from(id.get()).map(Self::new)
    }
}
