package dev.blnt.beans.sidecar.model;

import java.util.List;

/** The result of an import, whichever adapter produced it. */
public record Workspace(String tool, List<Unit> units) {}
