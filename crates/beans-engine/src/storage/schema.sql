CREATE TABLE entries (
    resource_url TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK (revision >= 0),
    model_kind INTEGER NOT NULL CHECK (model_kind IN (0, 1)),
    model BLOB NOT NULL,
    content BLOB,
    PRIMARY KEY (resource_url, revision),
    CHECK (model_kind <> 1 OR content IS NOT NULL)
) STRICT;
