**Status:** live

# Property-test acceptance evidence

The API readiness command treats generated cases and Rust test functions as separate counts.
Each property entry in requirements.json names an expected run and its minimum completed cases.
The domain-wide entry includes invariants whose implementations are still required, so passing
policy, quota and identity helpers cannot satisfy the entire API completion contract.

## Execution

Run cargo xtask db test-it for the database suite. The command creates its isolated namespace,
prints the effective seed, and uses libtest --show-output to retain successful property records.
Any present PROPTEST_CASES value is an error before database allocation, including zero and
positive values. To explore another reproducible input stream, set PROPTEST_RNG_SEED to a
nonempty decimal u64. The default seed is 2026092201.

The test helper invokes the actual property body, increments the completed count only after
success, and hashes length-framed Debug representations of those inputs. Rejected inputs do
not contribute. Failed properties produce counterexamples and no success record. Counts of
zero are rejected before the proptest runner starts. ChaCha and the requested count are
explicit; the source and Cargo.lock fingerprints bind the generator, formatting and toolchain
context needed to interpret the digest. A digest identifies the input stream; it does not
establish exhaustive input coverage.

Automatic local regression-file replay is disabled in the measured stream. Failed run output
retains the seed and shrunk counterexample; convert confirmed defects into explicit regression
fixtures. Existing receipts expire after 24 hours and changes to source or configuration
invalidate them. Fingerprints include the entire PROPTEST_* environment inventory, distinguishing
unset variables from empty values without printing configuration secrets.

## Acceptance

A property receipt includes all parsed property-run records from its command output. Acceptance
requires an exact match between receipt and output; unique property IDs; the expected single
configuration marker; effective environment values; the required seed and algorithm; nonzero
completed counts equal to requested counts; and input digests. The ordinary command exit,
test-name matches, ignored-test rejection and source/configuration checks still apply. Cargo
cases are identified by executable (or documentation crate) and matched test name. Identical
function names in different integration binaries count separately; repeating output from the
same binary does not create additional acceptance cases.

Negative controls cover vacuous runs, failed invariants, rejected-case accounting, missing runs,
duplicate records, partial runs, incompatible seed/algorithm, malformed records and mismatched
receipts. These controls validate the recorder and acceptance harness. They do not establish
correctness of reservations, sessions, telemetry, artifacts, commands or external services.
