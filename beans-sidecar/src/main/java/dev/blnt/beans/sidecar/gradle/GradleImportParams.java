package dev.blnt.beans.sidecar.gradle;

/**
 * Params of {@code gradle/import}. {@code gradleHome} optionally points
 * at an installation instead of the project's wrapper (tests,
 * wrapper-less projects).
 */
public record GradleImportParams(String projectDir, String gradleHome) {}
