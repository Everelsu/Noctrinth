-- How light the accent colour is drawn, as a percentage of the theme's own.
-- 100 is the theme exactly as it ships; below that the same purple is drawn
-- deeper, which is the difference between a lavender and a violet.
--
-- Numbered off upstream's round timestamps for the reason given in
-- 20260821120001_accent-intensity.sql.
ALTER TABLE settings
ADD COLUMN accent_brightness INTEGER NOT NULL DEFAULT 100;
