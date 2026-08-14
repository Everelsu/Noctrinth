-- Shared Minecraft options profile applied to every instance at launch.
-- Stored as JSON so the option catalogue can grow without further migrations.
ALTER TABLE settings
ADD COLUMN shared_game_options TEXT NULL;
