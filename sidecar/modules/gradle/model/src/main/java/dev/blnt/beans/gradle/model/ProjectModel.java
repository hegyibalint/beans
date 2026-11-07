package dev.blnt.beans.gradle.model;

import java.util.List;

/** A project-local snapshot; no live Gradle objects cross the tooling boundary. */
public interface ProjectModel {
    String getProjectDirectory();
    /** The project's default JVM toolchain; compiler tasks may override it. */
    String getJavaHome();
    List<SourceSetModel> getSourceSets();
}
