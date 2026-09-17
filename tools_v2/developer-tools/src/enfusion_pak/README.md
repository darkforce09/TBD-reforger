# Enfusion PAK archive access

One FORM/PAC1 reader parses the chunk directory and reads entry payloads at absolute file offsets. PAK payload compression uses zlib or raw deflate, according to the consumer policy. XOB mesh compression is a separate format.

- `archive_reader.rs`: bounded chunk/directory parsing, entry metadata, and stored payload reads.
- `payload.rs`: shared decompression and per-consumer length checks.
- `virtual_filesystem.rs`: sorted archive merging, first-match precedence, and virtual lookup.
- `world_source.rs`: game-root discovery and diagnostic/raw-byte access.
- `loose_source.rs`: directory sources and ordered fallback.
- `tests/`: synthetic policy coverage and the preserved reader regression suites.

`PakSet` uses case-insensitive blueprint lookup, strict DATA-span and decompressed-length checks, and zlib payloads. Invalid archives stop construction. `PakVfs` preserves world-tooling case-sensitive normalized lookup, zlib/raw-deflate decoding, and diagnostic-and-skip handling of malformed archives. Both use the same parser and payload implementation.

Directory sources remain exact-match first, then case-insensitive. A missing file may fall through to a later source; an error reading a file found in the selected source does not.
