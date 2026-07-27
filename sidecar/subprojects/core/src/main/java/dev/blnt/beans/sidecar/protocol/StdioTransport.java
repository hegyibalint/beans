package dev.blnt.beans.sidecar.protocol;

import com.fasterxml.jackson.databind.DeserializationFeature;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;

import java.io.FileDescriptor;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.PrintStream;
import java.io.UncheckedIOException;
import java.nio.charset.StandardCharsets;

/**
 * The wire: one JSON object per line on file descriptor 1.
 *
 * <p>It writes to the descriptor rather than to {@code System.out} on purpose.
 * {@code System.out} is a global any library can reassign or scribble on, and a
 * stray {@code println} in the middle of a message is a protocol corruption that
 * reads like a parse bug. {@link dev.blnt.beans.sidecar.Main} points
 * {@code System.out} at stderr so this stays the only writer.
 *
 * <p>Process-wide, because the SLF4J provider is instantiated by the runtime
 * before {@code main} and must reach the same synchronized channel.
 */
public final class StdioTransport {

    public static final StdioTransport INSTANCE = new StdioTransport();

    private final ObjectMapper mapper =
            new ObjectMapper().disable(DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES);

    private final PrintStream out =
            new PrintStream(new FileOutputStream(FileDescriptor.out), false, StandardCharsets.UTF_8);

    private StdioTransport() {}

    public Request parse(String line) throws IOException {
        return mapper.readValue(line, Request.class);
    }

    /** Bind a request's params tree to a handler's exact params record. */
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
            out.println(mapper.writeValueAsString(message));
        } catch (IOException e) {
            throw new UncheckedIOException(e);
        }
        out.flush();
    }
}
