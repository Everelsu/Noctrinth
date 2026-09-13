-- The refresh token an Ely.by sign-in on their own page comes back with.
--
-- Nullable, and that is the point: every account signed in before this one
-- exists has none, because the password flow it came from issues an access
-- token and a client token instead and refreshes them against a different
-- endpoint entirely. Which column is filled is therefore what says how an
-- account is kept alive, and both ways go on working side by side.
ALTER TABLE ely_users ADD COLUMN refresh_token TEXT;
