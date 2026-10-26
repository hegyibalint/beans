CREATE TABLE compilation_scopes (
    id INTEGER PRIMARY KEY,
    -- What workspace system this scope belongs to?
    -- 0 = TOML, 1 = Gradle, 2 = Maven
    kind INTEGER NOT NULL CHECK (kind IN (0, 1, 2)),
    -- Which project root this scope belongs to?
    -- We might have multiple, independent projects in a workspace
    root TEXT NOT NULL,
    -- An informative name to the scope often like "main" or "test"
    name TEXT NOT NULL,
    -- Which classpaths this scope sees?
    -- See `beans_core::classpath::Classpath`
    classpath TEXT NOT NULL CHECK (json_valid(classpath))
) STRICT;

CREATE TABLE models (
    id INTEGER PRIMARY KEY,
    -- Where is this source/binary/entry located in?
    -- See `beans_core::location::Location`
    location TEXT NOT NULL CHECK (json_valid(location)),
    -- Which platform the model belongs to?
    -- 0 = JVM, 1 = Java
    kind INTEGER NOT NULL CHECK (kind IN (0, 1)),
    -- The semantic model information encoded as JSON
    -- See `beans_platform_jvm::Class` (kind = 0)
    -- See `beans_lang_java::model::File` (kind = 1)
    model TEXT NOT NULL CHECK (json_valid(model)),
    -- The source text
    content BLOB,
    -- If we have a Java file, we must have its source
    CHECK (kind <> 1 OR content IS NOT NULL)
) STRICT;

CREATE INDEX jvm_models_by_root
ON models (json_extract(location, '$[0]'))
WHERE kind = 0;

CREATE INDEX jvm_classes_by_name
ON models (json_extract(model, '$.name'))
WHERE kind = 0;
