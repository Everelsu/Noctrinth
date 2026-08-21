-- Whether the accent preset colours the app's backgrounds as well as its
-- accent. On by default, since a preset that leaves the window the colour of a
-- different preset is the thing this was added to fix; off keeps every surface
-- and panel exactly as the theme paints it.
--
-- Numbered off upstream's round timestamps for the reason given in
-- 20260821120001_accent-intensity.sql.
ALTER TABLE settings
ADD COLUMN accent_tint_background INTEGER NOT NULL DEFAULT 1;
