# Match telemetry services

The serialisation that makes a corrected match report safe: reports of one match run one at a time,
and each locks the identities an earlier report of that match recorded.

## Contents

```text
apps/website/api_v2/src/match_telemetry/services/
├── mod.rs                   the module tree
└── result_serialization.rs  the source-match guard taken before the identity and account locks
```

## Boundaries

- Depends on: sqlx and a PostgreSQL connection inside the caller's transaction.
- Used by: `ingest_match_results` in
  `apps/website/api_v2/src/match_telemetry/handlers/match_results.rs`.
- Rules: `lock_source_and_prior_identities` takes the transaction advisory lock in namespace 1 on
  the hash of `source_match_id` before any identity or account lock, and returns the Arma
  identities the stored match already names, so the caller locks their accounts too; namespace 1
  stays reserved for match results.
