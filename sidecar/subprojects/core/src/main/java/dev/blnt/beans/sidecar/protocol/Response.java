package dev.blnt.beans.sidecar.protocol;

import com.fasterxml.jackson.annotation.JsonInclude;

/** One reply: {@code result} or {@code error}, never both. */
@JsonInclude(JsonInclude.Include.NON_NULL)
public record Response(Integer id, Object result, RpcError error) {

    public static Response ok(Integer id, Object result) {
        return new Response(id, result, null);
    }

    public static Response failure(Integer id, String message) {
        return new Response(id, null, new RpcError(message));
    }
}
