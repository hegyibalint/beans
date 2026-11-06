package dev.blnt.beans.sidecar;

import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import dev.blnt.beans.sidecar.model.Request;
import dev.blnt.beans.sidecar.model.Response;

import java.io.IOException;
import java.io.UncheckedIOException;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.Map;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.CompletionException;
import java.util.concurrent.CompletionStage;
import java.util.function.Function;

public final class Handler {

    private final Rpc rpc;
    private final ObjectMapper mapper = new ObjectMapper();
    private final Map<String, Function<Request, CompletionStage<Response>>> methods = new HashMap<>();

    public Handler(Rpc rpc) {
        this.rpc = rpc;
        register("ping", Void.class, params -> CompletableFuture.completedFuture("pong"));
    }

    public <P, R> void register(
        String name,
        Class<P> type,
        Function<P, ? extends CompletionStage<R>> method
    ) {
        var previous = methods.putIfAbsent(name, request -> invoke(request, type, method));
        if (previous != null) {
            throw new IllegalArgumentException("Method already registered: " + name);
        }
    }

    public void run() throws IOException {
        var pending = new ArrayList<CompletableFuture<Void>>();
        while (true) {
            // First, send out all responses finished since
            for (var iterator = pending.iterator(); iterator.hasNext();) {
                var reply = iterator.next();
                if (reply.isDone()) {
                    await(reply);
                    iterator.remove();
                }
            }

            // Read and deserialize an incoming JSON line
            JsonNode message;
            try {
                message = rpc.receive();
            } catch (JsonProcessingException error) {
                rpc.send(Response.failure(null, -32700, "Parse error"));
                continue;
            }
            if (message == null) {
                break;
            }

            // Bind the JSON line into a proper Request
            Request request;
            try {
                request = bindRequest(message);
            } catch (IllegalArgumentException error) {
                rpc.send(Response.failure(null, -32600, error.getMessage()));
                continue;
            }

            // Match who is going to handle the message, kick off the computation, or send back an error
            var method = methods.get(request.method());
            CompletionStage<Response> response = method == null
                    ? CompletableFuture.completedFuture(
                            Response.failure(request.id(), -32601, "Unknown method: " + request.method()))
                    : method.apply(request);
            var reply = response.thenAccept(result -> send(request, result));
            pending.add(reply.toCompletableFuture());
        }
        for (var reply : pending) {
            await(reply);
        }
    }

    private <P, R> CompletionStage<Response> invoke(
            Request request, Class<P> type, Function<P, ? extends CompletionStage<R>> method) {
        P params;
        try {
            if (type != Void.class && (request.params() == null || request.params().isNull())) {
                throw new IllegalArgumentException("Missing params");
            }
            params = mapper.treeToValue(request.params(), type);
        } catch (JsonProcessingException | IllegalArgumentException error) {
            return CompletableFuture.completedFuture(
                    Response.failure(request.id(), -32602, "Invalid params: " + error.getMessage()));
        }
        try {
            return method.apply(params).handle((result, error) -> error == null
                    ? Response.success(request.id(), result)
                    : Response.failure(request.id(), -32603, "Method failed: " + error.getMessage()));
        } catch (RuntimeException error) {
            return CompletableFuture.completedFuture(
                    Response.failure(request.id(), -32603, "Method failed: " + error.getMessage()));
        }
    }

    private Request bindRequest(JsonNode message) {
        if (!message.isObject()
                || !message.path("jsonrpc").isTextual()
                || !message.path("jsonrpc").textValue().equals("2.0")
                || !message.path("method").isTextual()) {
            throw new IllegalArgumentException("Invalid request");
        }
        var id = message.get("id");
        if (id != null && !id.isNull() && !id.isTextual()
                && !(id.isIntegralNumber() && id.canConvertToLong())) {
            throw new IllegalArgumentException("Invalid request ID");
        }
        return new Request(id, message.get("method").textValue(), message.get("params"));
    }

    private void send(Request request, Response response) {
        if (request.id() != null) {
            try {
                rpc.send(response);
            } catch (IOException error) {
                throw new UncheckedIOException(error);
            }
        }
    }

    private void await(CompletableFuture<Void> reply) throws IOException {
        try {
            reply.join();
        } catch (CompletionException error) {
            if (error.getCause() instanceof UncheckedIOException failure) {
                throw failure.getCause();
            }
            throw error;
        }
    }
}
