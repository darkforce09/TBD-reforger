# Review Workspace (`/missions/:id/artifacts/:artifact_id/workspace`)

The Scenario Creator opened read-only on exactly the version an artifact compiled from, so a
reviewer — or the author — inspects what was submitted with every editor tool and changes nothing.

## Architecture
- **`page.rs`**: route component — behind the sign-in gate, reads
  `GET /missions/:id/artifacts/:artifactId/workspace` (the artifact and its version, verified by the
  backend against the payload digest the artifact recorded), opens the editor's review mode
  (`apps/editor/shell/review_mode.rs`) on it, and mounts the editor with the banner over it. A
  refused read (403 for anyone but the author and administrators, 404 for an unknown artifact) is
  said in words with a link back to the mission.
- **`banner.rs`**: the persistent banner — "Review workspace of artifact <short digest>, version
  <semver>", the statement that nothing is saved, the mission title, compile time and document
  digest, and the artifact's compile findings (rule, severity, subject, message) in a disclosure.
- **`tests/review_workspace.rs`**: the banner's words and the refusal sentences, held against the
  captured workspace.

## What review mode changes in the editor
The editor boots from the reviewed version instead of the local draft and the server's current
version, and writes nothing while the review is open: no draft record, no flush on hide, no
warm-session marker, no writer election (the tab's writer role reads as read-only), no version
save, no mission-row mirror of time, weather, game mode, briefing or thumbnail, and no unsaved-work
prompt on leaving. Export Compiled compiles over the row fields the artifact recorded. Edits stay
in the tab's memory and are discarded with it.
