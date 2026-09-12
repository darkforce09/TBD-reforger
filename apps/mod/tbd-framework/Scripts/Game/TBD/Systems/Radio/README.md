# Systems/Radio

Tactical radio network frequency allocation, channel replication, and hardware tuning.

### Roles & Responsibilities
- `TBD_RadioPlan.c`: Data models representing network channels, encryption keys, and squad frequency presets.
- `TBD_RadioService.c`: Server-side distributor resolving radio channels for connected players from mission data.
- `TBD_RadioController.c`: Player controller RPC conduit delivering targeted radio plans to owning clients.
- `TBD_RadioClient.c`: Client cache storing local player frequency presets and driving UI displays.
- `TBD_RadioComponent.c`: Lifecycle component attached to `TBD_GameMode.et`.
- `TBD_RadioTuner.c`: Direct hardware tuner setting transceiver frequencies on character and vehicle radios.
- `TBD_RadioBridgeStub.c`: Integration boundary stub for third-party radio addons (TFAR, ACRE2).

### Call Flow & Contracts
Follows the Data/Service/Controller/Client pattern: `TBD_RadioService` -> `TBD_RadioController` RPC -> `TBD_RadioClient` cache -> `TBD_RadioTuner` configures engine radio transceivers.
