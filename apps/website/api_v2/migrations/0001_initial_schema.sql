-- T-145 Phase 1 — initial schema (FROZEN REFERENCE, do not hand-edit).
-- Generated from `pg_dump --schema-only` of the Go pipeline (00-05 raw SQL +
-- GORM AutoMigrate) — the authoritative DB/wire contract. Verified byte-equal to
-- the Go schema by the G2 round-trip diff. Future schema changes add NEW migration
-- files on top of this one; never edit this file.





CREATE EXTENSION IF NOT EXISTS pgcrypto WITH SCHEMA public;



COMMENT ON EXTENSION pgcrypto IS 'cryptographic functions';



CREATE TYPE public.announcement_status AS ENUM (
    'draft',
    'published',
    'archived'
);



CREATE TYPE public.announcement_tag AS ENUM (
    'update',
    'event',
    'modpack_update',
    'important'
);



CREATE TYPE public.audit_severity AS ENUM (
    'info',
    'warn',
    'crit'
);



CREATE TYPE public.event_status AS ENUM (
    'scheduled',
    'open',
    'locked',
    'live',
    'completed',
    'cancelled'
);



CREATE TYPE public.game_mode AS ENUM (
    'pve_coop',
    'pvp',
    'zeus'
);



CREATE TYPE public.leave_status AS ENUM (
    'pending',
    'approved',
    'denied'
);



CREATE TYPE public.mission_outcome AS ENUM (
    'success',
    'failure',
    'aborted',
    'pending'
);



CREATE TYPE public.mission_status AS ENUM (
    'draft',
    'pending_approval',
    'live',
    'rejected',
    'archived'
);



CREATE TYPE public.registration_state AS ENUM (
    'registered',
    'waitlisted',
    'withdrawn',
    'attended',
    'no_show'
);



CREATE TYPE public.terrain_type AS ENUM (
    'everon',
    'arland',
    'custom'
);



CREATE TYPE public.user_role AS ENUM (
    'enlisted',
    'mission_maker',
    'admin',
    'leader'
);



CREATE TYPE public.weather_type AS ENUM (
    'clear',
    'overcast',
    'heavy_rain',
    'dense_fog'
);





CREATE TABLE public.announcements (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    title text NOT NULL,
    body text NOT NULL,
    snippet text,
    tag public.announcement_tag DEFAULT 'update'::public.announcement_tag NOT NULL,
    thumbnail_url text,
    author_id text NOT NULL,
    status public.announcement_status DEFAULT 'draft'::public.announcement_status NOT NULL,
    is_pinned boolean DEFAULT false NOT NULL,
    pushed_to_discord boolean DEFAULT false NOT NULL,
    discord_message_id text,
    published_at timestamp with time zone,
    created_at timestamp with time zone,
    updated_at timestamp with time zone,
    deleted_at timestamp with time zone
);



CREATE TABLE public.audit_logs (
    id bigint NOT NULL,
    severity public.audit_severity DEFAULT 'info'::public.audit_severity NOT NULL,
    actor_id text,
    actor_name text,
    action text NOT NULL,
    message text NOT NULL,
    target_type text,
    target_id text,
    metadata jsonb,
    created_at timestamp with time zone
);



CREATE SEQUENCE public.audit_logs_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;



ALTER SEQUENCE public.audit_logs_id_seq OWNED BY public.audit_logs.id;



CREATE TABLE public.discord_roles (
    discord_role_id text NOT NULL,
    name text NOT NULL,
    mapped_role public.user_role,
    priority bigint DEFAULT 0 NOT NULL
);



CREATE TABLE public.event_missions (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    event_id uuid NOT NULL,
    mission_id uuid NOT NULL,
    start_time timestamp with time zone NOT NULL,
    created_at timestamp with time zone,
    updated_at timestamp with time zone
);



CREATE TABLE public.event_registrations (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    event_mission_id uuid NOT NULL,
    discord_id text NOT NULL,
    slot_id uuid,
    state public.registration_state DEFAULT 'registered'::public.registration_state NOT NULL,
    registered_at timestamp with time zone DEFAULT now() NOT NULL
);



CREATE TABLE public.events (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    name_override text,
    start_time timestamp with time zone NOT NULL,
    briefing text,
    banner_image_url text,
    status public.event_status DEFAULT 'scheduled'::public.event_status NOT NULL,
    registration_locked boolean DEFAULT false NOT NULL,
    max_slots bigint DEFAULT 0 NOT NULL,
    created_by text NOT NULL,
    match_id uuid,
    created_at timestamp with time zone,
    updated_at timestamp with time zone,
    deleted_at timestamp with time zone
);



