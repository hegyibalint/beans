package dev.blnt.beans.sidecar.gradle;

import dev.blnt.beans.sidecar.BuildAdapter;
import dev.blnt.beans.sidecar.ImportParams;
import dev.blnt.beans.sidecar.model.Selector;
import dev.blnt.beans.sidecar.model.Unit;
import dev.blnt.beans.sidecar.model.Workspace;
import org.gradle.tooling.GradleConnector;
import org.gradle.tooling.ProjectConnection;
import org.gradle.tooling.events.OperationType;
import org.gradle.tooling.events.StartEvent;
import org.gradle.tooling.model.idea.IdeaContentRoot;
import org.gradle.tooling.model.idea.IdeaDependency;
import org.gradle.tooling.model.idea.IdeaDependencyScope;
import org.gradle.tooling.model.idea.IdeaModule;
import org.gradle.tooling.model.idea.IdeaModuleDependency;
import org.gradle.tooling.model.idea.IdeaProject;
import org.gradle.tooling.model.idea.IdeaSingleEntryLibraryDependency;
import org.gradle.tooling.model.idea.IdeaSourceDirectory;

import java.io.File;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.EnumSet;
import java.util.HashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.function.Consumer;

/**
 * Reads a Gradle build through the Tooling API's stock {@code IdeaProject}
 * model, without injecting anything into the user's build.
 */
public final class GradleAdapter implements BuildAdapter {

    private static final List<String> BUILD_FILES = List.of(
            "settings.gradle", "settings.gradle.kts", "build.gradle", "build.gradle.kts");

    @Override
    public String name() {
        return "gradle";
    }

    @Override
    public boolean accepts(Path workspaceRoot) {
        return BUILD_FILES.stream().anyMatch(f -> Files.isRegularFile(workspaceRoot.resolve(f)));
    }

    @Override
    public Workspace importWorkspace(ImportParams params, Consumer<String> progress) {
        File projectDir = new File(params.workspaceRoot());
        GradleConnector connector = GradleConnector.newConnector().forProjectDirectory(projectDir);
        String gradleHome = params.option("gradleHome");
        if (gradleHome != null) {
            connector.useInstallation(new File(gradleHome));
        }

        progress.accept("Connecting to Gradle build at " + projectDir);
        try (ProjectConnection connection = connector.connect()) {
            IdeaProject idea = connection
                    .model(IdeaProject.class)
                    .withArguments("--quiet")
                    // Phase-level operations only. The unfiltered listener fires
                    // for every dependency download and inner operation.
                    .addProgressListener(
                            event -> {
                                if (event instanceof StartEvent) {
                                    progress.accept(event.getDescriptor().getDisplayName());
                                }
                            },
                            EnumSet.of(OperationType.PROJECT_CONFIGURATION, OperationType.BUILD_PHASE))
                    .get();
            return toWorkspace(idea);
        }
    }

    private static Workspace toWorkspace(IdeaProject idea) {
        // Module dependencies name their target by IDEA module name; unit ids
        // are built from project paths, so we need the mapping between them.
        Map<String, String> pathsByModuleName = new HashMap<>();
        for (IdeaModule module : idea.getModules()) {
            pathsByModuleName.put(module.getName(), module.getGradleProject().getPath());
        }

        List<Unit> units = new ArrayList<>();
        for (IdeaModule module : idea.getModules()) {
            String path = module.getGradleProject().getPath();
            String jdkHome = jdkHome(idea, module);

            List<Selector> mainSources = new ArrayList<>();
            List<Selector> testSources = new ArrayList<>();
            for (IdeaContentRoot root : module.getContentRoots()) {
                for (IdeaSourceDirectory dir : root.getSourceDirectories()) {
                    mainSources.add(tree(dir));
                }
                for (IdeaSourceDirectory dir : root.getTestDirectories()) {
                    testSources.add(tree(dir));
                }
            }

            // Sets, because a module carrying several test-ish source sets
            // reports the same dependency once per source set.
            Set<String> mainClasspath = new LinkedHashSet<>();
            Set<String> testClasspath = new LinkedHashSet<>();
            Set<String> mainDependsOn = new LinkedHashSet<>();
            Set<String> testDependsOn = new LinkedHashSet<>();
            for (IdeaDependency dependency : module.getDependencies()) {
                boolean testOnly = isTestScope(dependency);
                if (dependency instanceof IdeaSingleEntryLibraryDependency library) {
                    String entry = library.getFile().getAbsolutePath();
                    if (!testOnly) {
                        mainClasspath.add(entry);
                    }
                    testClasspath.add(entry);
                } else if (dependency instanceof IdeaModuleDependency moduleDependency) {
                    String targetPath = pathsByModuleName.get(moduleDependency.getTargetModuleName());
                    if (targetPath == null) {
                        continue;
                    }
                    String targetId = unitId(targetPath, "main");
                    if (!testOnly) {
                        mainDependsOn.add(targetId);
                    }
                    testDependsOn.add(targetId);
                }
            }

            String mainId = unitId(path, "main");
            if (!mainSources.isEmpty()) {
                units.add(new Unit(
                        mainId,
                        mainSources,
                        List.copyOf(mainDependsOn),
                        List.copyOf(mainClasspath),
                        jdkHome));
            }
            if (!testSources.isEmpty()) {
                testDependsOn.add(mainId);
                units.add(new Unit(
                        unitId(path, "test"),
                        testSources,
                        List.copyOf(testDependsOn),
                        List.copyOf(testClasspath),
                        jdkHome));
            }
        }
        return new Workspace("gradle", units);
    }

    /**
     * The stock model reports directories, not the source set's own filters, so
     * the pattern is the widest one that is still true. Which files matter is
     * the engine's call anyway: it hands each one to whichever language claims it.
     */
    private static Selector tree(IdeaSourceDirectory directory) {
        return new Selector.Tree(
                directory.getDirectory().getAbsolutePath(),
                List.of("**/*"),
                List.of(),
                directory.isGenerated());
    }

    private static boolean isTestScope(IdeaDependency dependency) {
        IdeaDependencyScope scope = dependency.getScope();
        return scope != null && "TEST".equalsIgnoreCase(scope.getScope());
    }

    private static String unitId(String projectPath, String kind) {
        return ":".equals(projectPath) ? ":" + kind : projectPath + ":" + kind;
    }

    private static String jdkHome(IdeaProject idea, IdeaModule module) {
        if (module.getJavaLanguageSettings() != null
                && module.getJavaLanguageSettings().getJdk() != null) {
            return module.getJavaLanguageSettings().getJdk().getJavaHome().getAbsolutePath();
        }
        if (idea.getJavaLanguageSettings() != null && idea.getJavaLanguageSettings().getJdk() != null) {
            return idea.getJavaLanguageSettings().getJdk().getJavaHome().getAbsolutePath();
        }
        return null;
    }
}
