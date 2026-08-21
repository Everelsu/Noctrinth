-- The accent is chosen from a handful of presets now, the way a theme is,
-- rather than dialled in on two sliders nobody wants to fiddle with. 'theme'
-- means the theme's own accent, which is what the sliders meant at 100.
--
-- The two columns those sliders wrote are dropped: they were added earlier
-- today and no release ever carried them, so there is nothing to preserve.
ALTER TABLE settings
ADD COLUMN accent_preset TEXT NOT NULL DEFAULT 'theme';

ALTER TABLE settings
DROP COLUMN accent_intensity;

ALTER TABLE settings
DROP COLUMN accent_brightness;
