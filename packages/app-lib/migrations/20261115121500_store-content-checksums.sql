CREATE TABLE store_blob_checksums (
    algorithm TEXT NOT NULL CHECK (algorithm IN ('sha1', 'md5')),
    digest TEXT NOT NULL,
    blob_sha512 TEXT NOT NULL REFERENCES store_blobs(sha512) ON DELETE CASCADE,
    PRIMARY KEY (algorithm, digest, blob_sha512)
);

CREATE INDEX store_blob_checksums_blob ON store_blob_checksums(blob_sha512);

INSERT OR IGNORE INTO store_blob_checksums (algorithm, digest, blob_sha512)
SELECT 'sha1', lower(instance_files.sha1), store_instance_files.blob_sha512
FROM instance_files JOIN store_instance_files ON store_instance_files.file_id = instance_files.id
WHERE length(instance_files.sha1) = 40;

ALTER TABLE curseforge_pending_files ADD COLUMN md5 TEXT;
