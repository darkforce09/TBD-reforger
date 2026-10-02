//! The `integer_id!` macro: an identifier over an integer type the caller names.
//!
//! **Role:** declares an integer identifier (`new`, `get`, `into_inner`, `Display`, `From` both
//! ways, `FromStr`) that is `Copy` like the integer it wraps.
//! **Position:** expanded in the calling crate; names serde through [`crate::__private`].
//! **Signals & state:** none; the expansion is a plain data type.
//! **Invariants:** the id compares, orders, hashes, prints and parses exactly as its integer and
//! serialises as the bare number.

/// Declares a `Copy` newtype identifier over the integer type written in the parentheses.
///
/// The declared struct derives `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`,
/// `PartialOrd`, `Ord` and a transparent serde `Serialize` / `Deserialize`. Attributes written
/// before the struct pass through; the inherent methods take the struct's own visibility.
/// `FromStr` fails exactly where the integer's own parse fails, with the integer's error. A first
/// argument `sqlx,` adds `#[derive(::sqlx::Type)] #[sqlx(transparent)]`, which the calling
/// crate's sqlx dependency resolves.
///
/// ```
/// newtype_ids::integer_id! {
///     /// The row number of an event.
///     pub struct EventNumber(i64);
/// }
///
/// let event = EventNumber::new(42);
/// assert_eq!(event.get(), 42);
/// assert_eq!("42".parse::<EventNumber>(), Ok(event));
/// assert_eq!(event.to_string(), "42");
/// ```
#[macro_export]
macro_rules! integer_id {
    (@declare [$($database:tt)*] $(#[$meta:meta])* $vis:vis struct $name:ident($inner:ty)) => {
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
        $vis struct $name($inner);

        // A caller uses the part of the generated surface it needs.
        #[allow(dead_code)]
        impl $name {
            /// The identifier with the value `value`.
            $vis const fn new(value: $inner) -> Self {
                Self(value)
            }

            /// The integer inside the identifier.
            $vis const fn get(&self) -> $inner {
                self.0
            }

            /// The integer inside the identifier, consuming it.
            $vis const fn into_inner(self) -> $inner {
                self.0
            }
        }

        impl ::core::fmt::Display for $name {
            fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                ::core::fmt::Display::fmt(&self.0, formatter)
            }
        }

        impl ::core::convert::From<$inner> for $name {
            fn from(value: $inner) -> Self {
                Self(value)
            }
        }

        impl ::core::convert::From<$name> for $inner {
            fn from(id: $name) -> Self {
                id.0
            }
        }

        impl ::core::str::FromStr for $name {
            type Err = <$inner as ::core::str::FromStr>::Err;

            fn from_str(text: &str) -> ::core::result::Result<Self, Self::Err> {
                text.parse::<$inner>().map(Self)
            }
        }
    };
    (sqlx, $(#[$meta:meta])* $vis:vis struct $name:ident($inner:ty) $(;)?) => {
        $crate::integer_id! {
            @declare [#[derive(::sqlx::Type)] #[sqlx(transparent)]]
            $(#[$meta])* $vis struct $name($inner)
        }
    };
    ($(#[$meta:meta])* $vis:vis struct $name:ident($inner:ty) $(;)?) => {
        $crate::integer_id! { @declare [] $(#[$meta])* $vis struct $name($inner) }
    };
}
