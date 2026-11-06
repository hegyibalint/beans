package dev.blnt.beans.sidecar;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import dev.blnt.beans.sidecar.model.Response;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.Timeout;

import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.OutputStream;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.Executors;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.TimeoutException;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

@Timeout(10)
class ProtocolTest {

    private final ObjectMapper mapper = new ObjectMapper();

    @Test
    void pings_keep_their_ids_across_multiple_lines() throws Exception {
        var responses = exchange("""
                {"jsonrpc":"2.0","id":1,"method":"ping"}
                {"jsonrpc":"2.0","id":"next","method":"ping"}
                """);
        assertEquals(2, responses.size());
        assertEquals(1, responses.get(0).get("id").intValue());
        assertEquals("next", responses.get(1).get("id").textValue());
        for (var response : responses) {
            assertEquals("2.0", response.get("jsonrpc").textValue());
            assertEquals("pong", response.get("result").textValue());
            assertFalse(response.has("error"));
        }
    }

    @Test
    void an_unknown_method_does_not_end_the_conversation() throws Exception {
        var responses = exchange("""
                {"jsonrpc":"2.0","id":1,"method":"unknown"}
                {"jsonrpc":"2.0","id":2,"method":"ping"}
                """);
        assertEquals(2, responses.size());
        assertEquals(1, responses.get(0).get("id").intValue());
        assertEquals(-32601, responses.get(0).get("error").get("code").intValue());
        assertFalse(responses.get(0).has("result"));
        assertEquals("pong", responses.get(1).get("result").textValue());
    }

    @Test
    void malformed_json_does_not_consume_the_next_line() throws Exception {
        var responses = exchange("""
                not json
                {"jsonrpc":"2.0","id":1,"method":"ping"} {}
                {"jsonrpc":"2.0","id":2,"method":"ping"}
                """);
        assertEquals(3, responses.size());
        for (var response : responses.subList(0, 2)) {
            assertEquals(-32700, response.get("error").get("code").intValue());
            assertTrue(response.get("id").isNull());
        }
        assertEquals(2, responses.get(2).get("id").intValue());
    }

