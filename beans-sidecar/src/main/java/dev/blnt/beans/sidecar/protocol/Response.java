package dev.blnt.beans.sidecar.protocol;

import com.fasterxml.jackson.annotation.JsonInclude;

/**
 * One outbound reply: {@code {id, result}} or {@code {id, error}} —
 * never both; absent halves are omitted from the wire.
 */
@JsonInclude(JsonInclude.Include.NON_NULL)
public record Response(Integer id, Object result, RpcError error) {

    public static Response ok(Integer id, Object result) {
        return new Response(id, result, null);
    }

    public static Response failure(Integer id, String message) {
        return new Response(id, null, new RpcError(message));
    }
}
