-- Noctrinth's own: files a CurseForge modpack brought with it used to be
-- stamped `curseforge`, the same as a CurseForge mod the user adds by hand.
-- The pack's content list only ever asks for `imported_modpack`, so an
-- instance installed from a CurseForge zip showed an empty pack with every one
-- of its several hundred mods filed under "Additional content" instead.
--
-- They are `curseforge_modpack` from now on. Existing instances are restamped
-- here rather than being left wrong until they are reinstalled: inside an
-- instance whose link says it came from an imported modpack, CurseForge
-- content is the pack's, because nothing else in the fork writes it there.
UPDATE instance_content_entries
SET source_kind = 'curseforge_modpack'
WHERE
	source_kind = 'curseforge'
	AND EXISTS (
		SELECT 1
		FROM instance_links
		WHERE
			instance_links.instance_id = instance_content_entries.instance_id
			AND instance_links.link_kind = 'imported_modpack'
	);
