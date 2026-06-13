package dev.blnt.beans.sidecar;

import com.google.gson.JsonObject;
import org.gradle.tooling.GradleConnector;
import org.gradle.tooling.ProjectConnection;
import org.gradle.tooling.model.idea.IdeaContentRoot;
import org.gradle.tooling.model.idea.IdeaDependency;
import org.gradle.tooling.model.idea.IdeaModule;
import org.gradle.tooling.model.idea.IdeaModuleDependency;
import org.gradle.tooling.model.idea.IdeaProject;
import org.gradle.tooling.model.idea.IdeaSingleEntryLibraryDependency;
import org.gradle.tooling.model.idea.IdeaSourceDirectory;

import java.io.File;
import java.util.function.Consumer;

/**
 * The {@code gradle/import} duty: extract a {@link WorkspaceModel}
 * through the Tooling API's stock {@link IdeaProject} model — no
 * injection into the user's build (ADR-0031; the custom tooling model
 * is the scheduled evolution when ap/run needs the processor path).
 *
 * <p>Params: {@code projectDir} (required); {@code gradleHome}
 * (optional — point at an installation instead of the project's
 * wrapper; used by tests and wrapper-less projects).
 */
final class GradleImport {

    static WorkspaceModel run(JsonObject params, Consumer<String> progress) {
        File projectDir = new File(params.get("projectDir").getAsString());
        GradleConnector connector = GradleConnector.newConnector().forProjectDirectory(projectDir);
        if (params.has("gradleHome")) {
            connector.useInstallation(new File(params.get("gradleHome").getAsString()));
        }

        progress.accept("Connecting to Gradle build at " + projectDir);
        try (ProjectConnection connection = connector.connect()) {
            IdeaProject idea = connection
                    .model(IdeaProject.class)
                    .addProgressListener(
                            (org.gradle.tooling.ProgressListener)
                                    event -> progress.accept(event.getDescription()))
                    .get();
            return toWorkspaceModel(idea);
        }
    }

    private static WorkspaceModel toWorkspaceModel(IdeaProject idea) {
        WorkspaceModel model = new WorkspaceModel();
        for (IdeaModule ideaModule : idea.getModules()) {
            WorkspaceModel.Module module = new WorkspaceModel.Module();
            module.name = ideaModule.getName();

            for (IdeaContentRoot root : ideaModule.getContentRoots()) {
                for (IdeaSourceDirectory src : root.getSourceDirectories()) {
                    (src.isGenerated() ? module.generatedSourceRoots : module.sourceRoots)
                            .add(src.getDirectory().getAbsolutePath());
                }
                for (IdeaSourceDirectory test : root.getTestDirectories()) {
                    (test.isGenerated() ? module.generatedSourceRoots : module.testSourceRoots)
                            .add(test.getDirectory().getAbsolutePath());
                }
            }

            for (IdeaDependency dep : ideaModule.getDependencies()) {
                if (dep instanceof IdeaSingleEntryLibraryDependency lib) {
                    module.compileClasspath.add(lib.getFile().getAbsolutePath());
                } else if (dep instanceof IdeaModuleDependency moduleDep) {
                    module.moduleDependencies.add(moduleDep.getTargetModuleName());
                }
            }

            if (ideaModule.getJavaLanguageSettings() != null
                    && ideaModule.getJavaLanguageSettings().getJdk() != null) {
                module.jdkHome =
                        ideaModule.getJavaLanguageSettings().getJdk().getJavaHome().getAbsolutePath();
            } else if (idea.getJavaLanguageSettings() != null
                    && idea.getJavaLanguageSettings().getJdk() != null) {
                module.jdkHome =
                        idea.getJavaLanguageSettings().getJdk().getJavaHome().getAbsolutePath();
            }

            model.modules.add(module);
        }
        return model;
    }

    private GradleImport() {}
}
