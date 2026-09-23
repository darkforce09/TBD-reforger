# Server Control Layout Specification

```text
+-----------------+-----------------------------------------------------------+
| SERVERS 3       | TBD Primary — Everon          [Credentials] [LAUNCH ...]  |
| [Fleet scen.]   | 203.0.113.24:2001                                         |
| ● Primary       |-----------------------------------------------------------|
| ○ Secondary     | ACTIVE PERSONNEL 47/64 | TERRAIN Everon | SERVER FPS 58.7 |
| ○ Staging       |-----------------------------------------------------------|
|                 | FLEET COMMANDS              | MISSION DEPLOYMENTS         |
|                 | [Start][Stop][Restart]      | [Request a deployment]      |
|                 | [List players]              |  mission v  event mission v |
|                 | Broadcast [ message ] [>]   |  [Deploy]                   |
|                 | Kick [uid][session][reason] | refusal: unbound seats …    |
|                 | Your command: queued …      | Your deployment: recorded … |
|                 | HISTORY            Refresh  | DEPLOYMENTS       Refresh   |
|                 | [Expired] Restart w/ miss.  | [Failed] Operation Iron Veil|
|                 | [Queued] List players Cancel| [Confirmed] Operation Iron… |
|                 | [Succeeded] …  [Failed] …   |   detail rows · [Cancel]    |
+-----------------+-----------------------------------------------------------+

FLEET SCENARIOS SHEET (side sheet)            CREDENTIALS SHEET (side sheet)
| arland — Arland   {1111…}TBD_Arland.conf |  | secret once · issue · list  |
|   Updated by you, …        [Edit][Remove]|  | revoke with a reason        |
| Register or replace: [terrain][name]     |
|   [scenario id]              [Save]      |
```
