package dev.blnt.beans.sidecar;

import dev.blnt.beans.sidecar.model.Workspace;

import java.nio.file.Path;
import java.util.function.Consumer;

/**
 * One build tool's translation into our model. Discovered through
 * {@link java.util.ServiceLoader}, so an adapter is a jar on the classpath and
 * nothing in core knows the tools by name.
 */
public interface BuildAdapter {

    /** Reported in the handshake and on the imported {@link Workspace}. */
    String name();

    boolean accepts(Path workspaceRoot);

    Workspace importWorkspace(ImportParams params, Consumer<String> progress);
}
