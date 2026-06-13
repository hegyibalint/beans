package dev.blnt.beans.sidecar;

import com.fasterxml.jackson.databind.JsonNode;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.nio.file.Files;
import java.nio.file.Path;
import java.time.Duration;
import java.util.List;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

/**
 * Protocol-level tests against the real fat jar over real stdio — the
 * sidecar's executable contract with the Rust client.
 */
class SidecarIntegrationTest {

    private static final Duration QUICK = Duration.ofSeconds(15);
    /** Imports may cold-start a daemon on CI; be generous. */
    private static final Duration IMPORT = Duration.ofMinutes(3);

    @Test
    void initialize_reports_protocol_version_and_capabilities() throws Exception {
        try (SidecarProcess sidecar = new SidecarProcess()) {
            sidecar.send("{\"id\":1,\"method\":\"initialize\"}");
            JsonNode response = sidecar.awaitResponse(QUICK);

            assertEquals(1, response.get("id").asInt());
            JsonNode result = response.get("result");
            assertEquals(1, result.get("protocolVersion").asInt());
            assertTrue(result.get("capabilities").get("gradle/import").asBoolean());
            assertNotNull(result.get("javaHome").asText());
        }
    }

    @Test
    void unknown_method_yields_error_and_process_keeps_serving() throws Exception {
        try (SidecarProcess sidecar = new SidecarProcess()) {
            sidecar.send("{\"id\":1,\"method\":\"no/such-duty\"}");
            JsonNode error = sidecar.awaitResponse(QUICK);
            assertEquals(1, error.get("id").asInt());
            assertTrue(error.get("error").get("message").asText().contains("no/such-duty"));

            sidecar.send("{\"id\":2,\"method\":\"initialize\"}");
            JsonNode ok = sidecar.awaitResponse(QUICK);
            assertEquals(2, ok.get("id").asInt());
            assertNotNull(ok.get("result"));
        }
    }

    @Test
    void unparseable_line_is_ignored_and_process_keeps_serving() throws Exception {
        try (SidecarProcess sidecar = new SidecarProcess()) {
            sidecar.send("this is not json");
            sidecar.send("{\"id\":1,\"method\":\"initialize\"}");
            JsonNode ok = sidecar.awaitResponse(QUICK);
            assertEquals(1, ok.get("id").asInt());
            assertNotNull(ok.get("result"));
        }
    }

    @Test
    void shutdown_responds_then_exits_cleanly() throws Exception {
        try (SidecarProcess sidecar = new SidecarProcess()) {
            sidecar.send("{\"id\":1,\"method\":\"shutdown\"}");
            JsonNode response = sidecar.awaitResponse(QUICK);
            assertEquals(1, response.get("id").asInt());
            assertTrue(sidecar.exited(QUICK), "process should exit after shutdown");
            assertEquals(0, sidecar.exitCode());
        }
    }

    @Test
    void gradle_import_produces_workspace_model(@TempDir Path projectDir) throws Exception {
        // Self-contained fixture: configured (non-conventional) source
        // root, no external dependencies — offline-safe.
        Files.writeString(
                projectDir.resolve("settings.gradle.kts"), "rootProject.name = \"it-fixture\"\n");
        Files.writeString(
                projectDir.resolve("build.gradle.kts"),
                """
                plugins { java }
                sourceSets { main { java.srcDirs("src") } }
                """);
        Path src = projectDir.resolve("src/com/example");
        Files.createDirectories(src);
        Files.writeString(
                src.resolve("App.java"), "package com.example;\npublic class App {}\n");

        String gradleHome = System.getProperty("test.gradleHome");
        assertNotNull(gradleHome, "test.gradleHome must be set by the build");

        try (SidecarProcess sidecar = new SidecarProcess()) {
            sidecar.send(String.format(
                    "{\"id\":1,\"method\":\"gradle/import\",\"params\":{"
                            + "\"projectDir\":\"%s\",\"gradleHome\":\"%s\"}}",
                    projectDir, gradleHome));
            JsonNode response = sidecar.awaitResponse(IMPORT);

            assertEquals(1, response.get("id").asInt());
            JsonNode modules = response.get("result").get("modules");
            assertEquals(1, modules.size());

            JsonNode module = modules.get(0);
            assertEquals("it-fixture", module.get("name").asText());

            List<String> roots = new java.util.ArrayList<>();
            module.get("sourceRoots").forEach(r -> roots.add(r.asText()));
            assertTrue(
                    roots.stream().anyMatch(r -> r.endsWith("/src") || r.endsWith("\\src")),
                    "configured root 'src' must be reported, got: " + roots);

            assertEquals(0, module.get("compileClasspath").size(), "fixture has no dependencies");
            assertEquals(0, module.get("moduleDependencies").size());

            assertFalse(
                    sidecar.notifications.isEmpty(),
                    "import should narrate progress notifications");
        }
    }
}
