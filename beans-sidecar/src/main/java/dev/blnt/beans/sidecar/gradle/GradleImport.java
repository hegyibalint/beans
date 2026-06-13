package dev.blnt.beans.sidecar.gradle;

import dev.blnt.beans.sidecar.model.WorkspaceModel;
import dev.blnt.beans.sidecar.model.WorkspaceModule;
import org.gradle.tooling.GradleConnector;
import org.gradle.tooling.ProjectConnection;
import org.gradle.tooling.events.OperationType;
import org.gradle.tooling.events.StartEvent;
import org.gradle.tooling.model.idea.IdeaContentRoot;
import org.gradle.tooling.model.idea.IdeaDependency;
import org.gradle.tooling.model.idea.IdeaModule;
import org.gradle.tooling.model.idea.IdeaModuleDependency;
import org.gradle.tooling.model.idea.IdeaProject;
import org.gradle.tooling.model.idea.IdeaSingleEntryLibraryDependency;
import org.gradle.tooling.model.idea.IdeaSourceDirectory;

import java.io.File;
import java.util.ArrayList;
import java.util.EnumSet;
import java.util.List;
import java.util.function.Consumer;

/**
 * The {@code gradle/import} duty: extract a {@link WorkspaceModel}
 * through the Tooling API's stock {@link IdeaProject} model — no
 * injection into the user's build (ADR-0031; the custom tooling model
 * is the scheduled evolution when ap/run needs the processor path).
 */
public final class GradleImport {

    public static WorkspaceModel run(GradleImportParams params, Consumer<String> progress) {
        File projectDir = new File(params.projectDir());
        GradleConnector connector = GradleConnector.newConnector().forProjectDirectory(projectDir);
        if (params.gradleHome() != null) {
            connector.useInstallation(new File(params.gradleHome()));
        }

        progress.accept("Connecting to Gradle build at " + projectDir);
        try (ProjectConnection connection = connector.connect()) {
            IdeaProject idea = connection
                    .model(IdeaProject.class)
                    // Quiet daemon logging; our progress narration comes
                    // from typed events, not log output.
                    .withArguments("--quiet")
                    // Narrate only phase-level operations. The unfiltered
                    // legacy listener fires for every dependency download
                    // and inner operation — a firehose no UI wants.
                    .addProgressListener(
                            event -> {
                                if (event instanceof StartEvent) {
                                    progress.accept(event.getDescriptor().getDisplayName());
                                }
                            },
                            EnumSet.of(
                                    OperationType.PROJECT_CONFIGURATION,
                                    OperationType.BUILD_PHASE))
                    .get();
            return toWorkspaceModel(idea);
        }
    }

    private static WorkspaceModel toWorkspaceModel(IdeaProject idea) {
        List<WorkspaceModule> modules = new ArrayList<>();
        for (IdeaModule ideaModule : idea.getModules()) {
            List<String> sourceRoots = new ArrayList<>();
            List<String> testSourceRoots = new ArrayList<>();
            List<String> generatedSourceRoots = new ArrayList<>();
            List<String> compileClasspath = new ArrayList<>();
            List<String> moduleDependencies = new ArrayList<>();

            for (IdeaContentRoot root : ideaModule.getContentRoots()) {
                for (IdeaSourceDirectory src : root.getSourceDirectories()) {
                    (src.isGenerated() ? generatedSourceRoots : sourceRoots)
                            .add(src.getDirectory().getAbsolutePath());
                }
                for (IdeaSourceDirectory test : root.getTestDirectories()) {
                    (test.isGenerated() ? generatedSourceRoots : testSourceRoots)
                            .add(test.getDirectory().getAbsolutePath());
                }
            }

            for (IdeaDependency dep : ideaModule.getDependencies()) {
                if (dep instanceof IdeaSingleEntryLibraryDependency lib) {
                    compileClasspath.add(lib.getFile().getAbsolutePath());
                } else if (dep instanceof IdeaModuleDependency moduleDep) {
                    moduleDependencies.add(moduleDep.getTargetModuleName());
                }
            }

            modules.add(new WorkspaceModule(
                    ideaModule.getName(),
                    sourceRoots,
                    testSourceRoots,
                    generatedSourceRoots,
                    compileClasspath,
                    moduleDependencies,
                    jdkHome(idea, ideaModule)));
        }
        return new WorkspaceModel(modules);
    }

    private static String jdkHome(IdeaProject idea, IdeaModule module) {
        if (module.getJavaLanguageSettings() != null
                && module.getJavaLanguageSettings().getJdk() != null) {
            return module.getJavaLanguageSettings().getJdk().getJavaHome().getAbsolutePath();
        }
        if (idea.getJavaLanguageSettings() != null
                && idea.getJavaLanguageSettings().getJdk() != null) {
            return idea.getJavaLanguageSettings().getJdk().getJavaHome().getAbsolutePath();
        }
        return null;
    }

    private GradleImport() {}
}
