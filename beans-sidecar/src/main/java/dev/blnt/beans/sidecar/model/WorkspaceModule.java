package dev.blnt.beans.sidecar.model;

import java.util.List;

/** One module of the imported workspace. All paths are absolute. */
public record WorkspaceModule(
        String name,
        List<String> sourceRoots,
        List<String> testSourceRoots,
        List<String> generatedSourceRoots,
        List<String> compileClasspath,
        List<String> moduleDependencies,
        String jdkHome) {}
