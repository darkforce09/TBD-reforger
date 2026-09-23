# Hub 6: Administration (`src/v2/pages/administration`)

Restricted tooling for server administrators, event coordinators, and mission reviewers.

## Pages
1. **`event_manager/` (`/admin/events`)**: Operation scheduling & slot template administration, and
   each operation's access — policies, event groups, reservation pools, participant evidence and
   waiting-list promotion.
2. **`server_control/` (`/admin/server`)**: Dedicated servers — the fleet command console (start,
   stop, restart, player list, broadcast, kick) with each command followed to its outcome, mission
   deployments of approved artifacts, the fleet scenario registry, and the machine credentials each
   server's host agent and game runtime authenticate with.
3. **`personnel/` (`/admin/personnel`)**: Community roster, promotions, and Discord role synchronization.
4. **`approvals/` (`/admin/approvals`)**: Mission review queue — the artifact under review with its
   provenance and compile findings, the review record and thread, the read-only review workspace,
   and the approve / approve-with-conditions / reject decision.
5. **`content_manager/` (`/admin/content`)**: CMS for announcements and doctrine articles.
6. **`audit_logs/` (`/admin/audit`)**: Security audit trail and administrative action logs.
