# Commit subject mining

The one reading of git history that answers which commits claim a
[ticket](/documentation_v2/glossary.md#ticket) and when each landed: every commit subject that
names a ticket id, with the commit's author date in UTC.

## Contents

```text
tools_v2/ticket-engine/src/cli/shipping/
├── commit_subjects.rs  `mine_subjects`, `subject_ids` and `to_utc_z`: ticket ids mined from commit subjects
└── tests/              unit tests for the id boundaries, the UTC normalisation and a live-history smoke
```

## Boundaries

- Depends on: `git log --pretty=%H%x1f%aI%x1f%s` over the checkout's `HEAD`; the `regex` and
  `time` crates; `crate::validate_rfc3339_utc`, which every normalised date passes before it is
  returned.
- Used by: `cmd_stamp_sha` in `tools_v2/ticket-engine/src/cli/shipping.rs`, which mines the
  subjects before it writes a token estimate, and the token estimator in
  `tools_v2/ticket-engine/src/metrics/estimates/`, which takes the `SubjectCommit` lists as input.
- Rules:
  - an id counts only when no ASCII letter or digit precedes it, it runs to its last dotted
    number, and a subject that names it twice counts it once (`subject_id_boundary_pins`);
  - dates of any offset come back as whole-second UTC ending in `Z`, and a date without an offset
    refuses (`utc_normalization`);
  - each ticket's list runs oldest commit first.