    @Test
    void invalid_envelopes_and_ids_are_rejected() throws Exception {
        for (var message : List.of(
                "null", "[]", "{}",
                "{\"jsonrpc\":2.0,\"id\":1,\"method\":\"ping\"}",
                "{\"jsonrpc\":\"1.0\",\"id\":1,\"method\":\"ping\"}",
                "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":123}",
                "{\"jsonrpc\":\"2.0\",\"id\":true,\"method\":\"ping\"}",
                "{\"jsonrpc\":\"2.0\",\"id\":1.5,\"method\":\"ping\"}",
                "{\"jsonrpc\":\"2.0\",\"id\":9223372036854775808,\"method\":\"ping\"}",
                "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":\"pong\"}")) {
            var responses = exchange(message);
            assertEquals(1, responses.size(), message);
            assertEquals(-32600, responses.get(0).get("error").get("code").intValue(), message);
        }
    }

    @Test
    void notifications_are_silent_but_explicit_null_ids_receive_a_response() throws Exception {
        var responses = exchange("""

                {"jsonrpc":"2.0","method":"ping"}
                {"jsonrpc":"2.0","method":"unknown"}
                {"jsonrpc":"2.0","id":null,"method":"ping"}
                """);
        assertEquals(1, responses.size());
        assertTrue(responses.get(0).get("id").isNull());
        assertEquals("pong", responses.get(0).get("result").textValue());
    }

    @Test
    void null_success_is_not_serialized_as_an_error_or_an_absent_result() throws Exception {
        var response = mapper.readTree(mapper.writeValueAsString(Response.success(null, null)));
        assertTrue(response.has("result"));
        assertTrue(response.get("result").isNull());
        assertFalse(response.has("error"));
    }

    @Test
    void output_failures_propagate() {
        var input = new ByteArrayInputStream(
                "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}".getBytes(StandardCharsets.UTF_8));
        var output = new OutputStream() {
            @Override
            public void write(int value) throws IOException {
                throw new IOException("broken output");
            }
        };
        var error = assertThrows(IOException.class,
                () -> new Handler(new Rpc(input, output)).run());
        assertEquals("broken output", error.getMessage());
    }

    @Test
    void modules_register_typed_methods_and_can_return_null() throws Exception {
        var responses = exchange("""
                {"jsonrpc":"2.0","id":1,"method":"echo","params":{"text":"hello"}}
                {"jsonrpc":"2.0","id":2,"method":"nothing"}
                """, handler -> {
            handler.register("echo", Echo.class, params -> CompletableFuture.completedFuture(params.text()));
            handler.register("nothing", Void.class, params -> CompletableFuture.completedFuture(null));
        });
        assertEquals("hello", responses.get(0).get("result").textValue());
        assertTrue(responses.get(1).has("result"));
        assertTrue(responses.get(1).get("result").isNull());
        assertFalse(responses.get(1).has("error"));
    }

    @Test
    void invalid_params_and_method_failures_do_not_stop_dispatch() throws Exception {
        var responses = exchange("""
                {"jsonrpc":"2.0","id":1,"method":"echo","params":"not a record"}
                {"jsonrpc":"2.0","id":2,"method":"echo"}
                {"jsonrpc":"2.0","id":3,"method":"throw"}
                {"jsonrpc":"2.0","id":4,"method":"fail"}
                {"jsonrpc":"2.0","id":5,"method":"ping"}
                """, handler -> {
            handler.register("echo", Echo.class, params -> CompletableFuture.completedFuture(params.text()));
            handler.register("throw", Void.class, params -> {
                throw new IllegalStateException("method failed");
            });
            handler.register("fail", Void.class,
                    params -> CompletableFuture.failedFuture(new IllegalStateException("future failed")));
        });
        assertEquals(5, responses.size());
        for (var response : responses.subList(0, 2)) {
            assertEquals(-32602, response.get("error").get("code").intValue());
        }
        for (var response : responses.subList(2, 4)) {
            assertEquals(-32603, response.get("error").get("code").intValue());
        }
        assertEquals("pong", responses.get(4).get("result").textValue());
    }

    @Test
    void duplicate_registrations_cannot_replace_existing_methods() throws Exception {
        var rpc = new Rpc(new ByteArrayInputStream(new byte[0]), new ByteArrayOutputStream());
        var handler = new Handler(rpc);
        assertThrows(IllegalArgumentException.class,
                () -> handler.register("ping", Void.class, params -> CompletableFuture.completedFuture("wrong")));
        handler.register("echo", Echo.class, params -> CompletableFuture.completedFuture(params.text()));
        assertThrows(IllegalArgumentException.class,
                () -> handler.register("echo", Void.class, params -> CompletableFuture.completedFuture(null)));
    }

    @Test
    void pending_requests_and_notifications_do_not_block_reads_and_are_drained_on_eof() throws Exception {
        for (boolean notification : List.of(false, true)) {
            var messages = """
                    {"jsonrpc":"2.0",%s"method":"delayed"}
                    {"jsonrpc":"2.0","id":2,"method":"ping"}
                    """.formatted(notification ? "" : "\"id\":1,");
            var eof = new CountDownLatch(1);
            var input = new ByteArrayInputStream(messages.getBytes(StandardCharsets.UTF_8)) {
                @Override
                public synchronized int read(byte[] bytes, int offset, int length) {
                    int count = super.read(bytes, offset, length);
                    if (count == -1) {
                        eof.countDown();
                    }
                    return count;
                }
            };
            var output = new ByteArrayOutputStream();
            var handler = new Handler(new Rpc(input, output));
            var delayed = new CompletableFuture<String>();
            handler.register("delayed", Void.class, params -> delayed);
            var worker = Executors.newSingleThreadExecutor();
            try {
                var running = worker.submit(() -> {
                    handler.run();
                    return null;
                });
                assertTrue(eof.await(5, TimeUnit.SECONDS), "reader must reach EOF without waiting for the method");
                assertThrows(TimeoutException.class, () -> running.get(100, TimeUnit.MILLISECONDS),
                        "EOF must drain the pending method");
                var beforeDrain = responses(output);
                assertEquals(1, beforeDrain.size());
                assertEquals(2, beforeDrain.get(0).get("id").intValue());
                assertEquals("pong", beforeDrain.get(0).get("result").textValue());

                delayed.complete("done");
                running.get(5, TimeUnit.SECONDS);
                var afterDrain = responses(output);
                assertEquals(notification ? 1 : 2, afterDrain.size());
                if (!notification) {
                    assertEquals(1, afterDrain.get(1).get("id").intValue());
                    assertEquals("done", afterDrain.get(1).get("result").textValue());
                }
            } finally {
                delayed.complete("done");
                worker.shutdownNow();
            }
        }
    }

    private List<JsonNode> exchange(String messages) throws Exception {
        return exchange(messages, handler -> {});
    }

    private List<JsonNode> exchange(String messages, Module module) throws Exception {
        var input = new ByteArrayInputStream(messages.getBytes(StandardCharsets.UTF_8));
        var output = new ByteArrayOutputStream();
        var handler = new Handler(new Rpc(input, output));
        module.register(handler);
        handler.run();
        return responses(output);
    }

    private List<JsonNode> responses(ByteArrayOutputStream output) throws IOException {
        List<JsonNode> responses = new ArrayList<>();
        for (var line : output.toString(StandardCharsets.UTF_8).lines().toList()) {
            responses.add(mapper.readTree(line));
        }
        return responses;
    }

    record Echo(String text) {}
}
