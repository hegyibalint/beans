CREATE TABLE java_types (
    entry_id INTEGER NOT NULL REFERENCES entries(id) ON DELETE CASCADE,
    node_index INTEGER NOT NULL CHECK (node_index >= 0),
    name TEXT NOT NULL,
    PRIMARY KEY (entry_id, node_index)
) STRICT;

CREATE INDEX java_types_by_name ON java_types (name);
