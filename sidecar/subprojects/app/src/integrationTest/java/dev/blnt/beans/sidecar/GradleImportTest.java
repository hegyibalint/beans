package dev.blnt.beans.sidecar;

import com.fasterxml.jackson.databind.JsonNode;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.nio.file.Files;
import java.nio.file.Path;
import java.time.Duration;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

class GradleImportTest {

    /** A cold daemon on CI is slow; be generous. */
    private static final Duration IMPORT = Duration.ofMinutes(3);

    @Test
    void a_source_set_becomes_a_unit_holding_the_directory_it_selects(@TempDir Path projectDir)
            throws Exception {
        JsonNode workspace = importFixture(projectDir);

        assertEquals("gradle", workspace.get("tool").asText());

        JsonNode main = unit(workspace, ":main");
        assertEquals(1, main.get("sources").size());

        JsonNode selector = main.get("sources").get(0);
        assertEquals("tree", selector.get("kind").asText());
        assertFalse(selector.get("generated").asBoolean());
        assertTrue(
                selector.get("base").asText().endsWith("mainSrc"),
                "configured directory must be reported, got: " + selector.get("base").asText());

        assertEquals(0, main.get("classpath").size(), "the fixture declares no dependencies");
    }

    @Test
    void main_and_test_are_separate_units_and_test_depends_on_main(@TempDir Path projectDir)
            throws Exception {
        JsonNode workspace = importFixture(projectDir);

        JsonNode test = unit(workspace, ":test");
        assertTrue(
                test.get("sources").get(0).get("base").asText().endsWith("testSrc"),
                "the test unit must select the test directory");

        List<String> dependsOn = new ArrayList<>();
        test.get("dependsOn").forEach(d -> dependsOn.add(d.asText()));
        assertTrue(dependsOn.contains(":main"), "expected a dependency on :main, got: " + dependsOn);
    }

    @Test
    void an_import_narrates_its_progress(@TempDir Path projectDir) throws Exception {
        try (SidecarProcess sidecar = new SidecarProcess()) {
            writeFixture(projectDir);
            sidecar.sendImport(1, projectDir, gradleHome());
            sidecar.awaitResponse(IMPORT);

            assertFalse(sidecar.notifications.isEmpty(), "an import should report progress");
        }
    }

    /**
     * Two source sets in non-default, non-overlapping directories: the layout is
     * configured rather than conventional, so a passing assertion cannot come
     * from us having guessed Gradle's defaults. No dependencies, so it is
     * offline-safe.
     */
    private static void writeFixture(Path projectDir) throws Exception {
        Files.writeString(
                projectDir.resolve("settings.gradle.kts"), "rootProject.name = \"it-fixture\"\n");
        Files.writeString(
                projectDir.resolve("build.gradle.kts"),
                """
                plugins { java }
                sourceSets {
                    main { java.setSrcDirs(listOf("mainSrc")) }
                    test { java.setSrcDirs(listOf("testSrc")) }
                }
                """);
        Path main = projectDir.resolve("mainSrc/com/example");
        Files.createDirectories(main);
        Files.writeString(main.resolve("App.java"), "package com.example;\npublic class App {}\n");
        Path test = projectDir.resolve("testSrc/com/example");
        Files.createDirectories(test);
        Files.writeString(
                test.resolve("AppTest.java"), "package com.example;\npublic class AppTest {}\n");
    }

    private static JsonNode importFixture(Path projectDir) throws Exception {
        try (SidecarProcess sidecar = new SidecarProcess()) {
            writeFixture(projectDir);
            sidecar.sendImport(1, projectDir, gradleHome());
            JsonNode response = sidecar.awaitResponse(IMPORT);
            assertNotNull(response.get("result"), "import failed: " + response.get("error"));
            return response.get("result");
        }
    }

    /** The fixture has no wrapper, so it borrows the Gradle running this build. */
    private static Map<String, String> gradleHome() {
        String home = System.getProperty("test.gradleHome");
        assertNotNull(home, "test.gradleHome must be set by the build");
        return Map.of("gradleHome", home);
    }

    private static JsonNode unit(JsonNode workspace, String id) {
        List<String> seen = new ArrayList<>();
        for (JsonNode unit : workspace.get("units")) {
            if (id.equals(unit.get("id").asText())) {
                return unit;
            }
            seen.add(unit.get("id").asText());
        }
        throw new AssertionError("no unit " + id + "; found: " + seen);
    }
}
