-- Upstream ships this as 20260818120000. Noctrinth already used that version
-- for 20260818120000_universal-skins.sql, which is applied on installs since
-- 0.18.0, so this one is renumbered to upstream's 0.18.1 release time instead:
-- two files with one version make sqlx read the applied checksum as modified,
-- and it is this migration, unreleased at the time of the sync, that can move.
ALTER TABLE settings
ADD COLUMN sync_theme_across_devices INTEGER NOT NULL DEFAULT TRUE;

ALTER TABLE settings
ADD COLUMN sync_behavior_across_devices INTEGER NOT NULL DEFAULT TRUE;
