package dev.blnt.beans.sidecar.model;

import com.fasterxml.jackson.annotation.JsonCreator;
import com.fasterxml.jackson.annotation.JsonSubTypes;
import com.fasterxml.jackson.annotation.JsonTypeInfo;
import com.fasterxml.jackson.annotation.JsonValue;
import java.util.List;

/** Matches Rust's transparent array of externally tagged location entries. */
public record Location(@JsonValue List<Entry> entries) {
    @JsonCreator(mode = JsonCreator.Mode.DELEGATING)
    public Location {}

    public static Location file(String path) {
        return new Location(List.of(new Entry.File(path)));
    }

    @JsonTypeInfo(use = JsonTypeInfo.Id.NAME, include = JsonTypeInfo.As.WRAPPER_OBJECT)
    @JsonSubTypes({
            @JsonSubTypes.Type(value = Entry.File.class, name = "File"),
            @JsonSubTypes.Type(value = Entry.Virtual.class, name = "Virtual"),
            @JsonSubTypes.Type(value = Entry.Container.class, name = "Container")
    })
    public sealed interface Entry {
        record File(@JsonValue String value) implements Entry {
            @JsonCreator(mode = JsonCreator.Mode.DELEGATING)
            public File {}
        }

        record Virtual(@JsonValue String value) implements Entry {
            @JsonCreator(mode = JsonCreator.Mode.DELEGATING)
            public Virtual {}
        }

        record Container(@JsonValue String value) implements Entry {
            @JsonCreator(mode = JsonCreator.Mode.DELEGATING)
            public Container {}
        }
    }
}