CREATE TABLE public.fire_missions (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    event_id uuid,
    created_by text NOT NULL,
    weapon_system text NOT NULL,
    fp_grid text NOT NULL,
    target_grid text NOT NULL,
    distance_m bigint NOT NULL,
    azimuth_deg numeric(5,1) NOT NULL,
    elevation_mils bigint NOT NULL,
    created_at timestamp with time zone
);



CREATE TABLE public.identity_link_codes (
    code character(6) NOT NULL,
    discord_id text NOT NULL,
    arma_id text,
    consumed_at timestamp with time zone,
    expires_at timestamp with time zone NOT NULL,
    created_at timestamp with time zone
);



CREATE TABLE public.match_player_stats (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    match_id uuid NOT NULL,
    discord_id text,
    arma_id text NOT NULL,
    role_played text,
    kills bigint DEFAULT 0 NOT NULL,
    deaths bigint DEFAULT 0 NOT NULL,
    team_kills bigint DEFAULT 0 NOT NULL,
    longest_kill_m bigint DEFAULT 0 NOT NULL,
    vehicles_destroyed bigint DEFAULT 0 NOT NULL,
    is_command boolean DEFAULT false NOT NULL,
    command_win boolean,
    source_event_id text NOT NULL,
    created_at timestamp with time zone
);



CREATE MATERIALIZED VIEW public.leaderboard_totals AS
 SELECT discord_id,
    sum(kills) AS kills,
    sum(deaths) AS deaths,
        CASE
            WHEN (sum(deaths) = (0)::numeric) THEN sum(kills)
            ELSE round((sum(kills) / sum(deaths)), 2)
        END AS kd_ratio,
    sum(team_kills) AS team_kills,
    max(longest_kill_m) AS longest_kill_m,
    sum(vehicles_destroyed) AS vehicles_destroyed,
    count(DISTINCT match_id) AS missions_played,
    count(*) FILTER (WHERE command_win) AS command_wins,
    NULLIF(count(*) FILTER (WHERE is_command), 0) AS command_games,
        CASE
            WHEN (count(*) FILTER (WHERE is_command) = 0) THEN (0)::numeric
            ELSE round(((count(*) FILTER (WHERE command_win))::numeric / (count(*) FILTER (WHERE is_command))::numeric), 3)
        END AS command_win_rate
   FROM public.match_player_stats s
  WHERE (discord_id IS NOT NULL)
  GROUP BY discord_id
  WITH NO DATA;



CREATE TABLE public.leave_requests (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    discord_id text NOT NULL,
    starts_on date NOT NULL,
    ends_on date NOT NULL,
    reason text,
    status public.leave_status DEFAULT 'pending'::public.leave_status NOT NULL,
    reviewed_by text,
    created_at timestamp with time zone
);



CREATE TABLE public.matches (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    source_match_id text,
    event_id uuid,
    mission_id uuid,
    terrain public.terrain_type,
    started_at timestamp with time zone NOT NULL,
    ended_at timestamp with time zone,
    outcome public.mission_outcome DEFAULT 'pending'::public.mission_outcome NOT NULL,
    winning_faction text,
    aar_replay_url text,
    created_at timestamp with time zone
);



CREATE TABLE public.mission_armories (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    mission_id uuid NOT NULL,
    faction text NOT NULL,
    category text NOT NULL,
    item_name text NOT NULL,
    quantity bigint,
    icon text,
    sort_order bigint DEFAULT 0 NOT NULL
);



CREATE TABLE public.mission_bookmarks (
    discord_id text NOT NULL,
    mission_id uuid NOT NULL,
    created_at timestamp with time zone
);



CREATE TABLE public.mission_versions (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    mission_id uuid NOT NULL,
    semver text NOT NULL,
    json_payload jsonb NOT NULL,
    editor_notes text,
    created_by text NOT NULL,
    created_at timestamp with time zone
);



CREATE TABLE public.missions (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    title text NOT NULL,
    author_id text NOT NULL,
    terrain public.terrain_type NOT NULL,
    custom_terrain_name text,
    game_mode public.game_mode NOT NULL,
    weather public.weather_type DEFAULT 'clear'::public.weather_type NOT NULL,
    time_of_day time without time zone DEFAULT '14:00:00'::time without time zone NOT NULL,
    max_players bigint NOT NULL,
    status public.mission_status DEFAULT 'draft'::public.mission_status NOT NULL,
    thumbnail_url text,
    briefing text,
    current_version_id uuid,
    rejection_reason text,
    reviewed_by text,
    reviewed_at timestamp with time zone,
    created_at timestamp with time zone,
    updated_at timestamp with time zone,
    deleted_at timestamp with time zone
);



