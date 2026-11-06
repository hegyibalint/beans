package dev.blnt.beans.sidecar.model;

import java.util.List;
import java.util.Objects;

/** Sources unmatched by any scope have unrestricted visibility. */
public record CompilationScopes(List<CompilationScope> scopes) {
    public CompilationScopes {
        Objects.requireNonNull(scopes, "scopes");
    }
}
