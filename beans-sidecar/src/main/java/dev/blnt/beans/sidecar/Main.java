package dev.blnt.beans.sidecar;

import dev.blnt.beans.sidecar.gradle.GradleImport;
import dev.blnt.beans.sidecar.gradle.GradleImportParams;
import dev.blnt.beans.sidecar.protocol.InitializeResult;
import dev.blnt.beans.sidecar.protocol.Request;
import dev.blnt.beans.sidecar.protocol.StdioTransport;

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.util.Map;

/**
 * The beans sidecar: a single long-lived JVM process serving JVM-bound
 * duties to the beans engine over a JSON-Lines stdio protocol
 * (ADR-0031). Duties are dispatched by method name, namespaced per
 * build tool ({@code gradle/import}, later {@code maven/import},
 * {@code ap/run}); the exact wire shapes live in
 * {@link dev.blnt.beans.sidecar.protocol}.
 */
public final class Main {

    public static void main(String[] args) throws Exception {
        StdioTransport transport = StdioTransport.INSTANCE;
        BufferedReader in =
                new BufferedReader(new InputStreamReader(System.in, StandardCharsets.UTF_8));
        String line;
        while ((line = in.readLine()) != null) {
            if (line.isBlank()) {
                continue;
            }
            Request request;
            try {
                request = transport.parse(line);
            } catch (Exception e) {
                System.err.println("sidecar: unparseable line: " + e.getMessage());
                continue;
            }
            handle(transport, request);
        }
        // stdin closed: the client is gone or asked us to wind down.
    }

    private static void handle(StdioTransport transport, Request request) {
        try {
            switch (request.method()) {
                case "initialize" -> transport.respond(request.id(), InitializeResult.current());
                case "gradle/import" -> transport.respond(
                        request.id(),
                        GradleImport.run(
                                transport.bind(request.params(), GradleImportParams.class),
                                transport::progress));
                case "shutdown" -> {
                    // Map.of(), not new Object(): Jackson refuses to
                    // serialize property-less objects, and a throw here
                    // would skip the exit (caught by the first
                    // integration-test run).
                    transport.respond(request.id(), Map.of());
                    System.exit(0);
                }
                default -> transport.respondError(
                        request.id(), "unknown method: " + request.method());
            }
        } catch (Exception e) {
            transport.respondError(
                    request.id(), e.getClass().getSimpleName() + ": " + e.getMessage());
        }
    }

    private Main() {}
}
