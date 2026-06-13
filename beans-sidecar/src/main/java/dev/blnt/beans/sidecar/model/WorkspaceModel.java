package dev.blnt.beans.sidecar.model;

import java.util.List;

/**
 * The one schema every import duty produces, whatever the build tool
 * (ADR-0031). The v1 actionable payload is the roots — beans indexes
 * them instead of blind-walking the workspace, and generated roots are
 * the annotation-processing level-0 story. The classpath rides along
 * dormant as the bytecode reader's future work-queue.
 */
public record WorkspaceModel(List<WorkspaceModule> modules) {}
