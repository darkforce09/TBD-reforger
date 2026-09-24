**Status:** live

# Mission artifacts, reviews and deployment

Design for the mission requirements (`missions_immutable_artifacts`, `missions_approval_binding`,
`missions_artifact_consumers`, `missions_mission_transitions`, `missions_deployment_confirmation`,
`missions_authored_preservation`, `missions_review_comments`, `missions_review_workspace`). It
records the chosen semantics before implementation; acceptance evidence is the command output
recorded in progress_checkpoint.md.

## Artifacts

A mission artifact is the compiled mod document of one mission version together with every
input that determined it, stored once and never changed (`mission_artifacts`; a trigger refuses
updates and deletes):

- the mission version and the SHA-256 of its stored payload bytes;
- the mission metadata the compiler read (title, author, terrain, custom terrain, maximum players,
  time of day, weather, game mode) and its SHA-256;
- the cargo catalog and the current modpack (id and version) the capacity walk measured against,
  as a SHA-256 of the catalog's canonical form;
- the compiler version and the mission schema version;
- the exact compiled bytes delivered to game runtimes, their SHA-256 and length, which must not
  exceed 8 MiB (8,388,608 bytes);
- the compiler's findings.

The artifact digest is the SHA-256 over all of these; compiling identical inputs again returns the
existing artifact. A version that does not compile, or that carries authored gameplay data the
compiler cannot represent, yields no artifact and an actionable rejection naming each unsupported
path.

## Review

Submitting a mission (`POST /missions/{id}/submit`, the author or an administrator) compiles its
current version into an artifact and opens a review of exactly that artifact; the artifact, the
review, the `pending_approval` status and the `mission.submit` audit record commit together, under
the mission row lock taken before the actor's account lock. A version that does not compile answers
422 (`NO_PLACED_SLOTS`, `UNCOMPILABLE_VERSION` or `DOCUMENT_CONTRACT_VIOLATION`, the last with every
finding, including a document over 8 MiB) and changes nothing. A mission already under review
answers 409; a `pending_approval` mission without a review (queued before artifacts existed) is
submittable once more, which opens its review. At most one review per mission is pending (a partial
unique index).

Approval (`POST /approvals/{id}/approve`, `{artifact_id, conditions?}`) and rejection
(`POST /approvals/{id}/reject`, `{artifact_id, reason}`) name the artifact they decide. Deciding an
artifact that is no longer the one under review answers 409 `REVIEWED_ARTIFACT_CHANGED` with the
artifact that is; a mission without a pending review answers 409 `NO_PENDING_REVIEW`. The decision
records the review state (`approved`, `approved_with_conditions` or `rejected`), the conditions or
the reason as a review comment bound to the review, version and artifact, the mission status,
`missions.approved_artifact_id`, the reviewer stamp and the audit record in one transaction; both
routes answer the decided mission row. The approved artifact stays the one deployments load even
when the author saves later versions.

Review comments (`POST /missions/{id}/review-comments`, `{body, artifact_id?}`) form one thread per
mission, each with its author and the version and artifact it concerns, and are audited. The
thread and every review are read with `GET /missions/{id}/reviews`. An artifact's provenance is
`GET /missions/{id}/artifacts/{artifactId}`, its exact bytes `.../document` (the SHA-256 is the
strong entity tag), and the review workspace `.../workspace` answers the artifact with exactly the
version it compiled from, after checking the version payload against the digest the artifact
recorded; the editor opens it read-only. Mission versions never change after insert (a trigger
refuses updates), and a version an artifact refers to cannot be deleted. These routes belong to the
mission's author and administrators: a mission the caller cannot see answers 404 like a missing
one, and a visible mission the caller does not own answers 403.

The wire is `contracts_v2/definitions/mission-review.schema.json`; `tests/mission_review_contract.rs`
validates live responses and request bodies against it, and `tests/mission_review_binding.rs`
holds the behaviour.

## Deployment

The fleet runs a mission artifact inside the scenario registered for its terrain
(`fleet_scenarios`: terrain key, scenario header resource, display name; administrators maintain
it with `GET /fleet/scenarios`, `PUT` and `DELETE /fleet/scenarios/{terrainKey}`).

An administrator requests a deployment with `POST /servers/{id}/deployments`
(`{mission_id, artifact_id, event_mission_id?}`, answered 202 with the deployment). An in-game
administrator's selection arrives as `POST /game-runtime/deployments` from the server's
`mod_runtime` credential, naming the Arma identity that asked; the platform accepts it only when
that identity is linked to an unbanned platform administrator, who is then the requester. Both
validate the selection before anything is persisted, under the server row lock taken before the
mission, event and account locks:

- the server exists and is active, and no other deployment of it is in flight (409
  `DEPLOYMENT_IN_PROGRESS`);
- the mission is live and the artifact is the one its latest approval decided (409
  `ARTIFACT_NOT_APPROVED`);
- the artifact was compiled against exactly the server's required modpack, or against none when
  the server requires none (422 `MODPACK_MISMATCH`);
- a scenario is registered for the artifact's terrain (422 `TERRAIN_NOT_RUNNABLE`);
- an event mission, when named, runs this catalog mission, belongs to an event bound to this
  server, and every ORBAT seat of it pairs with exactly one compiled slot of the artifact with the
  same role (422 `ORBAT_ARTIFACT_MISMATCH` listing each unpaired seat). The pairing is stored as
  the deployment's slot bindings.

The deployment then records its transition and issues one fleet command in the same transaction.
When the server's open runtime session reported an artifact of the same terrain, the transition
is a `scenario_restart`: a `load_mission` command for the game runtime, which fetches the
artifact, verifies its SHA-256 and restarts the scenario in-process. Otherwise it is a
`host_restart`: a `restart_with_mission` command for the host agent, which restarts the server
process on the terrain's scenario. Both commands are issued only by deployments.

A runtime learns what to run from `GET /game-runtime/deployment` (the in-flight deployment, else
the latest confirmed one) and reads the bytes from `GET /game-runtime/artifacts/{artifactId}`,
which serves only artifacts of its own server's deployments. It reports what it loaded when it
starts its runtime session (`loaded_artifact_id`, `loaded_artifact_sha256`). A deployment is
`confirmed` only by a session of its server, started after the request, that reports its artifact
with the artifact's exact document SHA-256; a session reporting any other artifact or bytes fails
it. A deployment also fails when its fleet command fails, expires, is cancelled or ends
indeterminate, and fails as a partial transition when no session confirms it before its deadline
(10 minutes for a scenario restart, 20 for a host restart). An administrator cancels a deployment
whose command is still queued. Settlement runs in the deployment reconciler and before every read
of a server's deployments, so every outcome is observable on the deployment, with its reason, and
audited.

The roster of an event (`GET /game-runtime/events/{id}/roster`) lists the slot bindings of the
deployment the server runs, when it runs a mission of that event: roster derivation, deployment
authorization and game loading all name the same artifact. Deployment authorization accepts only
seats bound by a deployment of the server whose artifact the requesting runtime session reported
loading.

## Authored data

Every authored field the mission document carries survives compilation unchanged, and authored
gameplay data the compiler cannot represent is refused at compilation with 422
`UNSUPPORTED_AUTHORED_DATA`, naming each unsupported path.

## Consumers

- **Game runtime** (`apps/mod/tbd-framework`): `TBD_DeployedMission.c` reads the deployment and
  loads its artifact only after `TBD_MissionArtifactVerification.c` has matched the SHA-256 of the
  exact bytes (`Core/Hashing/`); `TBD_MissionArtifactCache.c` keeps the last verified artifact and
  its deployment identity in `$profile:TBD_MissionArtifactCache/`, which runs, with a warning,
  when the platform cannot be reached at boot. `TBD_LoadedArtifactReport.c` names the loaded
  artifact when the runtime session starts, which is what confirms a deployment. In game,
  `#tbd missions` lists `GET /game-runtime/missions` (`TBD_DeployableMissionList.c`) and
  `#tbd mission <n>` requests a deployment through `POST /game-runtime/deployments`
  (`TBD_MissionDeploymentRelay.c`); the platform decides from the administrator's linked identity.
- **Website**: the approvals queue and review drawer (`pages/administration/approvals/`), the
  mission hub's review history and submission refusals (`pages/mission_hub/mission_review/`), the
  read-only artifact workspace (`pages/mission_hub/review_workspace/`), and Server Control's
  deployments and fleet scenarios (`pages/administration/server_control/mission_deployments/`,
  `fleet_scenarios/`).
- **Tooling** (`tools_v2/xtask/src/commands/mod_ops/`): `mod playtest --mission` submits, approves
  and deploys through the platform and waits for the runtime session's confirmation;
  `--artifact-file`, `mod world-boot --mission` and `mod test-mission` stage a document into the
  artifact cache; `mod world-boot --compiled` stages the artifact the platform compiled on
  submission; `mod test-game-runtime-api` checks the deployment and artifact reads with a
  `mod_runtime` credential. The shared client is the `website_api_client` module.