CREATE TABLE public.modpack_mods (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    modpack_id uuid NOT NULL,
    name text NOT NULL,
    is_key_dependency boolean DEFAULT false NOT NULL,
    sort_order bigint DEFAULT 0 NOT NULL
);



CREATE TABLE public.modpacks (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    name text NOT NULL,
    version text NOT NULL,
    total_size_bytes bigint NOT NULL,
    workshop_url text,
    is_current boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone
);



CREATE TABLE public.orbat_reservations (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    event_mission_id uuid NOT NULL,
    squad text NOT NULL,
    reserved_by text NOT NULL,
    reserved_at timestamp with time zone DEFAULT now() NOT NULL
);



CREATE TABLE public.orbat_slots (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    event_mission_id uuid NOT NULL,
    faction text NOT NULL,
    squad text NOT NULL,
    callsign text,
    role text NOT NULL,
    loadout text,
    tag text,
    slot_index bigint NOT NULL,
    assigned_to text,
    assigned_at timestamp with time zone
);



CREATE TABLE public.refresh_tokens (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    discord_id text NOT NULL,
    token_hash text NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    revoked_at timestamp with time zone,
    created_at timestamp with time zone
);



CREATE TABLE public.registry_items (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    modpack_id uuid NOT NULL,
    resource_name text NOT NULL,
    display_name text NOT NULL,
    category text NOT NULL,
    icon_url text,
    kind text NOT NULL,
    sort_order bigint DEFAULT 0 NOT NULL,
    created_at timestamp with time zone,
    updated_at timestamp with time zone
);



CREATE TABLE public.server_status_histories (
    id bigint NOT NULL,
    server_id uuid NOT NULL,
    player_count bigint NOT NULL,
    server_fps numeric(5,1) NOT NULL,
    recorded_at timestamp with time zone DEFAULT now() NOT NULL
);



CREATE SEQUENCE public.server_status_histories_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;



ALTER SEQUENCE public.server_status_histories_id_seq OWNED BY public.server_status_histories.id;



CREATE TABLE public.server_statuses (
    server_id uuid NOT NULL,
    is_online boolean DEFAULT false NOT NULL,
    player_count bigint DEFAULT 0 NOT NULL,
    max_players bigint DEFAULT 64 NOT NULL,
    server_fps numeric(5,1) DEFAULT 0 NOT NULL,
    uptime_seconds bigint DEFAULT 0 NOT NULL,
    current_match_id uuid,
    ingame_time text,
    ingame_weather text,
    updated_at timestamp with time zone
);



CREATE TABLE public.servers (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    name text NOT NULL,
    ip inet NOT NULL,
    port bigint NOT NULL,
    required_modpack_id uuid,
    is_active boolean DEFAULT true NOT NULL
);



CREATE TABLE public.user_discord_roles (
    discord_id text NOT NULL,
    discord_role_id text NOT NULL,
    synced_at timestamp with time zone DEFAULT now() NOT NULL
);



CREATE TABLE public.users (
    discord_id text NOT NULL,
    username text NOT NULL,
    discord_handle text,
    avatar_url text,
    arma_id text,
    arma_character text,
    role public.user_role DEFAULT 'enlisted'::public.user_role NOT NULL,
    is_banned boolean DEFAULT false NOT NULL,
    ban_reason text,
    banned_by text,
    banned_at timestamp with time zone,
    total_deployments bigint DEFAULT 0 NOT NULL,
    attendance_rate numeric(5,2) DEFAULT 0 NOT NULL,
    last_login_at timestamp with time zone,
    created_at timestamp with time zone,
    updated_at timestamp with time zone,
    deleted_at timestamp with time zone
);



CREATE TABLE public.vehicle_databases (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    name text NOT NULL,
    faction text NOT NULL,
    armor_type text NOT NULL,
    amphibious text,
    primary_threat text,
    profile_image_url text
);



CREATE TABLE public.warnings (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    discord_id text NOT NULL,
    issued_by text NOT NULL,
    reason text NOT NULL,
    created_at timestamp with time zone
);



CREATE TABLE public.wiki_pages (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    slug text NOT NULL,
    category text NOT NULL,
    title text NOT NULL,
    icon text,
    body_md text NOT NULL,
    nav_order bigint DEFAULT 0 NOT NULL,
    updated_by text,
    updated_at timestamp with time zone
);



ALTER TABLE ONLY public.audit_logs ALTER COLUMN id SET DEFAULT nextval('public.audit_logs_id_seq'::regclass);



