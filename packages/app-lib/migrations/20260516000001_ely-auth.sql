CREATE TABLE ely_users (
    uuid TEXT NOT NULL,
    active INTEGER NOT NULL DEFAULT FALSE,
    username TEXT NOT NULL,
    access_token TEXT NOT NULL,
    client_token TEXT NOT NULL,
    PRIMARY KEY (uuid)
);
