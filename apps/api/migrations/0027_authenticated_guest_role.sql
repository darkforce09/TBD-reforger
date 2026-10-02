-- Guest is an authenticated account without verified TBD membership.
-- Commit this enum extension before subsequent migrations use its value.
ALTER TYPE public.user_role ADD VALUE IF NOT EXISTS 'guest';
