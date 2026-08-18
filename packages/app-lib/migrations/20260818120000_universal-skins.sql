-- Whether the game looks skins up by name for players the server sent none for.
-- On by default: an offline-mode server leaves everyone as Steve otherwise, which
-- is the state this exists to fix.
ALTER TABLE settings
ADD COLUMN universal_skins INTEGER NOT NULL DEFAULT 1;
