CREATE TABLE java_types (
    model_id INTEGER NOT NULL REFERENCES models(id) ON DELETE CASCADE,
    node_index INTEGER NOT NULL CHECK (node_index >= 0),
    name TEXT NOT NULL,
    PRIMARY KEY (model_id, node_index)
) STRICT;

CREATE INDEX java_types_by_name ON java_types (name);
