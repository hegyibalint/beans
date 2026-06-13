package dev.blnt.beans.sidecar;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;

import java.io.BufferedReader;
import java.io.BufferedWriter;
import java.io.InputStreamReader;
import java.io.OutputStreamWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.time.Duration;
import java.util.List;
import java.util.concurrent.BlockingQueue;
import java.util.concurrent.CopyOnWriteArrayList;
import java.util.concurrent.LinkedBlockingQueue;
import java.util.concurrent.TimeUnit;

/**
 * Test harness around the real sidecar: spawns the fat jar with the
 * test JVM, writes request lines, reads the protocol stream. Plays the
 * Rust client's part the way the LSP role-play harness plays the
 * editor's.
 */
final class SidecarProcess implements AutoCloseable {

    private static final ObjectMapper MAPPER = new ObjectMapper();

    private final Process process;
    private final BufferedWriter stdin;
    private final BlockingQueue<JsonNode> messages = new LinkedBlockingQueue<>();
    final List<JsonNode> notifications = new CopyOnWriteArrayList<>();

    SidecarProcess() throws Exception {
        Path jar = Path.of(System.getProperty("test.sidecarJar"));
        Path java = Path.of(System.getProperty("java.home"), "bin", "java");
        process = new ProcessBuilder(java.toString(), "-jar", jar.toString()).start();
        stdin = new BufferedWriter(
                new OutputStreamWriter(process.getOutputStream(), StandardCharsets.UTF_8));

        Thread reader = new Thread(() -> {
            try (BufferedReader out = new BufferedReader(
                    new InputStreamReader(process.getInputStream(), StandardCharsets.UTF_8))) {
                String line;
                while ((line = out.readLine()) != null) {
                    JsonNode node = MAPPER.readTree(line);
                    if (node.has("id")) {
                        messages.put(node);
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

    /** Next response carrying an id (responses are ordered per protocol). */
    JsonNode awaitResponse(Duration timeout) throws Exception {
        JsonNode response = messages.poll(timeout.toMillis(), TimeUnit.MILLISECONDS);
        if (response == null) {
            throw new AssertionError("no response within " + timeout
                    + "; notifications so far: " + notifications);
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
