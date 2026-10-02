//! The `uuid_id!` macro: an identifier over a `Uuid`.
//!
//! **Role:** declares a UUID identifier (`new`, `as_uuid`, `into_inner`, `Display`, `From` both
//! ways, `FromStr`) that is `Copy` like the UUID it wraps.
//! **Position:** expanded in the calling crate; names serde and uuid through
//! [`crate::__private`], so the caller needs neither dependency.
//! **Signals & state:** none; the expansion is a plain data type.
//! **Invariants:** the id compares, orders, hashes, prints and parses exactly as its UUID (lower
//! case hyphenated when printed) and serialises as the bare UUID string.

/// Declares a `Copy` newtype identifier over a `Uuid`.
///
/// The declared struct derives `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`,
/// `PartialOrd`, `Ord` and a transparent serde `Serialize` / `Deserialize`. Attributes written
/// before the struct pass through; the inherent methods take the struct's own visibility.
/// `FromStr` accepts what `Uuid`'s own parse accepts and fails with its error. A first argument
/// `sqlx,` adds `#[derive(::sqlx::Type)] #[sqlx(transparent)]`, which the calling crate's sqlx
/// dependency (with its `uuid` feature) resolves.
///
/// ```
/// newtype_ids::uuid_id! {
///     /// A member's account.
///     pub struct AccountKey;
/// }
///
/// let text = "67e55044-10b1-426f-9247-bb680e5fe0c8";
/// let account: AccountKey = text.parse().unwrap();
/// assert_eq!(account.to_string(), text);
/// assert_eq!(AccountKey::new(account.into_inner()), account);
/// ```
#[macro_export]
macro_rules! uuid_id {
    (@declare [$($database:tt)*] $(#[$meta:meta])* $vis:vis struct $name:ident) => {
        $(#[$meta])*
        #[derive(
            ::core::fmt::Debug,
            ::core::clone::Clone,
            ::core::marker::Copy,
            ::core::cmp::PartialEq,
            ::core::cmp::Eq,
            ::core::hash::Hash,
            ::core::cmp::PartialOrd,
            ::core::cmp::Ord,
            $crate::__private::serde::Serialize,
            $crate::__private::serde::Deserialize,
        )]
        #[serde(crate = "newtype_ids::__private::serde", transparent)]
        $($database)*
        $vis struct $name($crate::__private::uuid::Uuid);

        // A caller uses the part of the generated surface it needs.
        #[allow(dead_code)]
        impl $name {
            /// The identifier holding `uuid`.
            $vis const fn new(uuid: $crate::__private::uuid::Uuid) -> Self {
                Self(uuid)
            }

            /// The UUID inside the identifier.
            $vis const fn as_uuid(&self) -> &$crate::__private::uuid::Uuid {
                &self.0
            }

            /// The UUID inside the identifier, consuming it.
            $vis const fn into_inner(self) -> $crate::__private::uuid::Uuid {
                self.0
            }
        }

        impl ::core::fmt::Display for $name {
            fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                ::core::fmt::Display::fmt(&self.0, formatter)
            }
        }

        impl ::core::convert::From<$crate::__private::uuid::Uuid> for $name {
            fn from(uuid: $crate::__private::uuid::Uuid) -> Self {
                Self(uuid)
            }
        }

        impl ::core::convert::From<$name> for $crate::__private::uuid::Uuid {
            fn from(id: $name) -> Self {
                id.0
            }
        }

        impl ::core::str::FromStr for $name {
            type Err = $crate::__private::uuid::Error;

            fn from_str(text: &str) -> ::core::result::Result<Self, Self::Err> {
                text.parse::<$crate::__private::uuid::Uuid>().map(Self)
            }
        }
    };
    (sqlx, $(#[$meta:meta])* $vis:vis struct $name:ident $(;)?) => {
        $crate::uuid_id! {
            @declare [#[derive(::sqlx::Type)] #[sqlx(transparent)]]
            $(#[$meta])* $vis struct $name
        }
    };
    ($(#[$meta:meta])* $vis:vis struct $name:ident $(;)?) => {
        $crate::uuid_id! { @declare [] $(#[$meta])* $vis struct $name }
    };
}
