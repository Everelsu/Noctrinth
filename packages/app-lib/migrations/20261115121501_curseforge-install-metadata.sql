CREATE TABLE curseforge_install_file_cache (
    key_hash TEXT NOT NULL,
    file_id INTEGER NOT NULL,
    payload TEXT NOT NULL,
    expires INTEGER NOT NULL,
    PRIMARY KEY (key_hash, file_id)
);
CREATE INDEX curseforge_install_file_cache_expires ON curseforge_install_file_cache(expires);
