package dev.blnt.beans.gradle.model;

import java.io.File;
import java.util.ArrayList;
import java.util.Collection;
import java.util.Collections;
import java.util.List;
import org.gradle.api.Project;
import org.gradle.api.plugins.JavaPluginExtension;
import org.gradle.api.tasks.SourceSet;
import org.gradle.api.tasks.SourceSetContainer;
import org.gradle.jvm.toolchain.JavaToolchainService;
import org.gradle.tooling.provider.model.ToolingModelBuilder;

final class ModelBuilder implements ToolingModelBuilder {
    @Override
    public boolean canBuild(String name) {
        return ProjectModel.class.getName().equals(name);
    }

    @Override
    public Object buildAll(String name, Project project) {
        List<SourceSetModel> sources = new ArrayList<>();
        SourceSetContainer sourceSets = project.getExtensions().findByType(SourceSetContainer.class);
        if (sourceSets == null) {
            return new Snapshots.Project(path(project.getProjectDir()), null, sources);
        }

        // Read the common JVM source-set contract without inspecting language-specific tasks.
        for (SourceSet sourceSet : sourceSets) {
            sources.add(new Snapshots.SourceSet(sourceSet.getName(),
                    paths(sourceSet.getAllSource().getSrcDirs(), true),
                    paths(sourceSet.getAllSource().getFiles(), true),
                    paths(sourceSet.getCompileClasspath().getFiles(), false),
                    paths(sourceSet.getOutput().getClassesDirs().getFiles(), true)));
        }

        // Report the project's default toolchain, not an inferred compiler-task selection.
        String javaHome = null;
        JavaPluginExtension java = project.getExtensions().findByType(JavaPluginExtension.class);
        if (java != null) {
            JavaToolchainService toolchains = project.getExtensions().getByType(JavaToolchainService.class);
            javaHome = path(toolchains.launcherFor(java.getToolchain()).get()
                    .getMetadata().getInstallationPath().getAsFile());
        }
        return new Snapshots.Project(path(project.getProjectDir()), javaHome, sources);
    }

    private static List<String> paths(Collection<File> files, boolean sort) {
        List<String> paths = new ArrayList<>();
        for (File file : files) {
            paths.add(path(file));
        }
        if (sort) {
            Collections.sort(paths);
        }
        return paths;
    }

    private static String path(File file) {
        return file.toPath().toAbsolutePath().normalize().toString();
    }
}
