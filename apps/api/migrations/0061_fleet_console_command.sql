-- An administrator's console command: one line for a dedicated server's RCON console, which the
-- host agent transmits once and whose reply it reports. The line can stop or restart the server
-- or its scenario, so each row records the command as process-changing and not idempotent in its
-- own columns; only the set of accepted actions changes here.
ALTER TABLE public.fleet_commands DROP CONSTRAINT fleet_commands_action_check;
ALTER TABLE public.fleet_commands ADD CONSTRAINT fleet_commands_action_check CHECK (action IN (
    'start', 'stop', 'restart', 'list_players', 'broadcast', 'kick', 'load_mission',
    'restart_with_mission', 'console_command'));
