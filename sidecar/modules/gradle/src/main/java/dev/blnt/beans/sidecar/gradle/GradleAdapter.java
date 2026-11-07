package dev.blnt.beans.sidecar.gradle;

import dev.blnt.beans.gradle.model.CollectModels;
import dev.blnt.beans.gradle.model.SourceSetModel;
import dev.blnt.beans.gradle.model.ProjectModel;
import dev.blnt.beans.sidecar.Handler;
import dev.blnt.beans.sidecar.Module;
import dev.blnt.beans.sidecar.model.CompilationScope;
import dev.blnt.beans.sidecar.model.ResourceFilter;
import org.gradle.tooling.GradleConnector;
import org.gradle.tooling.ProjectConnection;
import org.gradle.tooling.events.OperationType;
import org.gradle.tooling.events.StartEvent;

import java.io.File;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.EnumSet;
import java.util.HashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.CompletionStage;
import java.util.concurrent.Executor;
import java.util.concurrent.Executors;
import java.util.function.Consumer;

public final class GradleAdapter implements Module {
    private final Executor imports = Executors.newSingleThreadExecutor(GradleAdapter::importThread);

    @Override
    public void register(Handler handler) {
        handler.register("gradle/import", ImportParams.class, this::importWorkspace);
    }

    private CompletionStage<List<CompilationScope>> importWorkspace(ImportParams params) {
        return CompletableFuture.supplyAsync(() -> importWorkspace(params, System.err::println), imports);
    }

    public List<CompilationScope> importWorkspace(ImportParams params, Consumer<String> progress) {
        return toScopes(importModel(params, progress));
    }

    public List<ProjectModel> importModel(ImportParams params, Consumer<String> progress) {
        File projectDir = new File(params.workspaceRoot());
        GradleConnector connector = GradleConnector.newConnector().forProjectDirectory(projectDir);
        if (params.gradleHome() != null) {
            connector.useInstallation(new File(params.gradleHome()));
        }

        // Register a project-local model provider without changing build files or running build tasks.
        progress.accept("Connecting to Gradle build at " + projectDir);
        try (var files = new ModelFiles(); ProjectConnection connection = connector.connect()) {
            return connection.action(new CollectModels())
                    .withArguments("--quiet", "--init-script", files.script().toString(),
                            "-Dbeans.gradle.model.jar=" + files.jar(),
                            "-Dorg.gradle.java.installations.auto-download=false")
                    .setStandardOutput(System.err)
                    .setStandardError(System.err)
                    .addProgressListener(event -> {
                        if (event instanceof StartEvent) {
                            progress.accept(event.getDescriptor().getDisplayName());
                        }
                    }, EnumSet.of(OperationType.PROJECT_CONFIGURATION, OperationType.BUILD_PHASE))
                    .run();
        } catch (IOException error) {
            throw new UncheckedIOException(error);
        }
    }

    private static List<CompilationScope> toScopes(List<ProjectModel> projects) {
        // Relate known class directories to their source sets without assuming a language or "main".
        Map<String, Set<ResourceFilter>> outputs = new HashMap<>();
        for (ProjectModel project : projects) {
            for (SourceSetModel sourceSet : project.getSourceSets()) {
                for (String directory : sourceSet.getClassesDirectories()) {
                    outputs.computeIfAbsent(directory, ignored -> new LinkedHashSet<>())
                            .addAll(sourceInputs(sourceSet));
                }
            }
        }

        // Preserve file selection rather than approximating Gradle's arbitrary file predicates.
        List<CompilationScope> scopes = new ArrayList<>();
        for (ProjectModel project : projects) {
            for (SourceSetModel sourceSet : project.getSourceSets()) {
                Set<ResourceFilter> dependencies = new LinkedHashSet<>(jdkInputs(project.getJavaHome()));
                for (String entry : sourceSet.getCompileClasspath()) {
                    dependencies.add(file(entry));
                    var sources = outputs.get(entry);
                    if (sources != null) {
                        dependencies.addAll(sources);
                    }
                }
                scopes.add(new CompilationScope(sourceInputs(sourceSet), List.copyOf(dependencies)));
            }
        }
        return scopes;
    }

    private static List<ResourceFilter> sourceInputs(SourceSetModel sourceSet) {
        List<ResourceFilter> inputs = new ArrayList<>();
        for (String source : sourceSet.getSourceFiles()) {
            inputs.add(file(source));
        }
        return inputs;
    }

    private static List<ResourceFilter> jdkInputs(String home) {
        if (home == null) {
            throw new IllegalStateException("No default JVM toolchain reported by the project");
        }
        for (String runtime : List.of("lib/modules", "jre/lib/rt.jar", "lib/rt.jar")) {
            File artifact = new File(home, runtime);
            if (!artifact.isFile()) {
                continue;
            }
            List<ResourceFilter> inputs = new ArrayList<>();
            inputs.add(file(artifact.toString()));
            File tools = new File(home, "lib/tools.jar");
            if (!runtime.equals("lib/modules") && tools.isFile()) {
                inputs.add(file(tools.toString()));
            }
            return inputs;
        }
        throw new IllegalStateException("No supported JDK runtime found in " + home);
    }

    private static ResourceFilter file(String path) {
        return new ResourceFilter.File(Path.of(path).toAbsolutePath().normalize().toString());
    }

    private static Thread importThread(Runnable task) {
        var thread = new Thread(task, "beans-gradle-import");
        // Handler drains the returned stages at EOF; idle workers must not keep the JVM alive.
        thread.setDaemon(true);
        return thread;
    }
}
