-- Noctrinth's own: names, pictures and authors for content that came from
-- CurseForge.
--
-- The launcher learns what an installed file is by hashing it and asking
-- Modrinth, which has nothing to say about a CurseForge jar — so a pack
-- installed from a CurseForge zip drew several hundred rows of filename,
-- "Unknown" and no author. The ids were there all along on the content entry;
-- what was missing was anywhere to keep the answers.
--
-- Kept apart from the shared cache on purpose. That one is keyed by Modrinth
-- ids and shaped around Modrinth's payloads, so CurseForge rows in it would be
-- a standing trap for anything reading it by id.
--
-- `expires` is a unix timestamp; a row past it is refetched and overwritten.
CREATE TABLE curseforge_project_cache (
	project_id TEXT NOT NULL PRIMARY KEY,
	name TEXT NOT NULL,
	slug TEXT NULL,
	summary TEXT NULL,
	icon_url TEXT NULL,
	website_url TEXT NULL,
	author_id TEXT NULL,
	author_name TEXT NULL,
	author_url TEXT NULL,
	author_avatar_url TEXT NULL,
	expires INTEGER NOT NULL
);

CREATE TABLE curseforge_file_cache (
	file_id TEXT NOT NULL PRIMARY KEY,
	project_id TEXT NOT NULL,
	display_name TEXT NOT NULL,
	file_date TEXT NULL,
	expires INTEGER NOT NULL
);

CREATE INDEX curseforge_file_cache_project
	ON curseforge_file_cache(project_id);
