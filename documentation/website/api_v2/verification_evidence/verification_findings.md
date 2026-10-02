**Status:** live

# Milestone V findings

Every defect, gap and open question the verification-completeness milestone (V) found in the
website [API](/documentation/glossary/a_to_f.md#api), the frontend, the mod wire and the
tooling, with the class it was triaged into and where it ended: fixed by a named milestone agent,
filed as a [ticket](/documentation/glossary/n_to_z.md#ticket), recorded in a design note, or
left for an operator decision. The suites that found them are described in
[verification_completeness.md](verification_completeness.md); the gate results and the agent
roster are in [progress_checkpoint.md](progress_checkpoint.md).

## Triage classes

| Class | Meaning |
|---|---|
| FIX | It turned a V or register check red, was a fail-open test or gate on V's surface, or was small with a clear fix and a red-first test; fixed inside V. |
| NOTE | It needs an operator decision, is a feature, UX or refactor, sits in a file V did not own, or is large; filed as a ticket or recorded in a design note. A NOTE never leaves a V check red. |
| CLOSE | An open ticket the V work already fixed, closed with its evidence. |
| WATCH, CHECK | A count or a gate result held under observation until the final gates. |
| NOT RUN | A perturbation proof the permission classifier refused; left for the operator. |

Agent names in the outcome column are the V roster (for example K3b, the handler-defect batch);
"ship pending" means the ticket's body is filled and it ships once the V commit is stamped.
Paths are relative to `apps/website/api_v2/src/` for API code unless a longer path is given.

## Findings

| Id | Finding | Class | Outcome |
|---|---|---|---|
| V-F1 | frontend doc_audit red: 100 undocumented items in the other agent's committed data_viewer and equipment DTO files | FIX | fixed by L1 (operator-approved) |
| V-F2 | route-tags red: 12 equipment routes in a nested router outside routes.rs | FIX | fixed by K1a |
| V-F3 | /api/v1/debug/equipment-data/* answered anonymously in production, including /download | FIX (operator: development-only) | fixed by K1a |
| V-F4 | equipment-viewer goldens outside the seed reproduction chain | FIX (operator: include with a dataset fixture) | fixed by C1 and E3 |
| V-F5 | equipment handler module header lacked the four-point contract | FIX | fixed by K1a |
| V-F6 | mod TBD_FleetCommandArgumentsStruct cited the whole ClaimedFleetCommand | FIX | fixed by C2 |
| V-F7 | 24 API models without a @contract tag | FIX | fixed by A3 (123 tags in 53 files); T-1026 shipped |
| V-F8 | 12 handlers took a bare Query and answered 400 without the {error} envelope | FIX | fixed by K1a |
| V-F9 | contracts equipment-data-viewer and equipment-gameplay definition folders lacked READMEs | FIX | fixed by K1b |
| V-F10 | membership grace expires_at and overrides generated_at use chrono's default format, not rfc3339_utc | NOTE | T-1229 |
| V-F11 | PATCH /admin/users/{discordId} reportedly always answers 409 | NOTE | answered by V-F81; T-1244 |
| V-F12 | seeds/content_golden.sql is over 500 lines (seed data; the law-7 gate covers Rust and mod scripts) | NOTE | T-1230 |
| V-F13 | re-applying content_golden.sql after an API promotion or withdrawal refuses one statement | NOTE | T-1231 |
| V-F14 | require_test_database_url returns an Option that is never None: 46 dead skip branches | NOTE | T-1232; T-948 cancelled as fixed |
| V-F15 | the roster wire version is a bare literal 2 (consistent with schema and mod) | NOTE | T-1233 |
| V-F16 | the ci.yml website-frontend job red on main because of V-F1 | FIX | fixed by L1 (V-F1) |
| V-F17 | readiness_self_tests minimum 55 below the measured 59 | FIX | fixed by H1 (register minimum 59) |
| V-F18 | CLAUDE.md is not a readiness fingerprint input (AGENTS.md hashes as its link text), so instruction edits leave evidence current | NOTE | operator decision; dated note on T-1131 |
| V-F19 | assign_picker.rs:48 and member_search.rs:41 decode DataEnvelope of Member and drop total, limit and offset | NOTE | T-1234 |
| V-F20 | frontend MissionDetail.armory is a list of untyped values instead of typed rows | NOTE | T-1235 |
| V-F21 | frontend total fell to 1,796 when the doc_audit allowlist module was removed; frontend_quality must still hold | WATCH | closed: 1,826 passed at the final gate |
| V-F22 | readme-coverage on contracts: equipment-data-viewer fixtures and equipment-gameplay rules folders without READMEs | FIX | fixed by E1 |
| V-F23 | seed commands in xtask and the local development runbook ran psql without ON_ERROR_STOP | FIX | fixed by G1 (code) and E1 (runbook); T-1014 ship pending |
| V-F24 | xtask help listed a nonexistent migrate subcommand | FIX | fixed by G1; T-1012 shipped |
| V-F25 | /registry and /registry/compat rows carry database fields the cited closed item and edge definitions forbid | FIX | fixed by A3b |
| V-F26 | equipment-viewer positive fixtures were hand-trimmed samples of a gitignored 4.7 GB dataset | FIX | fixed by E3 (97 KB committed fixture, captures reproduce) |
| V-F27 | the game-runtime {error, details} envelope and the fleet refusal {code, state} have no schema | NOTE | T-1236 |
| V-F28 | fleet-command ExecutionResult.outcome has no per-action definitions | NOTE | T-1237 |
| V-F29 | documentation standards lacked the @contract sub-path and partial grammar | FIX | fixed by E1 |
| V-F30 | the API crate README lacked the V test binaries and support modules | FIX | fixed by E1 |
| V-F31 | T-1088: all 99 mod Struct classes and builders tagged | CLOSE | T-1088 ship pending |
| V-F32 | missions/models/registry.rs cited the catalogue item and edge definitions for API rows | FIX | fixed by G1 |
| V-F33 | arsenal-envelopes copies the kind and edge_type enums without a sync check | FIX | fixed by G1 (new contract_parity case) |
| V-F34 | perturbation proofs edited shared committed fixtures in place, so parallel agents saw transient reds | NOTE (process) | recorded in the checkpoint |
| V-F35 | JSON-body handlers mapped every rejection to 400 (no 413 or 415) | FIX | fixed by K2 |
| V-F36 | developer_login.rs and discord_oauth.rs answered a bad query in plain text | FIX | fixed by K2 |
| V-F37 | static mounts /uploads and /map-assets answer 404 without the {error} envelope | NOTE | T-1238 |
| V-F38 | rotate_session serialises on the account lock first, so only removing that lock turns a refresh race red | NOTE (design) | recorded in verification_completeness.md |
| V-F39 | X1's lock-removal perturbation and reruns refused by the permission classifier | NOT RUN | operator |
| V-F40 | 21 consumed write routes answer server-generated ids, secrets or request-time timestamps | FIX | fixed by D3b (normalisation table) |
| V-F41 | the mortar SavedFire DTO lived in a page with no round-trip test | FIX | fixed by D4 |
| V-F42 | content_golden.sql grew to 1,402 lines (see V-F12) | NOTE | T-1230 |
| V-F43 | the equipment ImportTestHook is compiled only under test | CLOSE | no issue |
| V-F44 | equipment_export_watcher never imports the diagnostic catalog in production | NOTE | T-1239 |
| V-F45 | json_difference.rs header named only contract_parity_goldens | FIX | fixed by G1 |
| V-F46 | contracts fixtures README omitted equipment-data-viewer; the backend reproduction was undescribed | FIX | fixed by E1 |
| V-F47 | Q2's perturbation (approved-artifact check removed) refused by the permission classifier | NOT RUN | operator |
| V-F48 | mission_artifact_support send() adds no ConnectInfo, so heavy suites share one rate-limit bucket | NOTE | T-1240 |
| V-F49 | docs and comments cited the deleted xtask engine_layer_rules.rs | FIX | fixed by E1 (docs) and G1 (comments) |
| V-F50 | rest of T-1055: stale text::gpu and rule lists in tooling, tests and ci.yml | FIX | fixed by G1; T-1055 shipped |
| V-F51 | xtask tooling prose rules red: a file named in prose not yet tracked; the history rule on legacy_archive.rs | CHECK | history rule fixed by G2 (V-F104); the prose case clears at commit |
| V-F52 | T-1041 (exemption mechanism) and F-1 (100 doc lines) | CLOSE | T-1041 shipped |
| V-F53 | 12 equipment viewer handlers still took a bare Query | FIX | fixed by K2 |
| V-F54 | readme-coverage: six equipment_data_viewer service folders without README | FIX | fixed by E1 |
| V-F55 | executor_claims lock_claim accepts reports on a lapsed but unreconciled lease | NOTE | T-1241 (operator decision) |
| V-F56 | review decisions take two locks where either alone suffices | NOTE (design) | recorded in verification_completeness.md |
| V-F57 | audit_stream_support header omitted controlled_races_audit; a race backend copied in two binaries | FIX | fixed by G1 |
| V-F58 | T-944 (audit id-order race and half-open socket) evidenced in both halves | CLOSE | T-944 shipped |
| V-F59 | Q4's two perturbations refused by the permission classifier | NOT RUN | operator |
| V-F60 | production allows one seat per participant per attached mission, not per event | NOTE (design) | recorded in verification_completeness.md by E1 |
| V-F61 | manual waitlist promote never answers 200 in generated runs | NOTE | T-1242 |
| V-F62 | a peer could bookmark another member's hidden draft and see its card (information leak) | FIX (security) | fixed by K3b; T-1173 ship pending |
| V-F63 | bookmarking a nonexistent mission answered 500 | FIX | fixed by K3b |
| V-F64 | remove_bookmark answered 200 for a nonexistent mission and discarded database errors | FIX | fixed by K3b |
| V-F65 | Path rejections answered text/plain 400 without the envelope (every Path route) | FIX | fixed by K3b (core/http/path_parameters.rs) |
| V-F66 | mission_armory accepted a negative quantity | FIX | fixed by K3b |
| V-F67 | mission_versions over-limit body answered 413 without details.code | FIX | fixed by K2 |
| V-F68 | route-acceptance round trip compared timestamps byte-wise (fraction digits differ) | FIX | fixed by K3a (instants) |
| V-F69 | MissionCreation and MissionChange schemas are closed but the handlers accept unknown fields | NOTE | T-1243 (operator decision) |
| V-F70 | route-acceptance contracts had no top-level array form | FIX | fixed by K3a |
| V-F71 | same as V-F68 on 13 operations routes | FIX | fixed by K3a |
| V-F72 | GET /events with an unknown scope answered 200 | FIX | fixed by K3b |
| V-F73 | event group administration answered 400 for an unknown discordId instead of 404 | FIX | fixed by K3b |
| V-F74 | four event request schemas are closed while handlers ignore unknown fields | NOTE | T-1243 |
| V-F75 | the server status stream opened for a malformed or unknown id | FIX | fixed by K3b |
| V-F76 | GET /servers/{id}/commands answered 200 for an unknown server | FIX | fixed by K3b |
| V-F77 | same as V-F65 on /matches/{matchId}/events | FIX | fixed by K3b |
| V-F78 | POST /game-runtime/sessions accepted no content type and answered 413 in plain text | FIX | fixed by K2 |
| V-F79 | same as V-F68 on fleet and telemetry routes | FIX | fixed by K3a |
| V-F80 | ServerInput and MachineCredentialIssue accept unknown keys though closed | NOTE | T-1243 |
| V-F81 | PATCH /admin/users/{discordId} answers 409 by design (Discord owns roles) and has no consumer | FIX + NOTE | framework refusal-only shape fixed by K3a; retirement T-1244 (operator decision) |
| V-F82 | flat JSON-body 400s in four administration handlers and the R1, R3 and R5 sets | FIX | fixed by K2 |
| V-F83 | R6's role-gate removal perturbation refused by the permission classifier | NOT RUN | operator |
| V-F84 | fire missions: a nonexistent event answered 200 and a POST stored a dangling event id | FIX | existence fixed by K3b; dated note on T-1177 |
| V-F85 | slot registration with a malformed slot id answered 404 | FIX | fixed by K3b |
| V-F86 | six more request schemas are closed while handlers accept unknown fields | NOTE | T-1243 |
| V-F87 | typed rows are strict: a leaderboard or announcement row missing a key fails the fetch | NOTE | documented by D4 in the feature docs |
| V-F88 | a frontend mock still carried the old 413 message | FIX | fixed by G2 |
| V-F89 | seed commands without ON_ERROR_STOP in the runbook and the runbook template | FIX | fixed by E1 |
| V-F90 | verification_core engine-layer rules listed a nonexistent text::gpu | FIX | fixed by G2; part of T-1055 |
| V-F91 | stale comments with old rule lists, module names and phase history | FIX | fixed by G2 |
| V-F92 | xtask prose rules red on README citations of an untracked file and legacy wording | CHECK | fixed by G2 (wording); the citation clears at commit |
| V-F93 | readme-coverage: nine of the other agent's API source folders | FIX | fixed by E1 |
| V-F94 | the design note omitted the registry row constraints case | FIX | fixed by E1 |
| V-F95 | the operations_events world answered no participants, so the array check was vacuous | FIX | fixed by G2 (participant seeded) |
| V-F96 | the design note lacked schema_items, refusal_only and the instant rule | FIX | fixed by E1 |
| V-F97 | mortar SaveResponse is private and decode-only; the frontend ignores token_type | NOTE | T-1245 |
| V-F98 | fire missions still skip viewer access and fire_missions.event_id has no foreign key | NOTE | operator decision (migration); dated note on T-1177, kept open |
| V-F99 | mission_lookup.rs doc said 404 on a bad id; the code answers 400 | FIX | fixed by G2 |
| V-F100 | list_server_commands answered query rejections with a fixed message | FIX | fixed by G2 |
| V-F101 | frontend fmt drift in the other agent's data_viewer files | FIX | fixed by G2 |
| V-F102 | clippy wasm32 warnings in changed data_viewer and content manager files | FIX | fixed by G2 |
| V-F103 | a review-binding case pinned the old flat 400; the null sweep lacked the moved equipment routes | FIX | fixed by G2 |
| V-F104 | history-rule red on legacy_archive.rs; a flaky equipment_gameplay test; a prose case red until commit | FIX | fixed by G2 (rename, cwd lock); T-1218 ship pending |
| V-F105 | three more handlers answered bad queries with a fixed message | FIX | fixed by G3 |
| V-F106 | dossier_upload.rs claimed the server's 413 names its size limit | FIX | fixed by G3 |
| V-F107 | data_viewer contents navigation bound aria-expanded to a bool | FIX | fixed by G3 |
| V-F108 | the rename leaves a local equipment_vehicle_exports/legacy/ folder unread | NOTE | T-1246 (operator); rename recorded on T-1218 |
| V-F109 | stale text::gpu history comments in both engines | FIX | fixed by G3 |
| V-F110 | environment_variables.md lacks the two EQUIPMENT_* variables; stale services header; viewer modules lack Role and Position headers | NOTE | T-1247 |
| V-F111 | readme-coverage 67 violations and 2 link breaks in the other agent's committed folders | FIX | fixed by E1c |
| V-F112 | aria-expanded bound to a bool in data_viewer field_row.rs:58 and the editor tool_row.rs:188 | NOTE | T-1248 |
| V-F113 | the golden seed keeps an attended registration on an event dated 2030, so My Deployments shows ATTENDED on an upcoming event | NOTE | T-1249 |
| V-F114 | R4 (missions reviews part) never reported; its perturbations were refused; its binary passes the full runs | NOT RUN | operator |
| V-F115 | 19 Workbench-generated Gameplay/Policy/Generated/ folders lack READMEs and their generator refuses foreign files | NOTE | T-1250 (operator decision, Workbench-bound) |
| V-F116 | 14 tbd-export equipment plugins have their menu attribute commented out; two export classes have no caller | NOTE | T-1251 |

## Tickets

The NOTE rows filed 23 idea tickets: T-1229 to T-1246 during the milestone, and T-1247 to
T-1251 at its close. The V work also settled existing tickets: T-1041, T-1026, T-1012, T-1105,
T-951, T-1055, T-944 and T-949 are shipped and take their `shipped_at` stamp after the commit;
T-1014, T-1088, T-1126, T-1173, T-1216 and T-1218 are filled and ship after that stamp; T-948,
T-986, T-933, T-906 and T-953 are cancelled with proof. T-1131 and T-1177 carry dated notes and
stay open for the operator.

## Related documentation

- [Verification completeness](verification_completeness.md) — the suites, their prefixes and the
  design facts recorded from V-F38, V-F56 and V-F60.
- [Progress checkpoint](progress_checkpoint.md) — the V gate results, the execution record and
  the open operator items.
- [Remaining milestones](remaining_milestones.md) — the register status after V.
