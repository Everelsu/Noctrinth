-- Noctrinth's own: where Java is downloaded from. `auto` tries Azul, Temurin,
-- Mojang and Mojang through the BMCLAPI mirror in turn; any one of them can be
-- pinned instead.
ALTER TABLE settings
ADD COLUMN java_provider TEXT NOT NULL DEFAULT 'auto';
