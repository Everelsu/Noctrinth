-- Noctrinth's own: the typeface the interface is set in. Empty is the theme's
-- own, Inter; `system` and `minecraft` are the two the interface offers by
-- name, and anything else is the family name of a font installed on the system.
ALTER TABLE settings
ADD COLUMN interface_font TEXT NOT NULL DEFAULT '';
