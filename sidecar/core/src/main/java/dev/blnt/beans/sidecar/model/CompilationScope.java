package dev.blnt.beans.sidecar.model;

import java.util.List;

/** Null dependencies are unrestricted; an empty list adds no resources beyond inputs. */
public record CompilationScope(List<ResourceFilter> inputs, List<ResourceFilter> dependencies) {}
