# SHA-256 hashing

SHA-256 (FIPS 180-4) in [EnfScript](/documentation_v2/glossary.md#enfscript), since the engine
exposes no cryptographic hash to script. A mission [artifact](/documentation_v2/glossary.md#artifact)
loads only when the SHA-256 of its exact bytes equals the digest the platform published for it.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Core/Hashing/
├── TBD_Sha256.c          the hasher: bytes absorbed in pieces, digest as 64 lowercase hex
├── TBD_Sha256Job.c       the same hash spread across frames, with hashing time kept apart
└── TBD_Sha256SelfTest.c  FIPS test vectors, run once per process before an artifact is judged
```

## How it works

`TBD_Sha256` takes its input as bytes held one per element of an `array<int>`, because every script
call on a `string` copies the whole string, which makes reading a large string byte by byte
quadratic. Large inputs are read from a file with `FileHandle.ReadArray`; `BytesOf` converts a
short string. `Absorb(bytes, start, maxBytes)` takes the input in pieces and returns where the next
call continues; `HexDigest` pads, finishes and returns the digest; `Of` does both for one array;
`IsHexDigest` tests for 64 lowercase hex characters, the form of every digest here and on the
platform. Script `int` is signed 32-bit, so the unsigned words ride in it: `+` and `<<` wrap as two's
complement does, each logical right shift masks off the sign copies with a `LOW_<n>_BITS` constant,
and every byte is masked to 0..255. Messages up to 256 MiB fit, since the bit length is one word.

`TBD_Sha256Job` hashes a byte array in 1 KiB slices (`SLICE_BYTES`) until `STEP_BUDGET_MS` (8 ms)
have passed in a frame, then yields through the call queue for `STEP_PAUSE_MS` (1 ms). A subclass
receives the digest and the wall-clock time in `OnHashed`; `GetWorkMs` gives the time spent hashing
alone. `Start` and `Cancel` bump a run counter, so a step queued for an earlier run does nothing.
Without a call queue the job finishes in the current frame.

`TBD_Sha256SelfTest.Passed()` runs the vectors on its first call: the empty message, `abc`, the
448- and 896-bit messages, 55, 56, 64 and 1000 times `a`, the 896-bit message absorbed in uneven
pieces, a string with a byte above 127 decoded from a JSON `é` escape (checked as UTF-8 or
Latin-1, whichever the engine's decoder produced; skipped with a WARNING when it is neither), and a
string written to `$profile:TBD_Sha256SelfTest.bin` and read back with `ReadArray`. It logs
`[TBD][Sha256] self-test-passed vectors=11 bytesAbove127=utf8` (or `latin1`), or an ERROR naming
each failed vector: a digest mismatch that follows is then this engine build hashing wrongly, not a
damaged artifact.

## Authority

- Server: everything; `TBD_Sha256Job` and `TBD_Sha256SelfTest` carry `@authority server`, and
  their only callers are the server's mission loaders and fleet command argument checks.
  `TBD_Sha256` carries no tag and is a pure function of its input.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_Log`; the engine's `FileIO`, `FileHandle`, `JsonLoadContext` and call queue.
- Used by: in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/`,
  `TBD_DeployedMission` (runs the self-test), `TBD_MissionArtifactVerification` (its
  `TBD_ArtifactDigestJob` extends `TBD_Sha256Job`), `TBD_MissionArtifactCache` and
  `TBD_RuntimeDeploymentStruct` (`IsHexDigest`); `TBD_FleetCommandArguments` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/FleetCommands/` (`IsHexDigest`).
- Rules: a digest is always 64 lowercase hex characters; large inputs reach the hasher as a byte
  array read from a file, never byte by byte from a string; the self-test runs before the first
  artifact is judged; lines added stay ASCII; `cargo xtask mod compile` checks that the scripts
  compile, and the self-test line in a dedicated server's log checks the hasher on that engine
  build.

## Related documentation

- [Mission loaders](/apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/README.md) —
  how a deployed artifact is fetched, cached and verified
