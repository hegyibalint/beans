CREATE TABLE models (
    id INTEGER PRIMARY KEY,
    -- beans_core::location::Location
    location TEXT NOT NULL CHECK (json_valid(location)),
    -- 0 = JVM, 1 = Java
    kind INTEGER NOT NULL CHECK (kind IN (0, 1)),
    -- beans_platform_jvm::Class (kind = 0)
    -- beans_lang_java::model::File (kind = 1)
    model TEXT NOT NULL CHECK (json_valid(model)),
    content BLOB,
    CHECK (kind <> 1 OR content IS NOT NULL)
) STRICT;

CREATE INDEX jvm_models_by_root
ON models (json_extract(location, '$[0]'))
WHERE kind = 0;

CREATE INDEX jvm_classes_by_name
ON models (json_extract(model, '$.name'))
WHERE kind = 0;
