# Content digest source

The source of `content_digest`: the one-shot digests, the incremental SHA-256 hasher, their hex
spelling, the error type and the crate root that exports them.

## Contents

```text
crates/foundation/content_digest/src/
├── error.rs          `Error` and `Result`: a file that cannot be read or changes length while hashed
├── hex_digests.rs    `sha256_hex`, `sha384_hex` and `sha384_hex_of_file` of whole inputs
├── lib.rs            the crate root: module header, `mod` lines and the re-exports
├── lowercase_hex.rs  the lowercase hex spelling of digest bytes, private
├── prelude.rs        the functions and the hasher for glob import
├── sha256_hasher.rs  `Sha256Hasher`: incremental SHA-256 with 8-byte little-endian length framing
└── tests/            unit tests: FIPS 180-4 vectors, file digests, the hasher's framing
```

## How it works

`hex_digests.rs` and `sha256_hasher.rs` both spell their result through `lowercase_hex.rs`, so the
one-shot and incremental digests of the same bytes are the same string. The hasher streams a file
through a 64 KiB buffer after feeding the length its metadata records, and compares that length
with the bytes it read.

## Boundaries

- Depends on: `sha2` and `thiserror`.
- Used by: the repository tooling, through the crate root.
- Rules: the moved migration checksum tests (`tests/hex_digests.rs`) pin that an unreadable file is
  `None`, never the digest of no bytes.
