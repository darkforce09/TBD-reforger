//! The `string_id!` macro: an identifier over an owned `String`.
//!
//! **Role:** declares a string identifier with the surface of a hand-written one (`new`,
//! `as_str`, `into_inner`, `Display`, `From`, `FromStr`, `Borrow<str>`, `AsRef<str>` and
//! equality with `str` and `&str`).
//! **Position:** expanded in the calling crate; names serde through [`crate::__private`].
//! **Signals & state:** none; the expansion is a plain data type.
//! **Invariants:** the id hashes, compares and orders exactly as its string, which is what makes
//! `Borrow<str>` sound; it serialises as the bare string; parsing never fails.

/// Declares a newtype identifier over a `String`.
///
/// The declared struct derives `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`, `PartialOrd`, `Ord`
/// and a transparent serde `Serialize` / `Deserialize`. Attributes written before the struct
/// (documentation, further derives) pass through. The inherent methods take the struct's own
/// visibility. A first argument `sqlx,` adds `#[derive(::sqlx::Type)] #[sqlx(transparent)]`,
/// which the calling crate's sqlx dependency resolves.
///
/// ```
/// use std::collections::HashMap;
///
/// newtype_ids::string_id! {
///     /// A terrain's identifier, such as `everon`.
///     pub struct TerrainName;
/// }
///
/// let everon = TerrainName::new("everon");
/// assert_eq!(everon, "everon");
/// assert_eq!(everon.to_string(), "everon");
/// let sizes = HashMap::from([(everon, 12_800_u32)]);
/// assert_eq!(sizes.get("everon"), Some(&12_800));
/// ```
#[macro_export]
macro_rules! string_id {
    (@declare [$($database:tt)*] $(#[$meta:meta])* $vis:vis struct $name:ident) => {
        $(#[$meta])*
        #[derive(
            ::core::fmt::Debug,
            ::core::clone::Clone,
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
        $vis struct $name(::std::string::String);

        // A caller uses the part of the generated surface it needs.
        #[allow(dead_code)]
        impl $name {
            /// The identifier spelled `value`.
            $vis fn new(value: impl ::core::convert::Into<::std::string::String>) -> Self {
                Self(value.into())
            }

            /// The identifier as a string slice.
            $vis fn as_str(&self) -> &str {
                &self.0
            }

            /// The string inside the identifier.
            $vis fn into_inner(self) -> ::std::string::String {
                self.0
            }
        }

        impl ::core::fmt::Display for $name {
            fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl ::core::convert::From<::std::string::String> for $name {
            fn from(value: ::std::string::String) -> Self {
                Self(value)
            }
        }

        impl ::core::convert::From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(::std::string::String::from(value))
            }
        }

        impl ::core::convert::From<$name> for ::std::string::String {
            fn from(id: $name) -> Self {
                id.0
            }
        }

        impl ::core::str::FromStr for $name {
            type Err = ::core::convert::Infallible;

            fn from_str(text: &str) -> ::core::result::Result<Self, Self::Err> {
                ::core::result::Result::Ok(Self(::std::string::String::from(text)))
            }
        }

        impl ::core::borrow::Borrow<str> for $name {
            fn borrow(&self) -> &str {
                &self.0
            }
        }

        impl ::core::convert::AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl ::core::cmp::PartialEq<str> for $name {
            fn eq(&self, other: &str) -> bool {
                self.0 == other
            }
        }

        impl ::core::cmp::PartialEq<&str> for $name {
            fn eq(&self, other: &&str) -> bool {
                self.0 == *other
            }
        }
    };
    (sqlx, $(#[$meta:meta])* $vis:vis struct $name:ident $(;)?) => {
        $crate::string_id! {
            @declare [#[derive(::sqlx::Type)] #[sqlx(transparent)]]
            $(#[$meta])* $vis struct $name
        }
    };
    ($(#[$meta:meta])* $vis:vis struct $name:ident $(;)?) => {
        $crate::string_id! { @declare [] $(#[$meta])* $vis struct $name }
    };
}
