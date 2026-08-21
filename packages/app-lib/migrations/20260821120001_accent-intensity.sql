-- How vivid the accent colour is drawn, as a percentage of the theme's own.
-- 100 is the theme exactly as it ships, which is why it is the default; below
-- that the purple is mixed toward grey, above it is pushed further out.
--
-- Numbered one second past the hour: upstream's migrations sit on round
-- timestamps, and two migrations sharing a version stop the launcher from
-- starting (see state::db::run_migrations).
ALTER TABLE settings
ADD COLUMN accent_intensity INTEGER NOT NULL DEFAULT 100;
