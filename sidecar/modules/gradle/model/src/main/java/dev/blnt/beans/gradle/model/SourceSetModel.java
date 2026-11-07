package dev.blnt.beans.gradle.model;

import java.util.List;

/** Language-neutral SourceSet configuration, not individual compiler-task overrides. */
public interface SourceSetModel {
    String getName();
    /** Declared roots, including directories not yet generated. */
    List<String> getSourceDirectories();
    /** Current allSource files, including resources and registered JVM languages. */
    List<String> getSourceFiles();
    List<String> getCompileClasspath();
    List<String> getClassesDirectories();
}
