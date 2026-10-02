-- A database clock coordinates the REST budget across all API replicas.
CREATE TABLE discord_rest_schedule (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    next_request_at timestamptz NOT NULL DEFAULT now()
);
INSERT INTO discord_rest_schedule(singleton) VALUES (true);
ALTER TABLE user_discord_roles DROP CONSTRAINT user_discord_roles_pkey;
ALTER TABLE user_discord_roles ADD PRIMARY KEY (discord_id, guild_id, discord_role_id);
