package dev.blnt.beans.sidecar.model;

import java.util.List;

/** Logical artifact roots and their source representations, not compilation dependencies. */
public record ArtifactSources(List<Location> artifacts, List<ResourceFilter> sources) {}