ALTER TABLE ONLY public.server_status_histories ALTER COLUMN id SET DEFAULT nextval('public.server_status_histories_id_seq'::regclass);



ALTER TABLE ONLY public.announcements
    ADD CONSTRAINT announcements_pkey PRIMARY KEY (id);



ALTER TABLE ONLY public.audit_logs
    ADD CONSTRAINT audit_logs_pkey PRIMARY KEY (id);



ALTER TABLE ONLY public.discord_roles
    ADD CONSTRAINT discord_roles_pkey PRIMARY KEY (discord_role_id);



ALTER TABLE ONLY public.event_missions
    ADD CONSTRAINT event_missions_pkey PRIMARY KEY (id);



ALTER TABLE ONLY public.event_registrations
    ADD CONSTRAINT event_registrations_pkey PRIMARY KEY (id);



ALTER TABLE ONLY public.events
    ADD CONSTRAINT events_pkey PRIMARY KEY (id);



ALTER TABLE ONLY public.fire_missions
    ADD CONSTRAINT fire_missions_pkey PRIMARY KEY (id);



ALTER TABLE ONLY public.identity_link_codes
    ADD CONSTRAINT identity_link_codes_pkey PRIMARY KEY (code);



ALTER TABLE ONLY public.leave_requests
    ADD CONSTRAINT leave_requests_pkey PRIMARY KEY (id);



ALTER TABLE ONLY public.match_player_stats
    ADD CONSTRAINT match_player_stats_pkey PRIMARY KEY (id);



ALTER TABLE ONLY public.matches
    ADD CONSTRAINT matches_pkey PRIMARY KEY (id);



ALTER TABLE ONLY public.mission_armories
    ADD CONSTRAINT mission_armories_pkey PRIMARY KEY (id);



ALTER TABLE ONLY public.mission_bookmarks
    ADD CONSTRAINT mission_bookmarks_pkey PRIMARY KEY (discord_id, mission_id);



ALTER TABLE ONLY public.mission_versions
    ADD CONSTRAINT mission_versions_pkey PRIMARY KEY (id);



ALTER TABLE ONLY public.missions
    ADD CONSTRAINT missions_pkey PRIMARY KEY (id);



ALTER TABLE ONLY public.modpack_mods
    ADD CONSTRAINT modpack_mods_pkey PRIMARY KEY (id);



ALTER TABLE ONLY public.modpacks
    ADD CONSTRAINT modpacks_pkey PRIMARY KEY (id);



ALTER TABLE ONLY public.orbat_reservations
    ADD CONSTRAINT orbat_reservations_pkey PRIMARY KEY (id);



ALTER TABLE ONLY public.orbat_slots
    ADD CONSTRAINT orbat_slots_pkey PRIMARY KEY (id);



ALTER TABLE ONLY public.refresh_tokens
    ADD CONSTRAINT refresh_tokens_pkey PRIMARY KEY (id);



ALTER TABLE ONLY public.registry_items
    ADD CONSTRAINT registry_items_pkey PRIMARY KEY (id);



ALTER TABLE ONLY public.server_status_histories
    ADD CONSTRAINT server_status_histories_pkey PRIMARY KEY (id);



ALTER TABLE ONLY public.server_statuses
    ADD CONSTRAINT server_statuses_pkey PRIMARY KEY (server_id);



ALTER TABLE ONLY public.servers
    ADD CONSTRAINT servers_pkey PRIMARY KEY (id);



ALTER TABLE ONLY public.user_discord_roles
    ADD CONSTRAINT user_discord_roles_pkey PRIMARY KEY (discord_id, discord_role_id);



ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_pkey PRIMARY KEY (discord_id);



ALTER TABLE ONLY public.vehicle_databases
    ADD CONSTRAINT vehicle_databases_pkey PRIMARY KEY (id);



ALTER TABLE ONLY public.warnings
    ADD CONSTRAINT warnings_pkey PRIMARY KEY (id);



ALTER TABLE ONLY public.wiki_pages
    ADD CONSTRAINT wiki_pages_pkey PRIMARY KEY (id);



CREATE INDEX idx_ann_published ON public.announcements USING btree (published_at DESC) WHERE ((status = 'published'::public.announcement_status) AND (deleted_at IS NULL));



CREATE INDEX idx_announcements_deleted_at ON public.announcements USING btree (deleted_at);



CREATE INDEX idx_audit_created ON public.audit_logs USING btree (created_at DESC);



CREATE INDEX idx_audit_logs_severity ON public.audit_logs USING btree (severity);



