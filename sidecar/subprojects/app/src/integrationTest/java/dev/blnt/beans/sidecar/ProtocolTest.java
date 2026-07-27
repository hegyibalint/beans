package dev.blnt.beans.sidecar;

import com.fasterxml.jackson.databind.JsonNode;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.nio.file.Path;
import java.time.Duration;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

class ProtocolTest {

    private static final Duration QUICK = Duration.ofSeconds(15);

    @Test
    void handshake_reports_the_protocol_version_and_the_adapters_found() throws Exception {
        try (SidecarProcess sidecar = new SidecarProcess()) {
            sidecar.send("{\"id\":1,\"method\":\"initialize\"}");
            JsonNode response = sidecar.awaitResponse(QUICK);

            assertEquals(1, response.get("id").asInt());
            JsonNode result = response.get("result");
            assertEquals(1, result.get("protocolVersion").asInt());
            assertNotNull(result.get("javaHome").asText());

            List<String> adapters = new ArrayList<>();
            result.get("adapters").forEach(a -> adapters.add(a.asText()));
            assertTrue(adapters.contains("gradle"), "expected the gradle adapter, got: " + adapters);
        }
    }

    @Test
    void an_unknown_method_is_an_error_the_process_survives() throws Exception {
        try (SidecarProcess sidecar = new SidecarProcess()) {
            sidecar.send("{\"id\":1,\"method\":\"no/such-method\"}");
            JsonNode error = sidecar.awaitResponse(QUICK);
            assertEquals(1, error.get("id").asInt());
            assertTrue(error.get("error").get("message").asText().contains("no/such-method"));

            sidecar.send("{\"id\":2,\"method\":\"initialize\"}");
            assertNotNull(sidecar.awaitResponse(QUICK).get("result"));
        }
    }

    @Test
    void an_unparseable_line_is_skipped_and_the_process_survives() throws Exception {
        try (SidecarProcess sidecar = new SidecarProcess()) {
            sidecar.send("this is not json");
            sidecar.send("{\"id\":1,\"method\":\"initialize\"}");
            JsonNode response = sidecar.awaitResponse(QUICK);
            assertEquals(1, response.get("id").asInt());
            assertNotNull(response.get("result"));
        }
    }

    @Test
    void importing_a_directory_no_adapter_claims_is_an_error(@TempDir Path empty) throws Exception {
        try (SidecarProcess sidecar = new SidecarProcess()) {
            sidecar.sendImport(1, empty, Map.of());
            JsonNode response = sidecar.awaitResponse(QUICK);
            assertTrue(response.get("error").get("message").asText().contains("no adapter"));
        }
    }

    @Test
    void shutdown_replies_before_the_process_exits() throws Exception {
        try (SidecarProcess sidecar = new SidecarProcess()) {
            sidecar.send("{\"id\":1,\"method\":\"shutdown\"}");
            JsonNode response = sidecar.awaitResponse(QUICK);
            assertEquals(1, response.get("id").asInt());
            assertTrue(sidecar.exited(QUICK), "process should exit after shutdown");
            assertEquals(0, sidecar.exitCode());
        }
    }
}
