-- The splash cube's choice lived only in the webview's localStorage, which the
-- dev server and the installed app keep apart (different origins), so it came
-- back as the default whenever the other one was opened. The interface still
-- caches it there, since the splash screen needs it before settings load.
ALTER TABLE settings
ADD COLUMN splash_cube TEXT NOT NULL DEFAULT 'theme';
