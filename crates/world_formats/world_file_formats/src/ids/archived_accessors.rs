//! The accessors every archived string identifier carries.
//!
//! **Role:** declares [`archived_string_id_accessors!`], which gives the archived form of a
//! string identifier the reads a zero-copy reader needs: `as_str`, `to_native`, `Display`,
//! `AsRef<str>` and equality with `str` and `&str`.
//! **Position:** expanded by [`crate::ids::terrain_feature_ids`] and
//! [`crate::ids::building_element_ids`] next to the identifier, where the archived tuple field is
//! visible.
//! **Signals & state:** none; the expansion is inherent methods and trait impls.
//! **Invariants:** the archived identifier reads exactly as the string it archives; nothing here
//! changes its byte layout.

/// Implements the zero-copy reads of `$archived`, the archived form of the string identifier
/// `$native`.
macro_rules! archived_string_id_accessors {
    ($native:ident, $archived:ident) => {
        impl $archived {
            /// The identifier as a string slice borrowed from the archive buffer.
            #[must_use]
            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }

            /// The identifier copied out of the archive buffer.
            #[must_use]
            pub fn to_native(&self) -> $native {
                $native::new(self.0.as_str())
            }
        }

        impl ::core::fmt::Display for $archived {
            fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                formatter.write_str(self.0.as_str())
            }
        }

        impl ::core::convert::AsRef<str> for $archived {
            fn as_ref(&self) -> &str {
                self.0.as_str()
            }
        }

        impl ::core::cmp::PartialEq<str> for $archived {
            fn eq(&self, other: &str) -> bool {
                self.0.as_str() == other
            }
        }

        impl ::core::cmp::PartialEq<&str> for $archived {
            fn eq(&self, other: &&str) -> bool {
                self.0.as_str() == *other
            }
        }
    };
}

pub(crate) use archived_string_id_accessors;
