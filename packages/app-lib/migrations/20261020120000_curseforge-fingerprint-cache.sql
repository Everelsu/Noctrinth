-- Noctrinth's own: which CurseForge file an installed file is, worked out
-- from its content.
--
-- A jar dropped into `mods/` by hand, or brought over from a launcher that
-- never wrote the ids down, reaches the content listing with no ids at all.
-- Modrinth does not know it either, so it was drawn as a bare filename.
-- CurseForge can say what it is from its fingerprint (a whitespace-normalised
-- MurmurHash2 of the file), but working that out means reading the whole
-- file, so the answer is kept here, keyed by the SHA1 the listing already has.
--
-- `project_id` and `file_id` are both NULL when CurseForge was asked and did
-- not know the file; that answer is kept too, so it is not asked again on
-- every listing. `expires` is a unix timestamp, as in the other CurseForge
-- caches.
CREATE TABLE curseforge_fingerprint_cache (
	sha1 TEXT NOT NULL PRIMARY KEY,
	project_id TEXT NULL,
	file_id TEXT NULL,
	expires INTEGER NOT NULL
);
