package dev.blnt.beans.sidecar.model;

import com.fasterxml.jackson.databind.JsonNode;

public sealed interface Response {

    static Response success(JsonNode id, Object result) {
        return new Success("2.0", id, result);
    }

    static Response failure(JsonNode id, int code, String message) {
        return new Failure("2.0", id, new RpcError(code, message));
    }

    record Success(String jsonrpc, JsonNode id, Object result) implements Response {}

    record Failure(String jsonrpc, JsonNode id, RpcError error) implements Response {}

    record RpcError(int code, String message) {}
}
