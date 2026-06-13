package dev.blnt.beans.sidecar;

import com.google.gson.Gson;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;

/**
 * The beans sidecar: a single long-lived JVM process serving JVM-bound
 * duties to the beans engine over a JSON-Lines stdio protocol
 * (ADR-0031).
 *
 * <p>Wire format: one JSON object per line. Requests carry
 * {@code {id, method, params}}; responses {@code {id, result}} or
 * {@code {id, error}}; id-less objects are notifications (progress,
 * log) the client renders but never awaits.
 *
 * <p>stdout is the protocol channel and nothing else; all writes go
 * through {@link #send(Object)} which synchronizes line emission.
 * Diagnostics that must not enter the protocol go to stderr.
 */
public final class Main {

    static final Gson GSON = new Gson();
    private static final Object STDOUT_LOCK = new Object();

    public static void main(String[] args) throws Exception {
        BufferedReader in =
                new BufferedReader(new InputStreamReader(System.in, StandardCharsets.UTF_8));
        String line;
        while ((line = in.readLine()) != null) {
            if (line.isBlank()) {
                continue;
            }
            JsonObject msg;
            try {
                msg = JsonParser.parseString(line).getAsJsonObject();
            } catch (RuntimeException e) {
                System.err.println("sidecar: unparseable line: " + e.getMessage());
                continue;
            }
            handle(msg);
        }
        // stdin closed: the client is gone or asked us to wind down.
    }

    private static void handle(JsonObject msg) {
        String method = msg.has("method") ? msg.get("method").getAsString() : "";
        Integer id = msg.has("id") ? msg.get("id").getAsInt() : null;
        JsonObject params =
                msg.has("params") ? msg.getAsJsonObject("params") : new JsonObject();

        try {
            switch (method) {
                case "initialize" -> respond(id, Handshake.capabilities());
                case "gradle/import" -> respond(id, GradleImport.run(params, Main::notifyProgress));
                case "shutdown" -> {
                    respond(id, new JsonObject());
                    System.exit(0);
                }
                default -> error(id, "unknown method: " + method);
            }
        } catch (Exception e) {
            error(id, e.getClass().getSimpleName() + ": " + e.getMessage());
        }
    }

    static void notifyLog(String level, String logger, String text) {
        JsonObject note = new JsonObject();
        note.addProperty("method", "log");
        JsonObject params = new JsonObject();
        params.addProperty("level", level);
        params.addProperty("logger", logger);
        params.addProperty("text", text);
        note.add("params", params);
        send(note);
    }

    static void notifyProgress(String text) {
        JsonObject note = new JsonObject();
        note.addProperty("method", "progress");
        JsonObject params = new JsonObject();
        params.addProperty("text", text);
        note.add("params", params);
        send(note);
    }

    private static void respond(Integer id, Object result) {
        JsonObject out = new JsonObject();
        if (id != null) {
            out.addProperty("id", id);
        }
        out.add("result", GSON.toJsonTree(result));
        send(out);
    }

    private static void error(Integer id, String message) {
        JsonObject out = new JsonObject();
        if (id != null) {
            out.addProperty("id", id);
        }
        JsonObject err = new JsonObject();
        err.addProperty("message", message);
        out.add("error", err);
        send(out);
    }

    private static void send(Object json) {
        synchronized (STDOUT_LOCK) {
            System.out.println(GSON.toJson(json));
            System.out.flush();
        }
    }

    private Main() {}
}
