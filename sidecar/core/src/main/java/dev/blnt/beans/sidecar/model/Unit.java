package dev.blnt.beans.sidecar.model;

import java.util.List;

/**
 * One compilation scope: what it can see is its own sources, its dependencies,
 * and their outputs. Finer than a build tool's module — a Gradle project with
 * main and test source sets is two units, because they see different things.
 *
 * <p>All paths are absolute. {@code dependsOn} holds ids of other units.
 */
public record Unit(
        String id,
        List<Selector> sources,
        List<String> dependsOn,
        List<String> classpath,
        String jdkHome) {}
