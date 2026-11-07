package dev.blnt.beans.gradle.model;

import java.io.Serializable;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

public final class Snapshots {
    public static final class Project implements ProjectModel, Serializable {
        private static final long serialVersionUID = 1L;
        private final String directory;
        private final String javaHome;
        private final List<SourceSetModel> sourceSets;

        public Project(String directory, String javaHome, List<SourceSetModel> sourceSets) {
            this.directory = directory;
            this.javaHome = javaHome;
            this.sourceSets = immutable(sourceSets);
        }

        public Project(ProjectModel model) {
            this.directory = model.getProjectDirectory();
            this.javaHome = model.getJavaHome();
            List<SourceSetModel> copies = new ArrayList<>();
            for (SourceSetModel sourceSet : model.getSourceSets()) {
                copies.add(new SourceSet(sourceSet));
            }
            this.sourceSets = immutable(copies);
        }

        @Override public String getProjectDirectory() { return directory; }
        @Override public String getJavaHome() { return javaHome; }
        @Override public List<SourceSetModel> getSourceSets() { return sourceSets; }
    }

    public static final class SourceSet implements SourceSetModel, Serializable {
        private static final long serialVersionUID = 1L;
        private final String name;
        private final List<String> sourceDirectories;
        private final List<String> sourceFiles;
        private final List<String> compileClasspath;
        private final List<String> classesDirectories;

        public SourceSet(String name, List<String> sourceDirectories, List<String> sourceFiles,
                         List<String> compileClasspath, List<String> classesDirectories) {
            this.name = name;
            this.sourceDirectories = immutable(sourceDirectories);
            this.sourceFiles = immutable(sourceFiles);
            this.compileClasspath = immutable(compileClasspath);
            this.classesDirectories = immutable(classesDirectories);
        }

        public SourceSet(SourceSetModel model) {
            this(model.getName(), model.getSourceDirectories(), model.getSourceFiles(),
                    model.getCompileClasspath(), model.getClassesDirectories());
        }

        @Override public String getName() { return name; }
        @Override public List<String> getSourceDirectories() { return sourceDirectories; }
        @Override public List<String> getSourceFiles() { return sourceFiles; }
        @Override public List<String> getCompileClasspath() { return compileClasspath; }
        @Override public List<String> getClassesDirectories() { return classesDirectories; }
    }

    private static <T> List<T> immutable(List<T> values) {
        return Collections.unmodifiableList(new ArrayList<>(values));
    }

    private Snapshots() {}
}
