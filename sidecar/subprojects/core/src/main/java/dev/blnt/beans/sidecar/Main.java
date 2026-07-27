package dev.blnt.beans.sidecar;

import dev.blnt.beans.sidecar.model.Workspace;
import dev.blnt.beans.sidecar.protocol.InitializeResult;
import dev.blnt.beans.sidecar.protocol.Request;
import dev.blnt.beans.sidecar.protocol.StdioTransport;

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.util.List;
import java.util.Map;
import java.util.ServiceLoader;
import java.util.function.Consumer;

/**
 * The sidecar: a JVM process that answers build questions over a line-delimited
 * JSON protocol on stdio and exits when its client goes away.
 */
public final class Main {

    public static void main(String[] args) throws Exception {
        // Descriptor 1 belongs to the transport. Anything that reaches for
        // System.out lands on stderr instead of corrupting a message.
        System.setOut(System.err);

        StdioTransport transport = StdioTransport.INSTANCE;
        List<BuildAdapter> adapters = ServiceLoader.load(BuildAdapter.class).stream()
                .map(ServiceLoader.Provider::get)
                .toList();

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
            handle(transport, adapters, request);
        }
        // stdin closed: the client is gone.
    }

    private static void handle(
            StdioTransport transport, List<BuildAdapter> adapters, Request request) {
        try {
            switch (request.method()) {
                case "initialize" -> transport.respond(
                        request.id(),
                        InitializeResult.of(adapters.stream().map(BuildAdapter::name).toList()));
                case "build/import" -> transport.respond(
                        request.id(),
                        importWorkspace(
                                adapters,
                                transport.bind(request.params(), ImportParams.class),
                                transport::progress));
                case "shutdown" -> {
                    // Map.of() rather than a bare object: Jackson refuses to
                    // serialize a property-less one, and throwing here would
                    // skip the exit.
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

    private static Workspace importWorkspace(
            List<BuildAdapter> adapters, ImportParams params, Consumer<String> progress) {
        Path root = Path.of(params.workspaceRoot());
        for (BuildAdapter adapter : adapters) {
            if (adapter.accepts(root)) {
                return adapter.importWorkspace(params, progress);
            }
        }
        throw new IllegalArgumentException("no adapter recognises the build at " + root);
    }

    private Main() {}
}
