# Hub 3: Mission Hub (`src/v2/pages/mission_hub`)

Central repository for community scenarios and editor launchpad.

## Pages
1. **`library/` (`/missions`)**: Searchable catalog of scenarios, with the slide-over dossier.
2. **`overview/` (`/missions/:id`)**: Detailed scenario dossier with launch editor trigger.
3. **`review_workspace/` (`/missions/:id/artifacts/:artifact_id/workspace`)**: The Scenario Creator
   opened read-only on the version an artifact compiled from, under a banner naming the artifact,
   the version and its compile findings.
4. **`create_dialog/`**: New scenario wizard modal.

## Shared
- **`mission_review/`**: the review record an author and an administrator read — every review with
  its decision, conditions or rejection reason, the approved artifact, the thread with a reply box,
  and the review workspace link — plus the submission control that names each refusal reason and
  lists every finding, and the artifact provenance and findings views the approvals drawer and the
  review workspace share.
