//! The identity the compiler records with every compiled artifact.
//!
//! **Role:** holds [`COMPILER_PACKAGE_VERSION`], the compiler name and version the API stores with
//! each artifact as its provenance.
//! **Position:** read by the API's artifact store, which writes it beside the compiled bytes and
//! hashes it into the artifact digest.
//! **Signals & state:** none; one constant.
//! **Invariants:** the value is the literal `website-map-engine 0.1.0`, whatever crate or package
//! the compiler lives in.

/// The compiler identity recorded with every compiled artifact as part of its provenance and
/// hashed into the artifact digest. It is a stored value, so it is a literal that stays the same
/// whatever crate or package name the compiler lives under; changing it changes the digest of
/// every artifact compiled afterwards and splits stored artifacts from their recompilations.
pub const COMPILER_PACKAGE_VERSION: &str = "website-map-engine 0.1.0";
