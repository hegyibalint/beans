package dev.blnt.beans.sidecar.gradle;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;

/** Supplies only the small provider JAR to the target build's init-script classpath. */
final class ModelFiles implements AutoCloseable {
    private final Path directory;

    ModelFiles() throws IOException {
        directory = Files.createTempDirectory("beans-gradle-model-");
        try {
            extract("model.jar");
            extract("init.gradle");
        } catch (IOException error) {
            try {
                close();
            } catch (IOException cleanup) {
                error.addSuppressed(cleanup);
            }
            throw error;
        }
    }

    Path jar() { return directory.resolve("model.jar"); }
    Path script() { return directory.resolve("init.gradle"); }

    private void extract(String name) throws IOException {
        try (var input = ModelFiles.class.getResourceAsStream(name)) {
            if (input == null) {
                throw new IOException("Missing Gradle model resource: " + name);
            }
            Files.copy(input, directory.resolve(name));
        }
    }

    @Override
    public void close() throws IOException {
        Files.deleteIfExists(script());
        Files.deleteIfExists(jar());
        Files.delete(directory);
    }
}
