package dev.blnt.beans.sidecar.protocol;

import com.fasterxml.jackson.databind.DeserializationFeature;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;

import java.io.IOException;
import java.io.UncheckedIOException;

/**
 * The wire: one JSON object per stdout line, reads from stdin handled
 * by the caller's loop. All emission funnels through {@link #send} —
 * stdout is the protocol channel and nothing else may write to it.
 *
 * <p>A process-wide instance exists because the SLF4J provider (which
 * the runtime instantiates before {@code main}) must reach the same
 * synchronized channel as everything else.
 */
public final class StdioTransport {

    public static final StdioTransport INSTANCE = new StdioTransport();

    private final ObjectMapper mapper =
            new ObjectMapper().disable(DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES);

    private StdioTransport() {}

    public Request parse(String line) throws IOException {
        return mapper.readValue(line, Request.class);
    }

    /** Bind a request's params tree to a duty's exact params record. */
    public <T> T bind(JsonNode params, Class<T> type) {
        try {
            return mapper.treeToValue(params == null ? mapper.createObjectNode() : params, type);
        } catch (IOException e) {
            throw new UncheckedIOException(e);
        }
    }

    public void respond(Integer id, Object result) {
        send(Response.ok(id, result));
    }

    public void respondError(Integer id, String message) {
        send(Response.failure(id, message));
    }

    public void notify(String method, Object params) {
        send(new Notification(method, params));
    }

    public void progress(String text) {
        notify("progress", new Notification.Progress(text));
    }

    public void log(String level, String logger, String text) {
        notify("log", new Notification.Log(level, logger, text));
    }

    private synchronized void send(Object message) {
        try {
            System.out.println(mapper.writeValueAsString(message));
        } catch (IOException e) {
            throw new UncheckedIOException(e);
        }
        System.out.flush();
    }
}
