package dev.blnt.beans.sidecar.model;

import java.util.List;

/** Null entries are unrestricted; an empty list adds no resources beyond inputs. */
public record Classpath(List<ResourceFilter> inputs, List<ResourceFilter> entries) {}
