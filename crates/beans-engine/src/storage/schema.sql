CREATE TABLE entries (
    id INTEGER PRIMARY KEY,
    model_kind INTEGER NOT NULL CHECK (model_kind IN (0, 1)),
    model TEXT NOT NULL CHECK (json_valid(model)),
    content BLOB,
    CHECK (model_kind <> 1 OR content IS NOT NULL)
) STRICT;

CREATE TABLE locations (
    entry_id INTEGER NOT NULL REFERENCES entries(id) ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK (position >= 0),
    kind TEXT NOT NULL CHECK (kind IN ('file', 'virtual', 'container')),
    value TEXT NOT NULL,
    PRIMARY KEY (entry_id, position)
) STRICT;
