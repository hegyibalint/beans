package dev.blnt.beans.sidecar.model;

import com.fasterxml.jackson.annotation.JsonSubTypes;
import com.fasterxml.jackson.annotation.JsonTypeInfo;

import java.util.List;

/**
 * How a unit names its inputs. A {@link Tree} is a rule that keeps matching as
 * files appear, so adding a source file needs no re-import; {@link Files} is
 * the escape hatch for inputs no pattern can describe.
 */
@JsonTypeInfo(use = JsonTypeInfo.Id.NAME, include = JsonTypeInfo.As.PROPERTY, property = "kind")
@JsonSubTypes({
    @JsonSubTypes.Type(value = Selector.Tree.class, name = "tree"),
    @JsonSubTypes.Type(value = Selector.Files.class, name = "files")
})
public sealed interface Selector {

    /** True when the contents are build output rather than hand-written source. */
    boolean generated();

    record Tree(String base, List<String> includes, List<String> excludes, boolean generated)
            implements Selector {}

    record Files(List<String> files, boolean generated) implements Selector {}
}
