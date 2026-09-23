# Core

Foundational utilities: structured logging, asset alias resolution, player chat and SHA-256.

### Roles & Responsibilities
- `TBD_Log.c`: Static structured logger outputting standard greppable `[TBD][<channel>]` events across all mod systems.
- `TBD_Registry.c`: Static asset resolver translating mission semantic alias strings (`kit:*`, `vehicle:*`) into Enfusion prefab resource paths via `Data/registry.json`.
- `TBD_RegistryPocComponent.c`: Development harness component spawning registered aliases in Workbench for visual verification.
- `TBD_PlayerChat.c`: Server -> player chat, to one player (`Tell`) or every connected player (`TellEveryone`, which returns how many it reached).
- `Hashing/`: SHA-256 in script, since the engine exposes none; a mission artifact loads only when the SHA-256 of its exact bytes matches the published one.
  - `TBD_Sha256.c`: FIPS 180-4 over bytes held one per element of an `array<int>` (every script call on a `string` copies the whole string, so large inputs are read from a file with `FileHandle.ReadArray`), absorbed in pieces, as 64 lowercase hex; unsigned 32-bit words carried in signed `int` (masked logical shifts, bytes masked to 0..255). Measured on a dedicated server: 1 MiB in about 0.35 s of CPU.
  - `TBD_Sha256SelfTest.c`: FIPS test vectors, once per process before the first artifact is judged: the empty message, `abc`, the 448- and 896-bit messages, 55/56/64/1000 x `a`, uneven pieces, and bytes above 127 (UTF-8 or Latin-1, whichever the engine's JSON decoder produced), and a string written to a file and read back with `ReadArray`. `[TBD][Sha256] self-test-passed vectors=11 ...` or an ERROR naming each failed vector.
  - `TBD_Sha256Job.c`: The same hash spread across frames (1 KiB slices, about 8 ms per frame) for artifacts up to 8 MiB; reports wall time and hashing time apart.

### Call Flow & Contracts
Pure static utilities consumed across all domains without circular dependencies.
