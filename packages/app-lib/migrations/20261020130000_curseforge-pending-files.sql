-- Noctrinth's own: CurseForge files an instance is waiting on because their
-- author does not let launchers download them.
--
-- CurseForge answers such a file with no download URL. The launcher no longer
-- goes around that; it looks for the file on disk first (the Downloads folder,
-- other launchers' instances, its own instances), and whatever it cannot find
-- waits here until the user downloads it from CurseForge's site and the
-- Downloads folder is checked again.
--
-- `size` and `sha1` are what a downloaded file is recognised by. `source_kind`
-- and `disabled` are what the file is installed as once it turns up, so a mod
-- that belonged to a pack, or that the user had switched off, comes back the
-- same way.
CREATE TABLE curseforge_pending_files (
	instance_id TEXT NOT NULL,
	file_id TEXT NOT NULL,
	project_id TEXT NOT NULL,
	file_name TEXT NOT NULL,
	size INTEGER NULL,
	sha1 TEXT NULL,
	source_kind TEXT NOT NULL,
	disabled INTEGER NOT NULL DEFAULT 0,
	added INTEGER NOT NULL,

	PRIMARY KEY (instance_id, file_id),
	FOREIGN KEY (instance_id) REFERENCES instances(id) ON DELETE CASCADE
);
