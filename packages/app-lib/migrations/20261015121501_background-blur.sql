-- Whether the background picture chosen in Appearance is blurred. The picture
-- itself is a file next to the database, not a column.
ALTER TABLE settings
ADD COLUMN background_blur INTEGER NOT NULL DEFAULT 1;
