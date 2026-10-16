CREATE TABLE models (
    id INTEGER PRIMARY KEY,
    -- beans_core::location::Location
    location TEXT NOT NULL CHECK (json_valid(location)),
    -- 0 = JVM, 1 = Java
    model_kind INTEGER NOT NULL CHECK (model_kind IN (0, 1)),
    -- beans_platform_jvm::Class (model_kind = 0)
    -- beans_lang_java::model::File (model_kind = 1)
    model TEXT NOT NULL CHECK (json_valid(model)),
    content BLOB,
    CHECK (model_kind <> 1 OR content IS NOT NULL)
) STRICT;
