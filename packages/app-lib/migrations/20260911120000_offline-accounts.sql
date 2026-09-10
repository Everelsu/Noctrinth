-- Accounts with nothing behind them but a name.
--
-- The game does not need an account system to start: singleplayer, a LAN world
-- and any server running in offline mode ask only for a name and the UUID that
-- goes with it. That is what this table holds, so the launcher has something to
-- launch as when nothing can be reached — see state/offline_auth.rs.
CREATE TABLE offline_users (
	uuid TEXT NOT NULL,
	username TEXT NOT NULL,
	active INTEGER NOT NULL DEFAULT FALSE,
	created INTEGER NOT NULL,
	PRIMARY KEY (uuid)
);
