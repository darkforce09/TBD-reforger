# Content digest

The `content_digest` crate: lowercase hex SHA-256 and SHA-384 of bytes and files, and an
incremental SHA-256 hasher that frames each field with its length. The repository tooling hashes
through it, so every digest the tools write or compare is spelled the same way.

## Contents

```text
crates/foundation/content_digest/
├── Cargo.toml  the package: `sha2`, `thiserror`, layout tier 0
└── src/        the one-shot digests, the incremental hasher, the error type and the prelude
```

## How it works

`sha256_hex` and `sha384_hex` hash a whole input; `sha384_hex_of_file` hashes a file's exact
bytes, the checksum `sqlx` stores for a migration in `_sqlx_migrations.checksum`, and answers
`None` when the file cannot be read. `Sha256Hasher` feeds one SHA-256 in pieces: `update` takes
bytes as they are, `update_length_framed` prefixes them with their 8-byte little-endian length so
two field sequences never hash the same byte stream, and `update_file_length_framed` streams a
file framed the same way, failing when the file cannot be read or changes length while it is read.
`finalize_hex` ends the computation. Every digest is lowercase hexadecimal, two characters per
byte, equal to the output of `sha256sum` and `sha384sum`.

## Getting started

Run from the repository root:

```bash
cargo test -p content_digest   # FIPS 180-4 vectors, the file digests and the hasher's framing
```

## Configuration

No feature and no environment variable.

## Public surface

- `sha256_hex(&[u8]) -> String`, `sha384_hex(&[u8]) -> String`,
  `sha384_hex_of_file(&Path) -> Option<String>`.
- `Sha256Hasher`: `new`, `update`, `update_length_framed`, `update_file_length_framed`
  (returns `Result<()>`), `finalize_hex`.
- `Error` (`FileUnreadable`, `FileLengthChanged`) and `Result`.
- `prelude`: the functions and the hasher.

## Boundaries

- Depends on: `sha2` (the hash functions) and `thiserror`.
- Used by: `xtask` (`cargo xtask db repair-migration-checksum`, the API readiness fingerprints,
  the ballistics and equipment export digests, the mission publication client, the staging load
  and observation receipts).
- Rules: foundation tier, so the crate depends on no workspace crate
  (`cargo xtask verify crate-tiers`); a digest is never computed over partially read bytes.

## Related documentation

- [Crate boundary rules](/documentation/standards/crate_boundary_rules.md) — the dependency
  directions between the workspace crates.
