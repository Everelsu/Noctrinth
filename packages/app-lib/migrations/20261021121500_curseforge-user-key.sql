-- Noctrinth's own: a CurseForge API key the user brought, used instead of the
-- one the build was made with. Kept with the settings rather than in them: the
-- settings are handed to the frontend whole, and a key has no business there.
ALTER TABLE settings
ADD COLUMN curseforge_api_key TEXT NULL;
