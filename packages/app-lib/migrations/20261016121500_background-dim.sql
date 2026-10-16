-- How far the background picture is darkened, from 0 (not at all) to 100.
ALTER TABLE settings
ADD COLUMN background_dim INTEGER NOT NULL DEFAULT 0;
