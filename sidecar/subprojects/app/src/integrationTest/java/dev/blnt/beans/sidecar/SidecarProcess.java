package dev.blnt.beans.sidecar;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;

import java.io.BufferedReader;
import java.io.BufferedWriter;
import java.io.InputStreamReader;
import java.io.OutputStreamWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.time.Duration;
import java.util.List;
import java.util.Map;
import java.util.concurrent.BlockingQueue;
import java.util.concurrent.CopyOnWriteArrayList;
import java.util.concurrent.LinkedBlockingQueue;
import java.util.concurrent.TimeUnit;

/** Plays the Rust client's part against the real jar over real stdio. */
final class SidecarProcess implements AutoCloseable {

    private static final ObjectMapper MAPPER = new ObjectMapper();

    private final Process process;
    private final BufferedWriter stdin;
    private final BlockingQueue<JsonNode> responses = new LinkedBlockingQueue<>();
    final List<JsonNode> notifications = new CopyOnWriteArrayList<>();

    SidecarProcess() throws Exception {
        Path jar = Path.of(System.getProperty("test.sidecarJar"));
        Path java = Path.of(System.getProperty("java.home"), "bin", "java");
        process = new ProcessBuilder(java.toString(), "-jar", jar.toString())
                // The sidecar points System.out at stderr, so stderr can carry
                // real volume. Inherit it rather than let an undrained pipe fill.
                .redirectError(ProcessBuilder.Redirect.INHERIT)
                .start();
        stdin = new BufferedWriter(
                new OutputStreamWriter(process.getOutputStream(), StandardCharsets.UTF_8));

        Thread reader = new Thread(() -> {
            try (BufferedReader out = new BufferedReader(
                    new InputStreamReader(process.getInputStream(), StandardCharsets.UTF_8))) {
                String line;
                while ((line = out.readLine()) != null) {
                    JsonNode node = MAPPER.readTree(line);
                    if (node.has("id")) {
                        responses.put(node);
                    } else {
                        notifications.add(node);
                    }
                }
            } catch (Exception ignored) {
                // stream closed with the process; tests assert on what arrived
            }
        });
        reader.setDaemon(true);
        reader.start();
    }

    void send(String json) throws Exception {
        stdin.write(json);
        stdin.write('\n');
        stdin.flush();
    }

    /** Built through the mapper so no test has to escape a path by hand. */
    void sendImport(int id, Path workspaceRoot, Map<String, String> options) throws Exception {
        ObjectNode params = MAPPER.createObjectNode();
        params.put("workspaceRoot", workspaceRoot.toString());
        if (!options.isEmpty()) {
            ObjectNode node = params.putObject("options");
            options.forEach(node::put);
        }
        ObjectNode request = MAPPER.createObjectNode();
        request.put("id", id);
        request.put("method", "build/import");
        request.set("params", params);
        send(MAPPER.writeValueAsString(request));
    }

    JsonNode awaitResponse(Duration timeout) throws Exception {
        JsonNode response = responses.poll(timeout.toMillis(), TimeUnit.MILLISECONDS);
        if (response == null) {
            throw new AssertionError(
                    "no response within " + timeout + "; notifications so far: " + notifications);
        }
        return response;
    }

    boolean exited(Duration timeout) throws InterruptedException {
        return process.waitFor(timeout.toMillis(), TimeUnit.MILLISECONDS);
    }

    int exitCode() {
        return process.exitValue();
    }

    @Override
    public void close() throws Exception {
        stdin.close();
        if (!process.waitFor(5, TimeUnit.SECONDS)) {
            process.destroyForcibly();
        }
    }
}