CREATE UNIQUE INDEX idx_event_mission ON public.event_missions USING btree (event_id, mission_id);



CREATE INDEX idx_event_missions_event_id ON public.event_missions USING btree (event_id);



CREATE INDEX idx_event_registrations_discord_id ON public.event_registrations USING btree (discord_id);



CREATE INDEX idx_event_registrations_event_mission_id ON public.event_registrations USING btree (event_mission_id);



CREATE INDEX idx_events_deleted_at ON public.events USING btree (deleted_at);



CREATE INDEX idx_events_start_time ON public.events USING btree (start_time);



CREATE INDEX idx_events_status ON public.events USING btree (status);



CREATE INDEX idx_identity_link_codes_discord_id ON public.identity_link_codes USING btree (discord_id);



CREATE UNIQUE INDEX idx_leaderboard_discord ON public.leaderboard_totals USING btree (discord_id);



CREATE INDEX idx_leave_requests_discord_id ON public.leave_requests USING btree (discord_id);



CREATE INDEX idx_link_codes_open ON public.identity_link_codes USING btree (discord_id) WHERE (consumed_at IS NULL);



CREATE INDEX idx_match_player_stats_discord_id ON public.match_player_stats USING btree (discord_id);



CREATE INDEX idx_match_player_stats_match_id ON public.match_player_stats USING btree (match_id);



CREATE UNIQUE INDEX idx_matches_source_match_id ON public.matches USING btree (source_match_id);



CREATE INDEX idx_mission_armories_mission_id ON public.mission_armories USING btree (mission_id);



CREATE INDEX idx_mission_payload_gin ON public.mission_versions USING gin (json_payload);



CREATE UNIQUE INDEX idx_mission_semver ON public.mission_versions USING btree (mission_id, semver);



CREATE INDEX idx_mission_versions_mission_id ON public.mission_versions USING btree (mission_id);



CREATE INDEX idx_missions_author_id ON public.missions USING btree (author_id);



CREATE INDEX idx_missions_deleted_at ON public.missions USING btree (deleted_at);



CREATE INDEX idx_missions_game_mode ON public.missions USING btree (game_mode);



CREATE INDEX idx_missions_status ON public.missions USING btree (status);



CREATE INDEX idx_missions_terrain ON public.missions USING btree (terrain);



CREATE INDEX idx_modpack_mods_modpack_id ON public.modpack_mods USING btree (modpack_id);



CREATE UNIQUE INDEX idx_mps_dedupe ON public.match_player_stats USING btree (match_id, arma_id, source_event_id);



CREATE UNIQUE INDEX idx_orbat_reservation ON public.orbat_reservations USING btree (event_mission_id, squad);



CREATE INDEX idx_orbat_reservations_event_mission_id ON public.orbat_reservations USING btree (event_mission_id);



CREATE INDEX idx_orbat_reservations_reserved_by ON public.orbat_reservations USING btree (reserved_by);



CREATE UNIQUE INDEX idx_orbat_slot ON public.orbat_slots USING btree (event_mission_id, squad, slot_index);



CREATE INDEX idx_orbat_slots_assigned_to ON public.orbat_slots USING btree (assigned_to);



CREATE INDEX idx_orbat_slots_event_mission_id ON public.orbat_slots USING btree (event_mission_id);



CREATE INDEX idx_refresh_tokens_discord_id ON public.refresh_tokens USING btree (discord_id);



CREATE UNIQUE INDEX idx_refresh_tokens_token_hash ON public.refresh_tokens USING btree (token_hash);



CREATE UNIQUE INDEX idx_reg_unique ON public.event_registrations USING btree (event_mission_id, discord_id);



CREATE UNIQUE INDEX idx_registry_items_modpack_resource ON public.registry_items USING btree (modpack_id, resource_name);



CREATE INDEX idx_registry_items_modpack_sort ON public.registry_items USING btree (modpack_id, sort_order);



CREATE INDEX idx_status_hist ON public.server_status_histories USING btree (server_id, recorded_at DESC);



CREATE UNIQUE INDEX idx_users_arma_id ON public.users USING btree (arma_id);



CREATE INDEX idx_users_deleted_at ON public.users USING btree (deleted_at);



CREATE INDEX idx_users_role ON public.users USING btree (role);



CREATE INDEX idx_users_role_active ON public.users USING btree (role) WHERE (deleted_at IS NULL);



CREATE INDEX idx_warnings_discord_id ON public.warnings USING btree (discord_id);



CREATE UNIQUE INDEX idx_wiki_pages_slug ON public.wiki_pages USING btree (slug);




