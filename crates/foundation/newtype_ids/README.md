# Newtype identifiers

The `newtype_ids` crate: three macros that declare, in the calling crate, an identifier type
around a string, an integer or a UUID, so that an id is a type of its own rather than a bare
primitive any other id converts into silently.

## Contents

```text
crates/foundation/newtype_ids/
├── Cargo.toml  the package: `serde` and `uuid`, layout tier 0
├── src/        the three macros, the hidden re-export they expand through, and the prelude
└── tests/      each macro expanded in a crate of its own
```

## How it works

Each macro takes the struct declaration, with any documentation and extra attributes before it,
and expands to a tuple struct around one inner value:

```rust
newtype_ids::string_id! {
    /// A terrain's identifier, such as `everon`.
    pub struct TerrainName;
}
newtype_ids::integer_id!(pub struct EventNumber(i64));
newtype_ids::uuid_id!(sqlx, pub struct AccountKey);
```

| Macro | Inner value | Surface |
|---|---|---|
| `string_id!` | `String` | `new(impl Into<String>)`, `as_str`, `into_inner`, `Display`, `From<String>`, `From<&str>`, `From<Id> for String`, `FromStr` (never fails), `Borrow<str>`, `AsRef<str>`, `PartialEq<str>`, `PartialEq<&str>` |
| `integer_id!` | the integer type in the parentheses | `Copy`; `new`, `get`, `into_inner` (all `const`), `Display`, `From` both ways, `FromStr` with the integer's error |
| `uuid_id!` | `Uuid` | `Copy`; `new`, `as_uuid`, `into_inner` (all `const`), `Display`, `From` both ways, `FromStr` with `uuid::Error` |

Every id derives `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`, `PartialOrd`, `Ord` and a
transparent serde `Serialize` / `Deserialize`, so its JSON is exactly the JSON of its inner value
and it compares, orders and hashes as that value. A string id borrows as `str`, so a
`HashMap<TerrainName, _>` is looked up with `map.get("everon")`. The inherent methods take the
struct's own visibility.

The expansions name serde and uuid through the hidden `newtype_ids::__private` module, so a
calling crate needs neither dependency; serde's `crate` attribute spells that path out, so a
caller depends on this crate under its own name, never renamed.

The optional first argument `sqlx,` adds `#[derive(::sqlx::Type)] #[sqlx(transparent)]`. The
derive resolves in the calling crate against its own sqlx dependency, so this crate has no sqlx
dependency, feature or `cfg`, and sqlx stays inside the API crates. A UUID id with the `sqlx,` arm
needs the caller's sqlx `uuid` feature.

## Getting started

Run from the repository root:

```bash
cargo test -p newtype_ids   # the three macros expanded and exercised, plus the doc examples
```

## Configuration

No features and no environment variables.

## Public surface

- `string_id!`, `integer_id!`, `uuid_id!`, at the crate root and in `prelude`.
- `__private`: hidden; the expansions' path to `serde` and `uuid`, never named by a caller.

## Boundaries

- Depends on: `serde` (with `derive`), `uuid` (with `serde`).
- Used by: `api_identifiers` (`crates/api/api_identifiers`), which declares the API's typed ids
  with it; the typed ids of the libraries declare themselves with it.
- Rules: serialisation is transparent (`a_string_id_serialises_exactly_as_its_string` and its
  integer and UUID counterparts); a string id is looked up by `&str`
  (`a_map_keyed_by_a_string_id_is_looked_up_by_str`); foundation tier, so the crate depends on no
  workspace crate, and sqlx enters only through the caller (`cargo xtask verify crate-tiers`).

## Related documentation

- [Crate boundary rules](/documentation/standards/crate_boundary_rules.md) — the dependency
  directions between the workspace crates.
