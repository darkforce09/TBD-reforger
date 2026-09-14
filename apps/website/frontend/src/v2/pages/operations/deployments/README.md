# My Deployments (`/deployments`)

The caller's own service record: what they are slotted into next, what they have flown, and the
leave they have booked off.

## Architecture
- **`page.rs`**: the route component. Fetches `GET /me/deployments` — both lists arrive in the one
  payload — and lays the page out: the identity and deployment-count column on the left, and the
  banner, history and leave sections on the right.
- **`active_orders.rs`**: the banner. The soonest deployment with its countdown, terrain, assigned
  slot and the two links into the operation, and the rest of the queue listed under it.
- **`service_record.rs`**: the combat history table — date, operation, role, outcome, replay link
  — and the decision about which stored replay strings may become an anchor.
- **`leave_of_absence.rs`**: the leave form, the date rules it checks before posting, and the
  table of the caller's own requests.
- **`leave_review_queue.rs`**: the administrator's queue, with approve and deny on each pending
  request.
- **`table_head.rs`**: the column heading all three tables share.
- **`tests/deployments.rs`**: the replay anchor sink over the shared adversarial corpus, the leave
  date rules and request body, the status badges, and the ban on fabricated personal telemetry.

## Not present in the legacy page
- **Personal kill/death, win-rate and favourite-loadout tiles**: the payload carries a total
  deployment count and nothing else personal, so the telemetry block is an explicit empty
  affordance. It stays that way until those counters are actually served.
